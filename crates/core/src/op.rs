//! 1Password CLI（op）を呼ぶ部分。
//! - 秘密の値は引数に入れない（引数は他のプロセスから見えることがあるため）。
//! - 読み取った出力は Zeroizing に入れ、使い終わったら消す。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum OpError {
    /// op が見つからない
    NotInstalled,
    /// サインインしていない・ロック中・承認を断った
    NotSignedIn(String),
    /// 項目IDの形がおかしい
    BadId,
    /// パスキーが消える恐れがあるので自動では戻さない
    PasskeyRisk,
    Parse(String),
    Other(String),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::NotInstalled => write!(f, "1Password CLI (op) が見つかりません"),
            OpError::NotSignedIn(s) => write!(f, "1Password にサインインできません: {s}"),
            OpError::BadId => write!(f, "項目IDの形が正しくありません"),
            OpError::PasskeyRisk => write!(f, "パスキーが消える恐れがあるため、自動では戻しません"),
            OpError::Parse(s) => write!(f, "op の出力を読めません: {s}"),
            OpError::Other(s) => write!(f, "op でエラー: {s}"),
        }
    }
}
impl std::error::Error for OpError {}

pub type OpResult<T> = Result<T, OpError>;

/// op を実行する口。試験ではにせものに差し替える。
pub trait OpRunner: Send + Sync {
    fn run(&self, args: &[&str], stdin: Option<&[u8]>) -> OpResult<Zeroizing<Vec<u8>>>;
}

pub struct RealOp {
    pub bin: PathBuf,
}

impl Default for RealOp {
    fn default() -> Self {
        RealOp { bin: find_op() }
    }
}

/// op の場所を探す。PATH にあればそれ、なければ Windows のよくある場所。
pub fn find_op() -> PathBuf {
    let exe = if cfg!(windows) { "op.exe" } else { "op" };
    if let Some(paths) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&paths) {
            let c = p.join(exe);
            if c.is_file() {
                return c;
            }
        }
    }
    if cfg!(windows) {
        let mut cands: Vec<PathBuf> = vec![];
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            let l = PathBuf::from(l);
            cands.push(l.join("Microsoft").join("WinGet").join("Links").join(exe));
            cands.push(l.join("1Password").join("cli").join(exe));
        }
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            cands.push(PathBuf::from(pf).join("1Password CLI").join(exe));
        }
        if let Some(c) = cands.into_iter().find(|c| c.is_file()) {
            return c;
        }
    }
    PathBuf::from(exe)
}

impl OpRunner for RealOp {
    fn run(&self, args: &[&str], stdin: Option<&[u8]>) -> OpResult<Zeroizing<Vec<u8>>> {
        let mut cmd = Command::new(&self.bin);
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OpError::NotInstalled
            } else {
                OpError::Other(e.to_string())
            }
        })?;
        // 標準入力は別の流れで書く（op が読みながら書く場合に、お互い待ちで固まらないように）
        let writer = match (stdin, child.stdin.take()) {
            (Some(data), Some(mut si)) => {
                let data = Zeroizing::new(data.to_vec());
                Some(std::thread::spawn(move || {
                    use std::io::Write;
                    let _ = si.write_all(&data);
                }))
            }
            _ => None,
        };
        let mut so = child
            .stdout
            .take()
            .ok_or_else(|| OpError::Other("stdout".into()))?;
        let mut se = child
            .stderr
            .take()
            .ok_or_else(|| OpError::Other("stderr".into()))?;
        let rd_out = std::thread::spawn(move || {
            use std::io::Read;
            let mut b = Zeroizing::new(Vec::new());
            let _ = so.read_to_end(&mut b);
            b
        });
        let rd_err = std::thread::spawn(move || {
            use std::io::Read;
            let mut b = Vec::new();
            let _ = se.read_to_end(&mut b);
            b
        });
        // 時間切れ（承認画面を放置したときなど）
        let deadline = std::time::Instant::now() + OP_TIMEOUT;
        let status = loop {
            match child.try_wait() {
                Ok(Some(st)) => break st,
                Ok(None) if std::time::Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(OpError::NotSignedIn("timeout".into()));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(e) => return Err(OpError::Other(e.to_string())),
            }
        };
        if let Some(w) = writer {
            let _ = w.join();
        }
        let stdout = rd_out.join().unwrap_or_default();
        let stderr = rd_err.join().unwrap_or_default();
        if status.success() {
            Ok(stdout)
        } else {
            Err(classify_err(&String::from_utf8_lossy(&stderr)))
        }
    }
}

