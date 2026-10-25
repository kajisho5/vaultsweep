//! 判定：漏洩・事故サイト・使い回し・弱い・2段階未設定・使ってなさそう・重複

use crate::domain::{host_of, site_key};
use crate::hibp::sha1_hex;
use crate::lists::Lists;
use crate::model::{Issue, ItemReport, Kind, Report, SourcesUsed};
use crate::op::LoginFull;
use crate::store::Settings;
use crate::strength;
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use zeroize::Zeroizing;

/// パスワードそのものを手放したあとの、判定に必要なものだけ
pub struct Prepared {
    pub id: String,
    pub title: String,
    pub vault: String,
    pub host: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<String>,
    pub has_totp: bool,
    pub pw_sha256: Option<Zeroizing<[u8; 32]>>,
    pub pw_sha1: Option<Zeroizing<String>>,
    pub strength: Option<&'static str>,
    pub user_key: Option<[u8; 32]>,
    pub username_hint: Option<String>,
    /// メール漏洩チェック用（キーがあるときだけ・メモリだけ）
    pub email: Option<Zeroizing<String>>,
}

pub fn prepare(l: LoginFull, keep_email: bool) -> Prepared {
    // ドメインと表示用URLは同じURLから取る。URLの中のID・パスワード・?以降・#以降は捨てる
    let (host, url) = l
        .urls
        .iter()
        .find_map(|u| host_of(u).map(|h| (Some(h), clean_url(u))))
        .unwrap_or((None, None));
    let (pw_sha256, pw_sha1, strength) = match &l.password {
        Some(p) => {
            let mut h = Sha256::new();
            h.update(p.as_bytes());
            let d: [u8; 32] = h.finalize().into();
            let st = l
                .strength
                .as_deref()
                .and_then(strength::level_from_1p)
                .unwrap_or_else(|| strength::estimate(p));
            (
                Some(Zeroizing::new(d)),
                Some(Zeroizing::new(sha1_hex(p))),
                Some(st),
            )
        }
        None => (None, None, None),
    };
    let (user_key, username_hint, email) = match &l.username {
        Some(u) => {
            let norm = Zeroizing::new(u.trim().to_lowercase());
            let mut h = Sha256::new();
            h.update(norm.as_bytes());
            let hint = format!("{}•••", u.chars().take(2).collect::<String>());
            let email = (keep_email && norm.contains('@') && norm.contains('.'))
                .then(|| Zeroizing::new(norm.to_string()));
            (Some(h.finalize().into()), Some(hint), email)
        }
        None => (None, None, None),
    };
    Prepared {
        id: l.id.clone(),
        title: l.title.clone(),
        vault: l.vault.clone(),
        host,
        url,
        updated_at: l.updated_at.clone(),
        has_totp: l.has_totp,
        pw_sha256,
        pw_sha1,
        strength,
        user_key,
        username_hint,
        email,
    }
    // l はここで捨てられ、Zeroizing の中身は消える
}

/// "https://user:pw@a.example/x?token=1#y" → "https://a.example/x"
pub fn clean_url(raw: &str) -> Option<String> {
    let t = raw.trim();
    let with = if t.contains("://") { t.to_string() } else { format!("https://{t}") };
    let mut u = url::Url::parse(&with).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    let _ = u.set_username("");
    let _ = u.set_password(None);
    u.set_query(None);
    u.set_fragment(None);
    Some(u.to_string())
}

pub fn risk_of(issues: &[Issue]) -> f64 {
    let mut sev: Vec<f64> = issues.iter().map(|i| i.kind().severity()).collect();
    if sev.is_empty() {
        return 0.0;
    }
    sev.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let mut r = sev[0] + 0.06 * (sev.len() as f64 - 1.0);
    if let Some(Issue::Leak { count }) = issues.iter().find(|i| i.kind() == Kind::Leak)
        && *count >= 1000
    {
        r += 0.04;
    }
    (r.min(0.99) * 100.0).round() / 100.0
}

/// 安全度（100点満点）：問題のない項目ほど、危ない問題が少ないほど高い
pub fn score_of(items: &[ItemReport]) -> u32 {
    if items.is_empty() {
        return 100;
    }
    let pen: f64 = items.iter().map(|i| i.risk).sum();
    let s = 100.0 * (1.0 - pen / items.len() as f64 * 1.6);
    s.clamp(0.0, 100.0).round() as u32
}

fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

pub struct AuditInput<'a> {
    pub items: &'a [Prepared],
    pub pwned: Option<&'a HashMap<String, u64>>,
    pub lists: &'a Lists,
    pub settings: &'a Settings,
    pub in_use: &'a BTreeSet<String>,
    pub now: DateTime<Utc>,
}

