//! vaultsweep アプリ本体（Tauri）
//! 起動のしかた
//!   vaultsweep.exe          画面を開く
//!   vaultsweep.exe --scan   画面を出さずに診断して、通知だけ出す（自動診断で使う）
//!   vaultsweep.exe --mcp    MCP サーバーとして動く（vaultsweep-mcp.exe と同じ）

mod clip;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use vaultsweep_core::freee::{self, PayMatch};
use vaultsweep_core::mcp::{Scanner, Server};
use vaultsweep_core::model::Report;
use vaultsweep_core::net::{Fetcher, HttpFetcher};
use vaultsweep_core::op::{self, Connection, OpError, RealOp};
use vaultsweep_core::scan::{self, Progress, ScanResult};
use vaultsweep_core::schedule;
use vaultsweep_core::store::{Pending, Settings, State, Store};
use zeroize::Zeroizing;

/// パスワード変更中だけメモリに持つもの（画面には渡さない）
#[derive(Default)]
struct Session {
    old: Option<Zeroizing<String>>,
    new: Option<Zeroizing<String>>,
}

#[derive(Default)]
struct AppState {
    sessions: Mutex<HashMap<String, Session>>,
    scanning: Mutex<bool>,
}

type R<T> = Result<T, OpError>;

fn other<E: ToString>(e: E) -> OpError {
    OpError::Other(e.to_string())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> R<T> + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(other)?
}

fn store() -> Store {
    Store::open_default()
}

fn ui_lang(s: &Settings) -> &'static str {
    match s.lang.as_deref() {
        Some("ja") => "ja",
        Some("en") => "en",
        _ => {
            if sys_locale::get_locale().map(|l| l.to_lowercase().starts_with("ja")).unwrap_or(false) {
                "ja"
            } else {
                "en"
            }
        }
    }
}

fn mcp_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) { "vaultsweep-mcp.exe" } else { "vaultsweep-mcp" };
    Some(exe.parent()?.join(name).to_string_lossy().to_string())
}

#[derive(Serialize)]
struct Boot {
    settings: Settings,
    state: State,
    report: Option<Report>,
    data_dir: String,
    mcp_path: Option<String>,
    next_run: Option<String>,
    os: &'static str,
    version: &'static str,
    system_lang: &'static str,
}

