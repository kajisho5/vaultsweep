//! Have I Been Pwned
//! - 漏洩パスワード：SHA-1 の先頭5文字だけを送る（k-匿名性）。Add-Padding で件数も隠す。
//! - 事故サイト一覧：キー不要。
//! - メールアドレスの検索：有料キーが必要。1分あたりの回数に上限がある。

use crate::net::Fetcher;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::sync::Mutex;

/// パスワード → SHA-1（大文字16進）
pub fn sha1_hex(pw: &str) -> String {
    let mut h = Sha1::new();
    h.update(pw.as_bytes());
    hex::encode_upper(h.finalize())
}

/// check_hashes の結果に入る「一部失敗」の印（ハッシュとは重ならない）
pub const PARTIAL: &str = "!partial";

/// 範囲の返事 "SUFFIX:COUNT" を読む。件数0（水増し分）は捨てる。
pub fn parse_range(body: &str) -> HashMap<String, u64> {
    body.lines()
        .filter_map(|l| {
            let (s, c) = l.trim().split_once(':')?;
            let c: u64 = c.trim().parse().ok()?;
            (c > 0).then(|| (s.trim().to_uppercase(), c))
        })
        .collect()
}

/// SHA-1 の一覧を調べて、載っていた回数を返す（載っていないものは入らない）
pub fn check_hashes(
    f: &dyn Fetcher,
    hashes: &[String],
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<HashMap<String, u64>, String> {
    let mut by_prefix: HashMap<String, Vec<String>> = HashMap::new();
    for h in hashes {
        if h.len() == 40 {
            by_prefix
                .entry(h[..5].to_string())
                .or_default()
                .push(h.clone());
        }
    }
    let prefixes: Vec<String> = by_prefix.keys().cloned().collect();
    let total = prefixes.len();
    let result = Mutex::new(HashMap::new());
    let errors = Mutex::new(Vec::<String>::new());
    let next = Mutex::new(0usize);
    let done = Mutex::new(0usize);
    std::thread::scope(|s| {
        for _ in 0..6.min(total.max(1)) {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        if *n >= total {
                            break;
                        }
                        *n += 1;
                        *n - 1
                    };
                    let p = &prefixes[i];
                    let url = format!("https://api.pwnedpasswords.com/range/{p}");
                    let mut res = f.get(&url, &[("Add-Padding", "true")]);
                for _ in 0..2 {
                    if matches!(res, Ok((200, _))) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    res = f.get(&url, &[("Add-Padding", "true")]);
                }
                match res {
                        Ok((200, body)) => {
                            let m = parse_range(&String::from_utf8_lossy(&body));
                            let mut r = result.lock().unwrap();
                            for full in &by_prefix[p] {
                                if let Some(c) = m.get(&full[5..]) {
                                    r.insert(full.clone(), *c);
                                }
                            }
                        }
                        Ok((code, _)) => errors.lock().unwrap().push(format!("HTTP {code}")),
                        Err(e) => errors.lock().unwrap().push(e),
                    }
                    let mut d = done.lock().unwrap();
                    *d += 1;
                    progress(*d, total);
                }
            });
        }
    });
    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() && errors.len() == total {
        return Err(errors[0].clone());
    }
    let mut r = result.into_inner().unwrap();
    // 一部だけ失敗したことを呼ぶ側に伝える（特別なキー）
    if !errors.is_empty() {
        r.insert(PARTIAL.to_string(), errors.len() as u64);
    }
    Ok(r)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Breach {
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub breach_date: String,
    #[serde(default)]
    pub added_date: String,
    #[serde(default)]
    pub data_classes: Vec<String>,
    #[serde(default)]
    pub is_verified: bool,
    #[serde(default)]
    pub is_fabricated: bool,
    #[serde(default)]
    pub is_spam_list: bool,
}

/// パスワードが漏れた（確認済みの）事故だけ、ドメインごとに
pub fn parse_breaches(body: &[u8]) -> Result<HashMap<String, Vec<Breach>>, String> {
    let all: Vec<Breach> = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let mut m: HashMap<String, Vec<Breach>> = HashMap::new();
    for b in all {
        if b.domain.is_empty() || b.is_fabricated || b.is_spam_list || !b.is_verified {
            continue;
        }
        if !b.data_classes.iter().any(|d| d == "Passwords") {
            continue;
        }
        m.entry(b.domain.to_lowercase()).or_default().push(b);
    }
    Ok(m)
}

pub const BREACHES_URL: &str = "https://haveibeenpwned.com/api/v3/breaches";

/// メールアドレスが載っている事故の名前（有料キーが必要）
pub fn breached_account(f: &dyn Fetcher, key: &str, email: &str) -> Result<Vec<String>, String> {
    let enc: String = url::form_urlencoded::byte_serialize(email.as_bytes()).collect();
    let url =
        format!("https://haveibeenpwned.com/api/v3/breachedaccount/{enc}?truncateResponse=true");
    match f.get(&url, &[("hibp-api-key", key)])? {
        (200, body) => {
            #[derive(Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct N {
                name: String,
            }
            let v: Vec<N> = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
            Ok(v.into_iter().map(|n| n.name).collect())
        }
        (404, _) => Ok(vec![]),
        (401, _) => Err("HIBP key rejected (401)".into()),
        (429, _) => Err("HIBP rate limit (429)".into()),
        (c, _) => Err(format!("HTTP {c}")),
    }
}

/// "taro@example.com" → "ta•••@example.com"
pub fn mask_email(e: &str) -> String {
    match e.split_once('@') {
        Some((l, d)) => format!("{}•••@{}", l.chars().take(2).collect::<String>(), d),
        None => "•••".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sha_and_range() {
        // "password" の SHA-1
        assert_eq!(
            sha1_hex("password"),
            "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8"
        );
        let m = parse_range("1E4C9B93F3F0682250B6CF8331B7EE68FD8:3861493\r\nAAAA:0\nBBBB:2");
        assert_eq!(m.get("1E4C9B93F3F0682250B6CF8331B7EE68FD8"), Some(&3861493));
        assert!(!m.contains_key("AAAA"));
        assert_eq!(mask_email("taro@example.com"), "ta•••@example.com");
    }
    #[test]
    fn breaches_filter() {
        let j = br#"[{"Name":"A","Title":"A","Domain":"a.example","BreachDate":"2020-01-01","AddedDate":"2020-02-01T00:00:00Z","DataClasses":["Email addresses","Passwords"],"IsVerified":true,"IsFabricated":false,"IsSpamList":false},
        {"Name":"B","Domain":"b.example","DataClasses":["Email addresses"],"IsVerified":true},
        {"Name":"C","Domain":"","DataClasses":["Passwords"],"IsVerified":true}]"#;
        let m = parse_breaches(j).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m["a.example"][0].breach_date, "2020-01-01");
    }
}
