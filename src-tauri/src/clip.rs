//! クリップボード。Windows では「クリップボードの履歴」と「ほかの端末との同期」に残らない印を付ける。
//! Mac では「秘密」「一時的」の印（nspasteboard.org の取り決め）を付け、履歴アプリや Spotlight の履歴に残りにくくする。
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

#[cfg(target_os = "macos")]
mod imp {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::{NSData, NSString};
    use zeroize::Zeroizing;

    pub fn write(value: &str) -> Result<(), String> {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ok = pb.setString_forType(&NSString::from_str(value), unsafe { NSPasteboardTypeString });
            if !ok {
                return Err("pasteboard".to_string());
            }
            // 中身は空でよい。印があることが大事
            let empty = NSData::new();
            for ty in ["org.nspasteboard.ConcealedType", "org.nspasteboard.TransientType"] {
                pb.setData_forType(Some(&empty), &NSString::from_str(ty));
            }
            Ok(())
        })
    }

    pub fn read() -> Option<Zeroizing<String>> {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            let s = pb.stringForType(unsafe { NSPasteboardTypeString })?;
            Some(Zeroizing::new(s.to_string()))
        })
    }

    pub fn clear() {
        autoreleasepool(|_| {
            NSPasteboard::generalPasteboard().clearContents();
        });
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    // Windows・Mac 以外（開発用）。tauri のプラグインを使う。
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

#[cfg(not(any(windows, target_os = "macos")))]
pub fn init(app: &tauri::AppHandle) {
    let _ = imp::APP.set(app.clone());
}
#[cfg(any(windows, target_os = "macos"))]
pub fn init(_app: &tauri::AppHandle) {}

pub const CLEAR_SECS: u64 = 30;

/// 秘密ではない文字（ダミー情報・設定）をコピーする
pub fn copy_plain(text: &str) -> Result<(), String> {
    imp::write(text)
}

static LAST: std::sync::Mutex<Option<[u8; 32]>> = std::sync::Mutex::new(None);

/// 自分が入れたパスワードがまだクリップボードにあれば消す（変更の終わり・アプリ終了で呼ぶ）
pub fn clear_if_ours() {
    let Some(d) = LAST.lock().unwrap().take() else { return };
    if let Some(now) = imp::read() {
        if digest(&now) == d {
            imp::clear();
        }
    }
}

/// 値を入れて、30秒後に消す予約をする
pub fn copy_with_timeout(value: Zeroizing<String>) -> Result<(), String> {
    imp::write(&value)?;
    let d = digest(&value);
    *LAST.lock().unwrap() = Some(d);
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