#[tauri::command]
fn bootstrap() -> Boot {
    let st = store();
    let settings = st.settings();
    Boot {
        next_run: schedule::next_run_now(&settings),
        system_lang: ui_lang(&Settings { lang: None, ..Settings::default() }),
        settings,
        state: st.state(),
        report: st.report(),
        data_dir: st.dir.to_string_lossy().to_string(),
        mcp_path: mcp_path(),
        os: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
async fn check_connection() -> R<Connection> {
    blocking(|| op::check_connection(&RealOp::default())).await
}

#[tauri::command]
async fn run_scan(app: AppHandle) -> R<ScanResult> {
    {
        let s = app.state::<AppState>();
        let mut busy = s.scanning.lock().unwrap();
        if *busy {
            return Err(OpError::Other("busy".into()));
        }
        *busy = true;
    }
    let a2 = app.clone();
    let res = blocking(move || {
        let f = HttpFetcher::new().ok();
        scan::run(&RealOp::default(), f.as_ref().map(|x| x as &dyn Fetcher), &store(), &|p: Progress| {
            let _ = a2.emit("scan-progress", p);
        })
    })
    .await;
    *app.state::<AppState>().scanning.lock().unwrap() = false;
    res
}

#[derive(Serialize)]
struct Saved {
    settings: Settings,
    next_run: Option<String>,
    schedule_error: Option<String>,
}

#[tauri::command]
async fn save_settings(settings: Settings) -> R<Saved> {
    blocking(move || {
        let st = store();
        let before = st.settings();
        let after = st.save_settings(settings).map_err(other)?;
        let sched_changed = before.auto_scan != after.auto_scan
            || before.schedule != after.schedule
            || before.schedule_time != after.schedule_time;
        let schedule_error = if sched_changed { apply_schedule(&after).err() } else { None };
        Ok(Saved { next_run: schedule::next_run_now(&after), settings: after, schedule_error })
    })
    .await
}

fn apply_schedule(s: &Settings) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("unsupported_os".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let args = if s.auto_scan {
        schedule::create_args(&exe.to_string_lossy(), s)
    } else {
        schedule::delete_args()
    };
    let mut cmd = std::process::Command::new("schtasks");
    cmd.args(&args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() || !s.auto_scan {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[tauri::command]
async fn set_in_use(id: String, on: bool) -> R<Option<Report>> {
    blocking(move || store().set_in_use(&id, on).map_err(other)).await
}

#[tauri::command]
async fn cancel_add(id: String) -> R<State> {
    blocking(move || {
        let s = store();
        s.cancel_add(&id).map_err(other)?;
        Ok(s.state())
    })
    .await
}

#[tauri::command]
async fn cancel_set(id: String, state: String, receipt: Option<String>) -> R<State> {
    blocking(move || {
        let s = store();
        s.cancel_set(&id, &state, receipt).map_err(other)?;
        Ok(s.state())
    })
    .await
}

/// 変更を始める：今のパスワードをメモリにだけ読み込む
#[tauri::command]
async fn change_begin(app: AppHandle, id: String) -> R<()> {
    op::check_id(&id)?;
    let i2 = id.clone();
    let old = blocking(move || op::read_password(&RealOp::default(), &i2)).await?;
    app.state::<AppState>().sessions.lock().unwrap().insert(id, Session { old, new: None });
    Ok(())
}

/// 新しいパスワードを 1Password の中で作って保存する
#[tauri::command]
async fn change_generate(app: AppHandle, id: String) -> R<()> {
    op::check_id(&id)?;
    if !app.state::<AppState>().sessions.lock().unwrap().contains_key(&id) {
        return Err(OpError::Other("no_session".into()));
    }
    let s = store().settings();
    let i2 = id.clone();
    let new = blocking(move || {
        let o = RealOp::default();
        op::generate_password(&o, &i2, &op::recipe(s.pw_length, s.pw_symbols))?;
        op::read_password(&o, &i2)
    })
    .await?;
    if let Some(sess) = app.state::<AppState>().sessions.lock().unwrap().get_mut(&id) {
        sess.new = new;
    }
    Ok(())
}

#[derive(Serialize)]
struct Copied {
    seconds: u64,
}

/// 今（old）か新しい（new）パスワードをコピーする。値は画面に返さない。
#[tauri::command]
fn copy_secret(app: AppHandle, id: String, which: String) -> R<Copied> {
    let v = {
        let st = app.state::<AppState>();
        let map = st.sessions.lock().unwrap();
        let sess = map.get(&id).ok_or(OpError::Other("no_session".into()))?;
        let v = if which == "new" { &sess.new } else { &sess.old };
        Zeroizing::new(v.as_ref().ok_or(OpError::Other("no_value".into()))?.to_string())
    };
    clip::copy_with_timeout(v).map_err(other)?;
    Ok(Copied { seconds: clip::CLEAR_SECS })
}

#[tauri::command]
fn copy_plain(text: String) -> R<()> {
    if text.len() > 4000 {
        return Err(OpError::Other("too_long".into()));
    }
    clip::copy_plain(&text).map_err(other)
}

/// サイトで保存できた：札を外して記録する
#[tauri::command]
async fn change_confirm(app: AppHandle, id: String) -> R<Option<Report>> {
    app.state::<AppState>().sessions.lock().unwrap().remove(&id);
    blocking(move || store().mark_changed(&id).map_err(other)).await
}

/// 元のパスワードに戻す（1Password の中だけ）
#[tauri::command]
async fn change_restore(app: AppHandle, id: String) -> R<()> {
    let passkey = store()
        .report()
        .and_then(|r| r.items.into_iter().find(|i| i.id == id))
        .map(|i| i.passkey_site)
        .unwrap_or(false);
    if passkey {
        return Err(OpError::PasskeyRisk);
    }
    let old = {
        let st = app.state::<AppState>();
        let map = st.sessions.lock().unwrap();
        map.get(&id).and_then(|s| s.old.as_ref().map(|v| Zeroizing::new(v.to_string())))
    };
    let Some(old) = old else { return Err(OpError::Other("no_value".into())) };
    let i2 = id.clone();
    blocking(move || op::restore_password(&RealOp::default(), &i2, &old)).await?;
    if let Some(s) = app.state::<AppState>().sessions.lock().unwrap().get_mut(&id) {
        s.new = None;
    }
    Ok(())
}

#[tauri::command]
fn change_end(app: AppHandle, id: String) {
    app.state::<AppState>().sessions.lock().unwrap().remove(&id);
}

/// 保管庫（アーカイブ）へ移す。完全には消さない。
#[tauri::command]
async fn archive(id: String) -> R<State> {
    blocking(move || {
        op::archive(&RealOp::default(), &id)?;
        let s = store();
        s.remove_item(&id).map_err(other)?;
        if s.state().cancel.contains_key(&id) {
            s.cancel_set(&id, "archived", None).map_err(other)?;
        }
        Ok(s.state())
    })
    .await
}

/// 退会の記録を 1Password の項目に残す（秘密ではない値だけ）
#[tauri::command]
async fn write_receipt(id: String, date_label: String, date: String, receipt_label: String, receipt: String) -> R<State> {
    blocking(move || {
        let mut pairs = vec![(date_label.as_str(), date.as_str())];
        if !receipt.trim().is_empty() {
            pairs.push((receipt_label.as_str(), receipt.as_str()));
        }
        op::add_text_fields(&RealOp::default(), &id, &pairs)?;
        let s = store();
        s.cancel_set(&id, "left", (!receipt.trim().is_empty()).then(|| receipt.clone())).map_err(other)?;
        Ok(s.state())
    })
    .await
}

/// URL を いつものブラウザ／メールアプリで開く（https と mailto だけ）
#[tauri::command]
fn open_url(app: AppHandle, url: String) -> R<()> {
    let ok = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:");
    if !ok || url.chars().any(|c| c.is_control()) || url.len() > 8000 {
        return Err(OpError::Other("bad_url".into()));
    }
    app.opener().open_url(url, None::<&str>).map_err(other)
}

#[derive(Serialize)]
struct Resolved {
    pending: Option<Pending>,
    state: State,
}

/// AI からの依頼を承認／却下する
#[tauri::command]
async fn pending_resolve(id: String, approve: bool) -> R<Resolved> {
    blocking(move || {
        let s = store();
        let p = s.take_pending(&id).map_err(other)?;
        if let (Some(p), true) = (&p, approve) {
            if p.kind == "archive" {
                op::archive(&RealOp::default(), &p.item_id)?;
                s.remove_item(&p.item_id).map_err(other)?;
                if s.state().cancel.contains_key(&p.item_id) {
                    s.cancel_set(&p.item_id, "archived", None).map_err(other)?;
                }
            }
        }
        Ok(Resolved { pending: p, state: s.state() })
    })
    .await
}

#[tauri::command]
async fn export_csv(app: AppHandle, header: Vec<String>, rows: Vec<Vec<String>>) -> R<String> {
    let path = blocking(move || {
        let dir = dirs::download_dir().unwrap_or_else(std::env::temp_dir);
        let name = format!("vaultsweep-{}.csv", chrono::Local::now().format("%Y%m%d-%H%M"));
        let p = dir.join(name);
        let esc = |s: &String| {
            // 表計算ソフトで式として動かないように
            let s = if s.starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.clone() };
            format!("\"{}\"", s.replace('"', "\"\""))
        };
        let mut out = String::from("\u{feff}");
        for r in std::iter::once(&header).chain(rows.iter()) {
            out.push_str(&r.iter().map(esc).collect::<Vec<_>>().join(","));
            out.push_str("\r\n");
        }
        std::fs::write(&p, out).map_err(other)?;
        Ok(p)
    })
    .await?;
    let _ = app.opener().reveal_item_in_dir(&path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn freee_match(text: String) -> R<Vec<PayMatch>> {
    let r = store().report().ok_or(OpError::Other("no_report".into()))?;
    freee::match_csv(&text, &r).map_err(other)
}

/// HIBP のキーの場所を確かめる（キーそのものは画面に返さない）
#[tauri::command]
async fn test_key_ref(reference: String) -> R<bool> {
    blocking(move || op::read_ref(&RealOp::default(), &reference).map(|k| !k.is_empty())).await
}

struct AppScanner;
impl Scanner for AppScanner {
    fn scan(&self, store: &Store) -> Result<Report, String> {
        let f = HttpFetcher::new().ok();
        scan::run(&RealOp::default(), f.as_ref().map(|x| x as &dyn Fetcher), store, &|_| {})
            .map(|r| r.report)
            .map_err(|e| e.to_string())
    }
}

/// 自動診断：画面を出さずに診断して、必要なら通知する
fn headless_scan(app: AppHandle) {
    std::thread::spawn(move || {
        let st = store();
        let s = st.settings();
        let ja = ui_lang(&s) == "ja";
        let f = HttpFetcher::new().ok();
        let res = scan::run(&RealOp::default(), f.as_ref().map(|x| x as &dyn Fetcher), &st, &|_| {});
        let msg: Option<(String, String)> = match (&res, s.notify.as_str()) {
            (_, "never") => None,
            (Ok(r), mode) if r.new_issues > 0 || mode == "always" => Some(if r.new_issues > 0 {
                if ja {
                    (format!("新しい問題が{}件見つかりました", r.new_issues), format!("安全度 {}。vaultsweep を開いて確かめてください。", r.report.score))
                } else {
                    (format!("{} new issues found", r.new_issues), format!("Safety score {}. Open vaultsweep to review.", r.report.score))
                }
            } else if ja {
                ("診断が終わりました".into(), format!("安全度 {}。新しい問題はありません。", r.report.score))
            } else {
                ("Scan finished".into(), format!("Safety score {}. No new issues.", r.report.score))
            }),
            (Ok(_), _) => None,
            (Err(OpError::NotSignedIn(_)), _) => Some(if ja {
                ("1Password がロック中で診断できませんでした".into(), "1Password を開いてロックを解除し、vaultsweep の「今すぐ診断」を押してください。".into())
            } else {
                ("Couldn't scan: 1Password is locked".into(), "Unlock 1Password, then press \"Scan now\" in vaultsweep.".into())
            }),
            (Err(e), _) => Some(if ja { ("自動診断に失敗しました".into(), e.to_string()) } else { ("Automatic scan failed".into(), e.to_string()) }),
        };
        if let Some((t, b)) = msg {
            let _ = app.notification().builder().title(t).body(b).show();
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        app.exit(0);
    });
}

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--mcp") {
        let st = store();
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let _ = Server::new(&st, &AppScanner).serve(stdin.lock(), stdout.lock());
        return;
    }
    let headless = args.iter().any(|a| a == "--scan");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            check_connection,
            run_scan,
            save_settings,
            set_in_use,
            cancel_add,
            cancel_set,
            change_begin,
            change_generate,
            copy_secret,
            copy_plain,
            change_confirm,
            change_restore,
            change_end,
            archive,
            write_receipt,
            open_url,
            pending_resolve,
            export_csv,
            freee_match,
            test_key_ref
        ])
        .setup(move |app| {
            clip::init(app.handle());
            if headless {
                headless_scan(app.handle().clone());
            } else {
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                    .title("vaultsweep")
                    .inner_size(1320.0, 860.0)
                    .min_inner_size(960.0, 640.0)
                    .build()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("vaultsweep failed to start");
}
