//! クリップボード。Windows では「クリップボードの履歴」と「ほかの端末との同期」に残らない印を付ける。
//! 30秒たって、まだ自分が入れた値のままなら消す。

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

fn digest(s: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().into()
}

#[cfg(windows)]
mod imp {
    use clipboard_win::{Clipboard, raw};
    use zeroize::Zeroizing;

    pub fn write(value: &str) -> Result<(), String> {
        let _c = Clipboard::new_attempts(10).map_err(|e| e.to_string())?;
        raw::set_string(value).map_err(|e| e.to_string())?;
        // Microsoft の決まり：履歴・クラウド同期・監視アプリに渡さない
        let zero = [0u8; 4];
        for name in ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"] {
            if let Some(f) = raw::register_format(name) {
                let _ = raw::set_without_clear(f.get(), &zero);
            }
        }
        Ok(())
    }

    pub fn read() -> Option<Zeroizing<String>> {
        let _c = Clipboard::new_attempts(10).ok()?;
        let mut buf = Zeroizing::new(Vec::new());
        raw::get_string(&mut buf).ok()?;
        Some(Zeroizing::new(String::from_utf8_lossy(&buf).to_string()))
    }

    pub fn clear() {
        if let Ok(_c) = Clipboard::new_attempts(10) {
            let _ = raw::empty();
        }
    }
}

#[cfg(not(windows))]
mod imp {
    // Windows 以外（開発用）。tauri のプラグインを使う。
    use std::sync::OnceLock;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    use zeroize::Zeroizing;
    pub static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

    pub fn write(value: &str) -> Result<(), String> {
        let app = APP.get().ok_or("no app")?;
        app.clipboard().write_text(value.to_string()).map_err(|e| e.to_string())
    }
    pub fn read() -> Option<Zeroizing<String>> {
        APP.get()?.clipboard().read_text().ok().map(Zeroizing::new)
    }
    pub fn clear() {
        if let Some(app) = APP.get() {
            let _ = app.clipboard().write_text(String::new());
        }
    }
}

#[cfg(not(windows))]
pub fn init(app: &tauri::AppHandle) {
    let _ = imp::APP.set(app.clone());
}
#[cfg(windows)]
pub fn init(_app: &tauri::AppHandle) {}

pub const CLEAR_SECS: u64 = 30;

/// 秘密ではない文字（ダミー情報・設定）をコピーする
pub fn copy_plain(text: &str) -> Result<(), String> {
    imp::write(text)
}

/// 値を入れて、30秒後に消す予約をする
pub fn copy_with_timeout(value: Zeroizing<String>) -> Result<(), String> {
    imp::write(&value)?;
    let d = digest(&value);
    drop(value);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(CLEAR_SECS));
        if let Some(now) = imp::read() {
            if digest(&now) == d {
                imp::clear();
            }
        }
    });
    Ok(())
}
