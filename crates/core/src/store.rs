//! パソコンに保存するもの（秘密の値は保存しない）
//! - settings.json : 設定
//! - state.json    : 「使っている」印・解約リスト・履歴・AIからの依頼
//! - report.json   : 最後の診断結果（サイト名・問題の種類だけ）
//! - cache/        : 外部の一覧データ

use crate::model::{Kind, Report};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "ja" / "en"。None ならパソコンの言語に合わせる
    pub lang: Option<String>,
    /// "system" / "light" / "dark"
    pub theme: String,
    pub auto_scan: bool,
    /// "weekly" / "daily" / "monthly"
    pub schedule: String,
    /// "HH:MM"
    pub schedule_time: String,
    /// "new" / "always" / "never"
    pub notify: String,
    /// 外部に問い合わせない
    pub offline: bool,
    pub stale_days: i64,
    /// "weak" / "fair"
    pub weak_level: String,
    pub pw_length: u32,
    pub pw_symbols: bool,
    /// HIBP のキーの場所（op://保管庫/項目/欄）。キーそのものは保存しない
    pub hibp_key_ref: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: None,
            theme: "system".into(),
            auto_scan: false,
            schedule: "weekly".into(),
            schedule_time: "09:00".into(),
            notify: "new".into(),
            offline: false,
            stale_days: 365,
            weak_level: "weak".into(),
            pw_length: 24,
            pw_symbols: true,
            hibp_key_ref: None,
        }
    }
}

