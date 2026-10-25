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
use tauri_plugin_updater::UpdaterExt;
use vaultsweep_core::mcp::{Scanner, Server};
use vaultsweep_core::model::Report;
use vaultsweep_core::net::{Fetcher, HttpFetcher};
use vaultsweep_core::op::{self, Connection, OpError, RealOp};
use vaultsweep_core::rules;
use vaultsweep_core::scan::{self, Progress, ScanResult};
use vaultsweep_core::schedule;
use vaultsweep_core::store::{Pending, Settings, State, Store};
use zeroize::Zeroizing;

/// パスワード変更中だけメモリに持つもの（画面には渡さない）
#[derive(Default)]
struct Session {
    old: Option<Zeroizing<String>>,
    new: Option<Zeroizing<String>>,
    /// 1Password の中を書き換えた（かもしれない）。確定か元に戻すまで、元のパスワードを手放さない。
    dirty: bool,
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
        let mut after = after;
        let mut schedule_error = None;
        if sched_changed {
            if let Err(e) = apply_schedule(&after) {
                schedule_error = Some(e);
                // 予定を作れなかったら「オン」のままにしない
                if after.auto_scan {
                    after.auto_scan = false;
                    after = st.save_settings(after).map_err(other)?;
                }
            }
        }
        Ok(Saved { next_run: schedule::next_run_now(&after), settings: after, schedule_error })
    })
    .await
}

fn apply_schedule(s: &Settings) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return apply_schedule_mac(s);
    }
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
        // schtasks のエラー文は文字コードが違って化けるので、終了コードだけ返す
        Err(format!("schtasks exit {}", out.status.code().unwrap_or(-1)))
    }
}

/// Mac：~/Library/LaunchAgents に予定を置いて launchd に読ませる（オフなら外して消す）
fn apply_schedule_mac(s: &Settings) -> Result<(), String> {
    use std::process::Command;
    let home = dirs::home_dir().ok_or("no home")?;
    let plist = home.join("Library").join("LaunchAgents").join(format!("{}.plist", schedule::LAUNCHD_LABEL));
    let uid = Command::new("/usr/bin/id").arg("-u").output().map_err(|e| e.to_string())?;
    let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
    if uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) {
        return Err("uid".into());
    }
    let domain = format!("gui/{uid}");
    // 前の予定は外す（入っていなければ失敗するが、それでよい）
    let _ = Command::new("/bin/launchctl").args(["bootout", &format!("{domain}/{}", schedule::LAUNCHD_LABEL)]).output();
    if !s.auto_scan {
        let _ = std::fs::remove_file(&plist);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let body = schedule::launchd_plist(&exe.to_string_lossy(), s).ok_or("time")?;
    std::fs::create_dir_all(plist.parent().ok_or("dir")?).map_err(|e| e.to_string())?;
    std::fs::write(&plist, body).map_err(|e| e.to_string())?;
    let out = Command::new("/bin/launchctl").args(["bootstrap", &domain]).arg(&plist).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("launchctl exit {}", out.status.code().unwrap_or(-1)))
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
    // 作り直しのときは、最初に読んだ「元のパスワード」をそのまま使う（作った値を元として読み直さない）
    if app.state::<AppState>().sessions.lock().unwrap().get(&id).is_some_and(|s| s.dirty) {
        return Ok(());
    }
    let i2 = id.clone();
    let old = blocking(move || op::read_password(&RealOp::default(), &i2)).await?;
    app.state::<AppState>().sessions.lock().unwrap().insert(id, Session { old, new: None, dirty: false });
    Ok(())
}

/// サイトの決まりを画面に見せる形（値ではなく決まりだけ）
#[derive(Serialize)]
struct RuleInfo {
    min: Option<u32>,
    max: Option<u32>,
    max_consecutive: Option<u32>,
    /// 必ず入れる種類："lower" / "upper" / "digit" / "symbol"
    required: Vec<String>,
    /// 使える記号が限られているときの記号の一覧。None なら制限なし
    symbols: Option<String>,
    letters: bool,
    digits: bool,
}

