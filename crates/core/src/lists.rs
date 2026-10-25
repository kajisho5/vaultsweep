//! 外部の一覧データ（1日1回だけ取りに行き、パソコンに保存して使う）
//! - 2FA Directory（MIT）: 2段階認証に対応しているサイト
//! - Passkeys Directory（CC BY 4.0）: パスキーに対応しているサイト
//! - JustDeleteMe（MIT）: 退会ページと難しさ
//! - HIBP の事故サイト一覧

use crate::domain::candidates;
use crate::hibp::{self, Breach};
use crate::model::Deletion;
use crate::net::Fetcher;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime};

pub const TOTP_URL: &str = "https://api.2fa.directory/v4/totp.json";
pub const PASSKEYS_URL: &str = "https://passkeys-api.2fa.directory/v1/supported.json";
pub const JDM_URL: &str =
    "https://raw.githubusercontent.com/jdm-contrib/jdm/master/_data/sites.json";

const MAX_AGE: Duration = Duration::from_secs(24 * 3600);

/// 保存が1日以内ならそれを使う。古ければ取りに行き、失敗したら古い保存を使う。
pub fn cached(cache_dir: &Path, name: &str, url: &str, f: Option<&dyn Fetcher>) -> Option<Vec<u8>> {
    let path = cache_dir.join(name);
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|age| age < MAX_AGE)
        .unwrap_or(false);
    if fresh && let Ok(b) = std::fs::read(&path) {
        return Some(b);
    }
    if let Some(f) = f
        && let Ok((200, body)) = f.get(url, &[])
    {
        let _ = std::fs::create_dir_all(cache_dir);
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, &body).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
        return Some(body);
    }
    std::fs::read(&path).ok()
}

#[derive(Default)]
pub struct Lists {
    /// ドメイン → 説明ページ
    pub totp: Option<HashMap<String, Option<String>>>,
    pub passkeys: Option<HashSet<String>>,
    pub jdm: Option<HashMap<String, Deletion>>,
    pub breaches: Option<HashMap<String, Vec<Breach>>>,
}

pub fn parse_totp(b: &[u8]) -> Option<HashMap<String, Option<String>>> {
    #[derive(Deserialize)]
    struct E {
        #[serde(default)]
        methods: Vec<String>,
        documentation: Option<String>,
    }
    let m: HashMap<String, E> = serde_json::from_slice(b).ok()?;
    Some(
        m.into_iter()
            .filter(|(_, e)| e.methods.iter().any(|x| x == "totp"))
            .map(|(k, e)| (k.to_lowercase(), e.documentation))
            .collect(),
    )
}

pub fn parse_passkeys(b: &[u8]) -> Option<HashSet<String>> {
    let m: HashMap<String, serde_json::Value> = serde_json::from_slice(b).ok()?;
    Some(m.into_keys().map(|k| k.to_lowercase()).collect())
}

pub fn parse_jdm(b: &[u8]) -> Option<HashMap<String, Deletion>> {
    #[derive(Deserialize)]
    struct S {
        url: Option<String>,
        #[serde(default)]
        difficulty: String,
        #[serde(default)]
        domains: Vec<String>,
        notes: Option<String>,
        email: Option<String>,
        email_subject: Option<String>,
        email_body: Option<String>,
    }
    let v: Vec<S> = serde_json::from_slice(b).ok()?;
    let mut m = HashMap::new();
    for s in v {
        let d = Deletion {
            url: s
                .url
                .filter(|u| u.starts_with("https://") || u.starts_with("http://")),
            difficulty: s.difficulty,
            email: s.email,
            email_subject: s.email_subject,
            email_body: s.email_body,
            notes: s.notes,
        };
        for dom in s.domains {
            m.entry(dom.to_lowercase()).or_insert_with(|| d.clone());
        }
    }
    Some(m)
}

impl Lists {
    pub fn load(cache_dir: &Path, f: Option<&dyn Fetcher>) -> Lists {
        Lists {
            totp: cached(cache_dir, "totp.json", TOTP_URL, f).and_then(|b| parse_totp(&b)),
            passkeys: cached(cache_dir, "passkeys.json", PASSKEYS_URL, f)
                .and_then(|b| parse_passkeys(&b)),
            jdm: cached(cache_dir, "jdm.json", JDM_URL, f).and_then(|b| parse_jdm(&b)),
            breaches: cached(cache_dir, "breaches.json", hibp::BREACHES_URL, f)
                .and_then(|b| hibp::parse_breaches(&b).ok()),
        }
    }

    fn find<'a, T>(m: &'a HashMap<String, T>, host: &str) -> Option<&'a T> {
        candidates(host).iter().find_map(|c| m.get(c))
    }

    pub fn totp_doc(&self, host: &str) -> Option<Option<String>> {
        self.totp
            .as_ref()
            .and_then(|m| Self::find(m, host).cloned())
    }
    pub fn passkey(&self, host: &str) -> bool {
        self.passkeys
            .as_ref()
            .map(|s| candidates(host).iter().any(|c| s.contains(c)))
            .unwrap_or(false)
    }
    pub fn deletion(&self, host: &str) -> Option<Deletion> {
        self.jdm.as_ref().and_then(|m| Self::find(m, host).cloned())
    }
    pub fn breaches_for(&self, host: &str) -> Vec<Breach> {
        self.breaches
            .as_ref()
            .and_then(|m| Self::find(m, host).cloned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_lists() {
        let t = parse_totp(br#"{"zoom.us":{"methods":["sms","totp"],"documentation":"https://d"},"x.example":{"methods":["sms"]}}"#).unwrap();
        assert!(t.contains_key("zoom.us"));
        assert!(!t.contains_key("x.example"));
        let p = parse_passkeys(br#"{"a.example":{"mfa":"allowed"}}"#).unwrap();
        assert!(p.contains("a.example"));
        let j = parse_jdm(br#"[{"name":"A","url":"https://a.example/del","difficulty":"easy","domains":["a.example","b.a.example"]}]"#).unwrap();
        assert_eq!(j["b.a.example"].difficulty, "easy");
        let l = Lists {
            totp: Some(t),
            passkeys: Some(p),
            jdm: Some(j),
            breaches: None,
        };
        assert!(l.totp_doc("us.zoom.us").is_some());
        assert!(l.passkey("login.a.example"));
        assert!(l.deletion("x.a.example").is_some());
    }
}