impl Settings {
    /// おかしな値を直す
    pub fn sanitized(mut self) -> Self {
        let d = Settings::default();
        if !matches!(self.lang.as_deref(), None | Some("ja") | Some("en")) {
            self.lang = None;
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = d.theme;
        }
        if !matches!(self.schedule.as_str(), "weekly" | "daily" | "monthly") {
            self.schedule = d.schedule;
        }
        let ok_time = self.schedule_time.len() == 5
            && self.schedule_time.as_bytes()[2] == b':'
            && self.schedule_time[..2]
                .parse::<u8>()
                .map(|h| h < 24)
                .unwrap_or(false)
            && self.schedule_time[3..]
                .parse::<u8>()
                .map(|m| m < 60)
                .unwrap_or(false);
        if !ok_time {
            self.schedule_time = d.schedule_time;
        }
        if !matches!(self.notify.as_str(), "new" | "always" | "never") {
            self.notify = d.notify;
        }
        self.stale_days = self.stale_days.clamp(30, 3650);
        if !matches!(self.weak_level.as_str(), "weak" | "fair") {
            self.weak_level = d.weak_level;
        }
        self.pw_length = self.pw_length.clamp(12, 64);
        if let Some(r) = &self.hibp_key_ref
            && !r.starts_with("op://")
        {
            self.hibp_key_ref = None;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelEntry {
    /// "todo"（まだ） / "left"（退会した） / "archived"（片付けた）
    pub state: String,
    pub title: String,
    pub domain: Option<String>,
    pub added_at: String,
    pub left_at: Option<String>,
    pub archived_at: Option<String>,
    #[serde(default)]
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: String,
    pub score: u32,
    pub flagged: usize,
    pub new_issues: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixedEntry {
    pub at: String,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pending {
    pub id: String,
    /// "change" / "archive"
    pub kind: String,
    pub item_id: String,
    pub title: String,
    pub at: String,
    pub from: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub in_use: BTreeSet<String>,
    pub cancel: BTreeMap<String, CancelEntry>,
    pub history: Vec<HistoryEntry>,
    pub fixed: Vec<FixedEntry>,
    pub pending: Vec<Pending>,
    /// 前回までに見つかった "項目ID:種類"（新しい問題の判定用）
    pub known: BTreeSet<String>,
}

pub struct Store {
    pub dir: PathBuf,
}

fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

impl Store {
    pub fn default_dir() -> PathBuf {
        if let Some(d) = std::env::var_os("VAULTSWEEP_DATA_DIR") {
            return PathBuf::from(d);
        }
        dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("vaultsweep")
    }
    pub fn open_default() -> Store {
        Store::new(Self::default_dir())
    }
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        let dir = dir.into();
        let _ = std::fs::create_dir_all(&dir);
        Store { dir }
    }
    pub fn cache_dir(&self) -> PathBuf {
        self.dir.join("cache")
    }

    fn read<T: for<'de> Deserialize<'de> + Default>(&self, name: &str) -> T {
        std::fs::read(self.dir.join(name))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    fn write<T: Serialize>(&self, name: &str, v: &T) -> Result<(), String> {
        write_atomic(
            &self.dir.join(name),
            &serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?,
        )
    }

    pub fn settings(&self) -> Settings {
        self.read::<Settings>("settings.json").sanitized()
    }
    pub fn save_settings(&self, s: Settings) -> Result<Settings, String> {
        let s = s.sanitized();
        self.write("settings.json", &s)?;
        Ok(s)
    }
    pub fn state(&self) -> State {
        self.read("state.json")
    }
    /// 読んで、変えて、書く
    pub fn update_state<R>(&self, f: impl FnOnce(&mut State) -> R) -> Result<R, String> {
        let mut s = self.state();
        let r = f(&mut s);
        self.write("state.json", &s)?;
        Ok(r)
    }
    pub fn report(&self) -> Option<Report> {
        std::fs::read(self.dir.join("report.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }
    pub fn save_report(&self, r: &Report) -> Result<(), String> {
        self.write("report.json", r)
    }

    /// 診断結果を保存し、履歴を足す。新しく見つかった問題の数を返す。
    pub fn record_scan(&self, r: &Report) -> Result<usize, String> {
        self.save_report(r)?;
        self.update_state(|s| {
            let keys: BTreeSet<String> = r
                .items
                .iter()
                .flat_map(|i| {
                    i.issues
                        .iter()
                        .map(move |x| format!("{}:{}", i.id, x.kind().as_str()))
                })
                .collect();
            let first = s.history.is_empty() && s.known.is_empty();
            let new_issues = if first {
                0
            } else {
                keys.difference(&s.known).count()
            };
            s.known = keys;
            s.history.push(HistoryEntry {
                at: r.scanned_at.clone(),
                score: r.score,
                flagged: r.flagged_count(),
                new_issues,
            });
            if s.history.len() > 200 {
                let cut = s.history.len() - 200;
                s.history.drain(..cut);
            }
            new_issues
        })
    }

    /// パスワードを変えた項目の札を外し、記録する
    pub fn mark_changed(&self, id: &str) -> Result<Option<Report>, String> {
        let Some(mut r) = self.report() else {
            return Ok(None);
        };
        let mut title = String::new();
        let mut group = None;
        if let Some(it) = r.items.iter_mut().find(|i| i.id == id) {
            title = it.title.clone();
            group = it.issues.iter().find_map(|x| match x {
                crate::model::Issue::Reuse { group, .. } => Some(*group),
                _ => None,
            });
            it.issues.retain(|x| !x.kind().fixed_by_password_change());
        }
        if let Some(g) = group {
            // 同じパスワードの仲間が1件だけになったら、その札も外す
            let mates: Vec<usize> = r
                .items
                .iter()
                .enumerate()
                .filter(|(_, i)| {
                    i.issues.iter().any(
                        |x| matches!(x, crate::model::Issue::Reuse { group, .. } if *group == g),
                    )
                })
                .map(|(n, _)| n)
                .collect();
            for &n in &mates {
                let others: Vec<String> = mates
                    .iter()
                    .filter(|&&m| m != n)
                    .map(|&m| r.items[m].title.clone())
                    .collect();
                let it = &mut r.items[n];
                if others.is_empty() {
                    it.issues.retain(|x| x.kind() != Kind::Reuse);
                } else {
                    for x in it.issues.iter_mut() {
                        if let crate::model::Issue::Reuse {
                            others: o, total, ..
                        } = x
                        {
                            *total = others.len() as u32;
                            *o = others.iter().take(3).cloned().collect();
                        }
                    }
                }
            }
        }
        r.recompute();
        self.save_report(&r)?;
        self.update_state(|s| {
            s.fixed.push(FixedEntry {
                at: now(),
                id: id.to_string(),
                title,
            });
            if s.fixed.len() > 1000 {
                s.fixed.remove(0);
            }
        })?;
        Ok(Some(r))
    }

    pub fn set_in_use(&self, id: &str, on: bool) -> Result<Option<Report>, String> {
        self.update_state(|s| {
            if on {
                s.in_use.insert(id.to_string());
            } else {
                s.in_use.remove(id);
            }
        })?;
        let Some(mut r) = self.report() else {
            return Ok(None);
        };
        if on {
            if let Some(it) = r.items.iter_mut().find(|i| i.id == id) {
                it.issues.retain(|x| x.kind() != Kind::Stale);
            }
            r.recompute();
            self.save_report(&r)?;
        }
        Ok(Some(r))
    }

    pub fn cancel_add(&self, id: &str) -> Result<(), String> {
        let r = self.report();
        let it = r.as_ref().and_then(|r| r.items.iter().find(|i| i.id == id));
        let title = it
            .map(|i| i.title.clone())
            .unwrap_or_else(|| id.to_string());
        let domain = it.and_then(|i| i.domain.clone());
        self.update_state(|s| {
            s.cancel.entry(id.to_string()).or_insert(CancelEntry {
                state: "todo".into(),
                title,
                domain,
                added_at: now(),
                left_at: None,
                archived_at: None,
                receipt: None,
            });
        })
    }

    pub fn cancel_set(&self, id: &str, state: &str, receipt: Option<String>) -> Result<(), String> {
        if !matches!(state, "todo" | "left" | "archived" | "remove") {
            return Err("bad state".into());
        }
        self.update_state(|s| {
            if state == "remove" {
                s.cancel.remove(id);
                return;
            }
            if let Some(e) = s.cancel.get_mut(id) {
                e.state = state.to_string();
                match state {
                    "left" => e.left_at = Some(now()),
                    "archived" => e.archived_at = Some(now()),
                    _ => {}
                }
                if receipt.is_some() {
                    e.receipt = receipt;
                }
            }
        })
    }

    /// 保管庫へ移した項目を結果から外す
    pub fn remove_item(&self, id: &str) -> Result<(), String> {
        if let Some(mut r) = self.report() {
            r.items.retain(|i| i.id != id);
            r.recompute();
            self.save_report(&r)?;
        }
        Ok(())
    }

    pub fn add_pending(
        &self,
        kind: &str,
        item_id: &str,
        title: &str,
        from: &str,
    ) -> Result<Pending, String> {
        let id = format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let p = Pending {
            id,
            kind: kind.into(),
            item_id: item_id.into(),
            title: title.into(),
            at: now(),
            from: from.into(),
        };
        let p2 = p.clone();
        self.update_state(move |s| {
            s.pending
                .retain(|x| !(x.kind == p2.kind && x.item_id == p2.item_id));
            s.pending.push(p2);
        })?;
        Ok(p)
    }

    pub fn take_pending(&self, id: &str) -> Result<Option<Pending>, String> {
        self.update_state(|s| {
            let pos = s.pending.iter().position(|p| p.id == id)?;
            Some(s.pending.remove(pos))
        })
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}