#[derive(Serialize)]
struct ChangePlan {
    domain: Option<String>,
    rule: Option<RuleInfo>,
    plan: rules::Plan,
    /// 本人が決めた決まり（あれば）
    site: Option<rules::SiteRule>,
}

fn rule_info(r: &rules::Rule) -> RuleInfo {
    let mut req: Vec<String> = vec![];
    for q in &r.required {
        for (on, k) in [(q.lower, "lower"), (q.upper, "upper"), (q.digit, "digit"), (!q.symbols.is_empty(), "symbol")] {
            if on && !req.iter().any(|x| x == k) {
                req.push(k.into());
            }
        }
    }
    let a = r.allowed.as_ref();
    RuleInfo {
        min: r.min,
        max: r.max,
        max_consecutive: r.max_consecutive,
        required: req,
        symbols: a.filter(|a| !a.any).map(|a| a.symbols.iter().filter(|c| !c.is_whitespace()).collect()),
        letters: a.map(|a| a.any || a.lower || a.upper).unwrap_or(true),
        digits: a.map(|a| a.any || a.digit).unwrap_or(true),
    }
}

/// 項目のサイトと、使う作り方を決める
fn plan_for(id: &str) -> (Option<String>, Option<rules::Rule>, Option<rules::SiteRule>, rules::Plan, Settings) {
    let st = store();
    let s = st.settings();
    let item = st.report().and_then(|r| r.items.into_iter().find(|i| i.id == id));
    let domain = item.as_ref().and_then(|i| i.domain.clone()).map(|d| d.to_lowercase());
    let rule = item.as_ref().and_then(|i| i.pw_rule.as_deref()).map(rules::parse);
    let site = domain.as_ref().and_then(|d| s.site_rules.get(d).copied());
    let plan = rules::plan(s.pw_length, s.pw_symbols, rule.as_ref(), site);
    (domain, rule, site, plan, s)
}

/// 変更画面で、このサイトの決まりと作り方を見せる
#[tauri::command]
async fn change_plan(id: String) -> R<ChangePlan> {
    op::check_id(&id)?;
    blocking(move || {
        let (domain, rule, site, plan, _) = plan_for(&id);
        Ok(ChangePlan { domain, rule: rule.as_ref().map(rule_info), plan, site })
    })
    .await
}

/// 新しいパスワードを 1Password の中で作って保存する。
/// site を渡したら、このサイトの決まりとして覚えてから作る（作り直しにも使う）。
/// 作ったあと決まりに合うかを確かめ、合わなければ5回まで作り直す。
/// 元のパスワード（old）は最初に預かったものを持ち続ける。
#[tauri::command]
async fn change_generate(app: AppHandle, id: String, site: Option<rules::SiteRule>) -> R<()> {
    op::check_id(&id)?;
    if !app.state::<AppState>().sessions.lock().unwrap().contains_key(&id) {
        return Err(OpError::Other("no_session".into()));
    }
    let i2 = id.clone();
    let (plan, check) = blocking(move || {
        let given = site.map(|sr| sr.sanitized());
        if let Some(sr) = given {
            let st = store();
            let (domain, ..) = plan_for(&i2);
            if let Some(d) = domain {
                let mut s = st.settings();
                s.site_rules.insert(d, sr);
                st.save_settings(s).map_err(other)?;
            }
        }
        let (_, rule, saved, plan, s) = plan_for(&i2);
        // 渡された決まりを優先（ドメインが無くて覚えられない項目でも、今回はそれで作る）
        let site = given.or(saved);
        let plan = match site {
            Some(_) => rules::plan(s.pw_length, s.pw_symbols, None, site),
            None => plan,
        };
        let check = match site {
            Some(sr) => Some(rules::site_rule_as_rule(sr)),
            None => rule.filter(|_| plan.source == "rule"),
        };
        Ok((plan, check))
    })
    .await?;
    if plan.manual {
        return Err(OpError::Other("rule_manual".into()));
    }
    match app.state::<AppState>().sessions.lock().unwrap().get_mut(&id) {
        Some(sess) => sess.dirty = true,
        None => return Err(OpError::Other("no_session".into())),
    }
    let i2 = id.clone();
    let (new, ok) = blocking(move || {
        let o = RealOp::default();
        let recipe = plan.recipe();
        let mut last = None;
        for _ in 0..5 {
            op::generate_password(&o, &i2, &recipe)?;
            let pw = op::read_password(&o, &i2)?;
            let fits = match (&check, pw.as_deref()) {
                (Some(r), Some(p)) => rules::satisfies(p, r),
                _ => true,
            };
            last = pw;
            if fits {
                return Ok((last, true));
            }
        }
        Ok((last, false))
    })
    .await?;
    if let Some(sess) = app.state::<AppState>().sessions.lock().unwrap().get_mut(&id) {
        sess.new = new;
    }
    if !ok {
        return Err(OpError::Other("rule_unmet".into()));
    }
    Ok(())
}

