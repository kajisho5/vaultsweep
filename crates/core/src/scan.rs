//! 診断の流れ（画面・自動診断・MCP で共通）

use crate::audit::{self, AuditInput, Prepared};
use crate::hibp;
use crate::lists::Lists;
use crate::model::{EmailBreach, Report};
use crate::net::Fetcher;
use crate::op::{self, OpError, OpRunner};
use crate::store::Store;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Progress {
    Connecting,
    Reading { done: usize, total: usize },
    Checking { done: usize, total: usize },
    Lists,
    Emails { done: usize, total: usize },
    Done,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub report: Report,
    pub new_issues: usize,
}

pub fn run(
    opr: &dyn OpRunner,
    fetcher: Option<&dyn Fetcher>,
    store: &Store,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<ScanResult, OpError> {
    let settings = store.settings();
    let state = store.state();
    let online = !settings.offline && fetcher.is_some();
    let fetch = if online { fetcher } else { None };

    progress(Progress::Connecting);
    let raw = op::list_logins_raw(opr)?;
    let listed = op::parse_list(&raw).map(|l| l.len()).unwrap_or(0);
    let mut prog = |d: usize, t: usize| progress(Progress::Reading { done: d, total: t });
    let logins = op::get_logins(opr, &raw, &mut prog)?;
    drop(raw);

    let hibp_key = match (&settings.hibp_key_ref, online) {
        (Some(r), true) => op::read_ref(opr, r).ok(),
        _ => None,
    };
    let prepared: Vec<Prepared> = logins
        .into_iter()
        .map(|l| audit::prepare(l, hibp_key.is_some()))
        .collect();

    let mut pwned_partial = false;
    let pwned = match fetch {
        Some(f) => {
            let hashes: Vec<String> = prepared
                .iter()
                .filter_map(|p| p.pw_sha1.as_ref().map(|h| h.to_string()))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let cb = |d: usize, t: usize| progress(Progress::Checking { done: d, total: t });
            hibp::check_hashes(f, &hashes, &cb).ok().map(|mut m| {
                if m.remove(hibp::PARTIAL).is_some() {
                    pwned_partial = true;
                }
                m
            })
        }
        None => None,
    };

    progress(Progress::Lists);
    let lists = Lists::load(&store.cache_dir(), fetch);

    let mut report = audit::audit(AuditInput {
        items: &prepared,
        pwned: pwned.as_ref(),
        lists: &lists,
        settings: &settings,
        in_use: &state.in_use,
        now: chrono::Utc::now(),
    });

    if pwned_partial {
        report.warnings.push("hibp_partial".into());
    }
    if prepared.len() < listed {
        report
            .warnings
            .push(format!("skipped:{}", listed - prepared.len()));
    }
    if let (Some(key), Some(f)) = (&hibp_key, fetch) {
        let emails: Vec<String> = prepared
            .iter()
            .filter_map(|p| p.email.as_ref().map(|e| e.to_string()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(20)
            .collect();
        let total = emails.len();
        for (i, e) in emails.iter().enumerate() {
            if i > 0 {
                // 有料キーの一番安いプランは1分10回まで
                std::thread::sleep(std::time::Duration::from_millis(6500));
            }
            match hibp::breached_account(f, key, e) {
                Ok(b) if !b.is_empty() => report.email_breaches.push(EmailBreach {
                    masked: hibp::mask_email(e),
                    breaches: b,
                }),
                Ok(_) => {}
                Err(_) => {
                    report.warnings.push("hibp_email_failed".into());
                    break;
                }
            }
            progress(Progress::Emails { done: i + 1, total });
        }
    }
    drop(prepared);

    let new_issues = store.record_scan(&report).map_err(OpError::Other)?;
    progress(Progress::Done);
    Ok(ScanResult { report, new_issues })
}