pub fn audit(inp: AuditInput) -> Report {
    let items = inp.items;
    // 使い回し：同じ SHA-256 のまとまり
    let mut groups: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for (i, p) in items.iter().enumerate() {
        if let Some(h) = &p.pw_sha256 {
            groups.entry(**h).or_default().push(i);
        }
    }
    let mut reuse_group: HashMap<usize, (u32, Vec<usize>)> = HashMap::new();
    let mut gid = 0u32;
    let mut gl: Vec<&Vec<usize>> = groups.values().filter(|v| v.len() > 1).collect();
    gl.sort_by_key(|v| v[0]);
    for v in gl {
        gid += 1;
        for &i in v.iter() {
            reuse_group.insert(i, (gid, v.iter().copied().filter(|&j| j != i).collect()));
        }
    }
    // 重複：同じサイト＋同じユーザー名
    let mut dups: HashMap<(String, [u8; 32]), Vec<usize>> = HashMap::new();
    for (i, p) in items.iter().enumerate() {
        if let (Some(h), Some(u)) = (&p.host, &p.user_key) {
            dups.entry((site_key(h), *u)).or_default().push(i);
        }
    }
    let mut dup_of: HashMap<usize, usize> = HashMap::new();
    for v in dups.values().filter(|v| v.len() > 1) {
        for &i in v {
            let other = *v.iter().find(|&&j| j != i).unwrap();
            dup_of.insert(i, other);
        }
    }

    let mut out = Vec::with_capacity(items.len());
    for (i, p) in items.iter().enumerate() {
        let mut iss = vec![];
        let updated = p.updated_at.as_deref().and_then(parse_time);
        let days = updated.map(|u| (inp.now - u).num_days().max(0));
        if let (Some(m), Some(h)) = (inp.pwned, &p.pw_sha1)
            && let Some(c) = m.get(h.as_str())
        {
            iss.push(Issue::Leak { count: *c });
        }
        if let (Some(host), Some(u)) = (&p.host, updated) {
            let mut bs = inp.lists.breaches_for(host);
            bs.sort_by(|a, b| b.breach_date.cmp(&a.breach_date));
            for b in bs {
                // 事故の公表（HIBP への追加日）より後に変えていなければ
                let added = parse_time(&b.added_date).or_else(|| {
                    NaiveDate::parse_from_str(&b.breach_date, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|d| d.and_utc())
                });
                if let Some(a) = added
                    && u < a
                {
                    iss.push(Issue::Breach {
                        breach: if b.title.is_empty() {
                            b.name.clone()
                        } else {
                            b.title.clone()
                        },
                        date: b.breach_date.clone(),
                    });
                    break;
                }
            }
        }
        if let Some((g, others)) = reuse_group.get(&i) {
            iss.push(Issue::Reuse {
                group: *g,
                others: others
                    .iter()
                    .take(3)
                    .map(|&j| items[j].title.clone())
                    .collect(),
                total: others.len() as u32,
            });
        }
        if let Some(st) = p.strength
            && strength::is_weak(st, &inp.settings.weak_level)
        {
            iss.push(Issue::Weak {
                strength: st.to_string(),
            });
        }
        let twofa = p.host.as_deref().and_then(|h| inp.lists.totp_doc(h));
        if let Some(doc) = &twofa
            && !p.has_totp
        {
            iss.push(Issue::Nofa { doc: doc.clone() });
        }
        if let Some(d) = days
            && d >= inp.settings.stale_days
            && !inp.in_use.contains(&p.id)
        {
            iss.push(Issue::Stale { days: d });
        }
        if let Some(&j) = dup_of.get(&i) {
            iss.push(Issue::Dup {
                other: items[j].title.clone(),
                other_id: items[j].id.clone(),
            });
        }
        iss.sort_by(|a, b| {
            b.kind()
                .severity()
                .partial_cmp(&a.kind().severity())
                .unwrap()
        });
        out.push(ItemReport {
            id: p.id.clone(),
            title: p.title.clone(),
            vault: p.vault.clone(),
            domain: p.host.clone(),
            url: p.url.clone(),
            updated_at: p.updated_at.clone(),
            days_since_update: days,
            risk: risk_of(&iss),
            issues: iss,
            has_totp: p.has_totp,
            passkey_site: p
                .host
                .as_deref()
                .map(|h| inp.lists.passkey(h))
                .unwrap_or(false),
            twofa_doc: twofa.flatten(),
            deletion: p.host.as_deref().and_then(|h| inp.lists.deletion(h)),
            username_hint: p.username_hint.clone(),
        });
    }
    let mut warnings = vec![];
    if inp.settings.offline {
        warnings.push("offline".to_string());
    } else if inp.pwned.is_none() {
        warnings.push("hibp_failed".to_string());
    }
    let mut r = Report {
        scanned_at: inp
            .now
            .with_timezone(&chrono::Local)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
        total: out.len(),
        score: 0,
        counts: Default::default(),
        items: out,
        sources: SourcesUsed {
            pwned_passwords: inp.pwned.is_some(),
            breaches: inp.lists.breaches.is_some(),
            twofa: inp.lists.totp.is_some(),
            passkeys: inp.lists.passkeys.is_some(),
            justdeleteme: inp.lists.jdm.is_some(),
        },
        warnings,
        email_breaches: vec![],
    };
    r.recompute();
    r
}