#[derive(Serialize)]
struct UpdateInfo {
    version: String,
    current: String,
}

/// 新しい版があるか（GitHub のリリースの一覧表 latest.json を見る）。オフラインの設定なら見ない
#[tauri::command]
async fn update_check(app: AppHandle) -> R<Option<UpdateInfo>> {
    if store().settings().offline {
        return Err(OpError::Other("offline".into()));
    }
    let up = app
        .updater()
        .map_err(|e| OpError::Other(format!("update: {e}")))?
        .check()
        .await
        .map_err(|e| OpError::Other(format!("update: {e}")))?;
    Ok(up.map(|u| UpdateInfo { version: u.version.clone(), current: u.current_version.clone() }))
}

/// 新しい版を入れて開き直す。パスワード変更の途中（元のパスワードを預かっている間）は断る。
/// 進み具合は "update-progress"（0〜100、分からなければ null）で知らせる。
#[tauri::command]
async fn update_install(app: AppHandle) -> R<()> {
    if app.state::<AppState>().sessions.lock().unwrap().values().any(|s| s.dirty) {
        return Err(OpError::Other("change_unfinished".into()));
    }
    let u = app
        .updater()
        .map_err(|e| OpError::Other(format!("update: {e}")))?
        .check()
        .await
        .map_err(|e| OpError::Other(format!("update: {e}")))?
        .ok_or(OpError::Other("no_update".into()))?;
    let a2 = app.clone();
    let mut got: u64 = 0;
    u.download_and_install(
        move |chunk, total| {
            got += chunk as u64;
            let pct = total.filter(|t| *t > 0).map(|t| (got * 100 / t).min(100));
            let _ = a2.emit("update-progress", pct);
        },
        || {},
    )
    .await
    .map_err(|e| OpError::Other(format!("update: {e}")))?;
    app.restart();
}

/// このサイトの決まりを忘れる（公開データか設定どおりに戻す）
#[tauri::command]
async fn site_rule_clear(domain: String) -> R<()> {
    blocking(move || {
        let st = store();
        let mut s = st.settings();
        s.site_rules.remove(&domain.to_lowercase());
        st.save_settings(s).map_err(other)?;
        Ok(())
    })
    .await
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
    // 記録できてから元のパスワードを手放す（記録に失敗しても「戻す」ができるように）
    let i2 = id.clone();
    let r = blocking(move || store().mark_changed(&i2).map_err(other)).await?;
    app.state::<AppState>().sessions.lock().unwrap().remove(&id);
    clip::clear_if_ours();
    Ok(r)
}