/// op を待つ最長時間（Windows Hello の承認を含む）
pub const OP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

pub fn classify_err(stderr: &str) -> OpError {
    let s = stderr.trim();
    let l = s.to_lowercase();
    let short: String = s.chars().take(300).collect();
    if l.contains("not signed in")
        || l.contains("no accounts configured")
        || l.contains("sign in")
        || l.contains("authorization")
        || l.contains("dismissed")
        || l.contains("locked")
        || l.contains("session expired")
    {
        OpError::NotSignedIn(short)
    } else {
        OpError::Other(short)
    }
}

/// 項目IDは英数字だけ（引数の書き換えを防ぐ）
pub fn check_id(id: &str) -> OpResult<()> {
    if !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(OpError::BadId)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub version: String,
    pub vaults: usize,
    pub account: Option<String>,
}

pub fn version(op: &dyn OpRunner) -> OpResult<String> {
    let out = op.run(&["--version"], None)?;
    Ok(String::from_utf8_lossy(&out).trim().to_string())
}

/// 接続確認。保管庫の一覧を取ると、ロック中ならアプリの承認（Windows Hello など）が出る。
pub fn check_connection(op: &dyn OpRunner) -> OpResult<Connection> {
    let version = version(op)?;
    let out = op.run(&["vault", "list", "--format", "json"], None)?;
    let v: Value = serde_json::from_slice(&out).map_err(|e| OpError::Parse(e.to_string()))?;
    let vaults = v.as_array().map(|a| a.len()).unwrap_or(0);
    let account = op
        .run(&["whoami", "--format", "json"], None)
        .ok()
        .and_then(|o| serde_json::from_slice::<Value>(&o).ok())
        .and_then(|w| w.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()));
    Ok(Connection {
        version,
        vaults,
        account,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
}

/// 1件分（秘密の値は Zeroizing）
pub struct LoginFull {
    pub id: String,
    pub title: String,
    pub vault: String,
    pub urls: Vec<String>,
    pub updated_at: Option<String>,
    pub password: Option<Zeroizing<String>>,
    pub username: Option<Zeroizing<String>>,
    /// 1Password が付けた強さ（あれば）
    pub strength: Option<String>,
    pub has_totp: bool,
}

/// ログイン項目の一覧（秘密は含まない）を、そのままの JSON で返す
pub fn list_logins_raw(op: &dyn OpRunner) -> OpResult<Zeroizing<Vec<u8>>> {
    op.run(
        &["item", "list", "--categories", "Login", "--format", "json"],
        None,
    )
}

pub fn parse_list(raw: &[u8]) -> OpResult<Vec<ListItem>> {
    serde_json::from_slice(raw).map_err(|e| OpError::Parse(e.to_string()))
}

/// 全項目の中身をまとめて取る。うまくいかなければ1件ずつ取る。
pub fn get_logins(
    op: &dyn OpRunner,
    list_raw: &[u8],
    progress: &mut dyn FnMut(usize, usize),
) -> OpResult<Vec<LoginFull>> {
    let list = parse_list(list_raw)?;
    let total = list.len();
    if total == 0 {
        return Ok(vec![]);
    }
    if let Ok(out) = op.run(
        &["item", "get", "-", "--format", "json", "--reveal"],
        Some(list_raw),
    ) {
        let mut res = vec![];
        let mut ok = true;
        for v in serde_json::Deserializer::from_slice(&out).into_iter::<Value>() {
            match v {
                Ok(mut v) => {
                    if let Some(a) = v.as_array_mut() {
                        for x in a.iter_mut() {
                            res.push(take_login(x));
                        }
                    } else {
                        res.push(take_login(&mut v));
                    }
                    wipe(&mut v);
                }
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && res.len() == total {
            progress(total, total);
            return Ok(res);
        }
    }
    let mut res = Vec::with_capacity(total);
    for (i, it) in list.iter().enumerate() {
        // 読めない項目が1件あっても、ほかは続ける（サインイン・ロックの問題だけは止める）
        match check_id(&it.id).and_then(|_| get_login(op, &it.id)) {
            Ok(l) => res.push(l),
            Err(e @ (OpError::NotSignedIn(_) | OpError::NotInstalled)) => return Err(e),
            Err(_) => {}
        }
        progress(i + 1, total);
    }
    Ok(res)
}

pub fn get_login(op: &dyn OpRunner, id: &str) -> OpResult<LoginFull> {
    check_id(id)?;
    let out = op.run(&["item", "get", id, "--format", "json", "--reveal"], None)?;
    let mut v: Value = serde_json::from_slice(&out).map_err(|e| OpError::Parse(e.to_string()))?;
    let l = take_login(&mut v);
    wipe(&mut v);
    Ok(l)
}

/// JSON から必要なものだけ取り出す（秘密の値は Zeroizing に移す）
fn take_login(v: &mut Value) -> LoginFull {
    let s = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    let id = s(v, "id").unwrap_or_default();
    let title = s(v, "title").unwrap_or_default();
    let vault = v
        .get("vault")
        .and_then(|x| x.get("name"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let mut urls: Vec<(bool, String)> = v
        .get("urls")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|u| {
                    let h = u.get("href").and_then(|h| h.as_str())?;
                    Some((
                        u.get("primary").and_then(|p| p.as_bool()).unwrap_or(false),
                        h.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    urls.sort_by_key(|(p, _)| !*p);
    let updated_at = s(v, "updated_at");
    let mut password = None;
    let mut username = None;
    let mut strength = None;
    let mut has_totp = false;
    if let Some(fields) = v.get_mut("fields").and_then(|f| f.as_array_mut()) {
        for f in fields.iter_mut() {
            let ty = f
                .get("type")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let purpose = f
                .get("purpose")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let fid = f
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if ty == "OTP" {
                let has_val = f
                    .get("value")
                    .and_then(|x| x.as_str())
                    .map(|x| !x.is_empty())
                    .unwrap_or(false)
                    || f.get("totp")
                        .and_then(|x| x.as_str())
                        .map(|x| !x.is_empty())
                        .unwrap_or(false);
                has_totp |= has_val;
            }
            if (purpose == "PASSWORD" || (purpose.is_empty() && fid == "password"))
                && password.is_none()
            {
                if let Some(Value::String(val)) = f.get_mut("value")
                    && !val.is_empty()
                {
                    password = Some(Zeroizing::new(std::mem::take(val)));
                }
                strength = f
                    .get("password_details")
                    .and_then(|d| d.get("strength"))
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string());
            } else if (purpose == "USERNAME" || (purpose.is_empty() && fid == "username"))
                && username.is_none()
                && let Some(Value::String(val)) = f.get_mut("value")
                && !val.is_empty()
            {
                username = Some(Zeroizing::new(std::mem::take(val)));
            }
        }
    }
    LoginFull {
        id,
        title,
        vault,
        urls: urls.into_iter().map(|(_, h)| h).collect(),
        updated_at,
        password,
        username,
        strength,
        has_totp,
    }
}

/// JSON の中の文字列をすべて上書きして消す
pub fn wipe(v: &mut Value) {
    match v {
        Value::String(s) => s.zeroize(),
        Value::Array(a) => a.iter_mut().for_each(wipe),
        Value::Object(o) => o.values_mut().for_each(wipe),
        _ => {}
    }
}

/// パスワードの作り方（例 "letters,digits,symbols,24"）
pub fn recipe(len: u32, symbols: bool) -> String {
    let len = len.clamp(12, 64);
    if symbols {
        format!("letters,digits,symbols,{len}")
    } else {
        format!("letters,digits,{len}")
    }
}

/// 新しいパスワードを 1Password の中で作って保存する。値は返さない（op 側で作る）。
pub fn generate_password(op: &dyn OpRunner, id: &str, recipe: &str) -> OpResult<()> {
    check_id(id)?;
    if !recipe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ',')
    {
        return Err(OpError::Other("recipe".into()));
    }
    let flag = format!("--generate-password={recipe}");
    op.run(&["item", "edit", id, &flag, "--format", "json"], None)?;
    Ok(())
}

/// パスワードを元に戻す。値は標準入力の JSON で渡す（引数には入れない）。
/// 1Password の JSON テンプレートで編集するとパスキーが消えることがあるため、
/// パスキー対応サイトの項目では使わない（呼ぶ側で passkey_site を確認）。
pub fn restore_password(op: &dyn OpRunner, id: &str, old: &str) -> OpResult<()> {
    check_id(id)?;
    let out = op.run(&["item", "get", id, "--format", "json", "--reveal"], None)?;
    let mut v: Value = serde_json::from_slice(&out).map_err(|e| OpError::Parse(e.to_string()))?;
    if looks_like_passkey(&v) {
        wipe(&mut v);
        return Err(OpError::PasskeyRisk);
    }
    let mut done = false;
    if let Some(fields) = v.get_mut("fields").and_then(|f| f.as_array_mut()) {
        for f in fields.iter_mut() {
            let purpose = f.get("purpose").and_then(|x| x.as_str()).unwrap_or("");
            let fid = f.get("id").and_then(|x| x.as_str()).unwrap_or("");
            if purpose == "PASSWORD" || (purpose.is_empty() && fid == "password") {
                if let Some(Value::String(val)) = f.get_mut("value") {
                    val.zeroize();
                }
                f["value"] = Value::String(old.to_string());
                if let Some(o) = f.as_object_mut() {
                    o.remove("password_details");
                }
                done = true;
                break;
            }
        }
    }
    if !done {
        wipe(&mut v);
        return Err(OpError::Other("password field not found".into()));
    }
    let body = Zeroizing::new(serde_json::to_vec(&v).map_err(|e| OpError::Parse(e.to_string()))?);
    wipe(&mut v);
    op.run(&["item", "edit", id, "--format", "json"], Some(&body))?;
    Ok(())
}

fn looks_like_passkey(v: &Value) -> bool {
    match v {
        Value::Object(o) => o.iter().any(|(k, x)| {
            k.to_lowercase().contains("passkey")
                || (k == "type"
                    && x.as_str()
                        .map(|s| s.eq_ignore_ascii_case("PASSKEY"))
                        .unwrap_or(false))
                || looks_like_passkey(x)
        }),
        Value::Array(a) => a.iter().any(looks_like_passkey),
        _ => false,
    }
}

/// 今のパスワードを読む（変更の前に、メモリにだけ持つ）
pub fn read_password(op: &dyn OpRunner, id: &str) -> OpResult<Option<Zeroizing<String>>> {
    Ok(get_login(op, id)?.password)
}

/// 保管庫（アーカイブ）へ移す。完全には消さない。
pub fn archive(op: &dyn OpRunner, id: &str) -> OpResult<()> {
    check_id(id)?;
    op.run(&["item", "delete", id, "--archive"], None)?;
    Ok(())
}

/// 秘密ではない情報（退会日・受付番号など）を項目に書き足す。
/// "vaultsweep" という欄を作り、その中に入れる。既存のメモ欄は変えない。
pub fn add_text_fields(op: &dyn OpRunner, id: &str, pairs: &[(&str, &str)]) -> OpResult<()> {
    check_id(id)?;
    let mut args: Vec<String> = vec!["item".into(), "edit".into(), id.into()];
    for (k, v) in pairs {
        let clean = |s: &str| -> String {
            s.chars()
                .filter(|c| !c.is_control() && !matches!(c, '.' | '=' | '[' | ']' | '\\'))
                .take(120)
                .collect()
        };
        let val: String = v.chars().filter(|c| !c.is_control()).take(200).collect();
        args.push(format!("vaultsweep.{}[text]={}", clean(k), val));
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    op.run(&refs, None)?;
    Ok(())
}

/// op://… の参照から値を読む（HIBP のキーなど）
pub fn read_ref(op: &dyn OpRunner, reference: &str) -> OpResult<Zeroizing<String>> {
    if !reference.starts_with("op://") || reference.chars().any(|c| c.is_control()) {
        return Err(OpError::Other("reference must start with op://".into()));
    }
    let out = op.run(&["read", reference], None)?;
    Ok(Zeroizing::new(
        String::from_utf8_lossy(&out).trim().to_string(),
    ))
}