/// 元のパスワードに戻す（1Password の中だけ）
#[tauri::command]
async fn change_restore(app: AppHandle, id: String, no_passkey: bool) -> R<()> {
    // 1Password の決まり：JSON で書き戻すとパスキーが消える。
    // パスキー対応サイト、または本人が「パスキーは入れていない」と答えていないときは、自動で戻さない。
    let passkey = store()
        .report()
        .and_then(|r| r.items.into_iter().find(|i| i.id == id))
        .map(|i| i.passkey_site)
        .unwrap_or(false);
    if passkey || !no_passkey {
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
    // 1Password が元どおりになったので、覚えていた値も手放す
    app.state::<AppState>().sessions.lock().unwrap().remove(&id);
    clip::clear_if_ours();
    Ok(())
}

#[tauri::command]
fn change_end(app: AppHandle, id: String) {
    {
        let st = app.state::<AppState>();
        let mut map = st.sessions.lock().unwrap();
        // 作ったまま確定していないものは、元のパスワードを覚えたままにする（あとで戻せるように）。アプリを閉じれば消える。
        if !map.get(&id).is_some_and(|s| s.dirty) {
            map.remove(&id);
        }
    }
    clip::clear_if_ours();
}

/// 作ったまま確定していない項目（名前は画面側で引く）
fn unfinished_ids(app: &AppHandle) -> Vec<String> {
    app.state::<AppState>().sessions.lock().unwrap().iter().filter(|(_, s)| s.dirty).map(|(k, _)| k.clone()).collect()
}

#[tauri::command]
fn change_unfinished(app: AppHandle) -> Vec<String> {
    unfinished_ids(&app)
}

/// 確定していない項目があっても、本人が選んだので閉じる
#[tauri::command]
fn quit_now(app: AppHandle) {
    app.state::<AppState>().sessions.lock().unwrap().clear();
    clip::clear_if_ours();
    app.exit(0);
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
        let found = s.state().pending.into_iter().find(|p| p.id == id);
        // 片付けが成功してから依頼を消す（失敗したら依頼は残る）
        if let (Some(p), true) = (&found, approve) {
            if p.kind == "archive" {
                op::archive(&RealOp::default(), &p.item_id)?;
                s.remove_item(&p.item_id).map_err(other)?;
                if s.state().cancel.contains_key(&p.item_id) {
                    s.cancel_set(&p.item_id, "archived", None).map_err(other)?;
                }
            }
        }
        let p = s.take_pending(&id).map_err(other)?;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            change_plan,
            site_rule_clear,
            update_check,
            update_install,
            copy_secret,
            copy_plain,
            change_confirm,
            change_unfinished,
            quit_now,
            change_restore,
            change_end,
            archive,
            write_receipt,
            open_url,
            pending_resolve,
            export_csv,
            test_key_ref
        ])
        .setup(move |app| {
            clip::init(app.handle());
            if headless {
                // Mac：裏での診断中は Dock にアイコンを出さない
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                headless_scan(app.handle().clone());
            } else {
                // アプリの場所が変わっても自動診断が古い場所を指さないよう、起動のたびに登録し直す
                std::thread::spawn(|| {
                    let s = store().settings();
                    if s.auto_scan {
                        let _ = apply_schedule(&s);
                    }
                });
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                    .title("vaultsweep")
                    .inner_size(1320.0, 860.0)
                    .min_inner_size(960.0, 640.0)
                    .build()?;
            }
            Ok(())
        })
        .on_window_event(|window, ev| {
            // 新しいパスワードを作ったまま（サイトでの変更が確定していない）なら、すぐには閉じず画面で確かめる
            if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                let ids = unfinished_ids(window.app_handle());
                if !ids.is_empty() {
                    api.prevent_close();
                    let _ = window.emit("close-guard", ids);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("vaultsweep failed to start")
        .run(|app, ev| match ev {
            // Mac の ⌘Q など、ウィンドウの×を通らずにアプリを終える操作も、確定していない変更があれば止めて画面で確かめる
            // （「このまま閉じる」から呼ぶ app.exit(0) は code が入っているので止めない）
            tauri::RunEvent::ExitRequested { api, code: None, .. } => {
                let ids = unfinished_ids(app);
                if !ids.is_empty() {
                    if let Some(w) = app.get_webview_window("main") {
                        api.prevent_exit();
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = w.emit("close-guard", ids);
                    }
                }
            }
            // 閉じるときに、自分が入れたパスワードがまだクリップボードにあれば消す
            tauri::RunEvent::Exit => clip::clear_if_ours(),
            _ => {}
        });
}
