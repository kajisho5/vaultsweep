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
    /// 新しい版を自動で確かめる
    pub auto_update: bool,
    /// 本人が決めた、サイトごとのパスワードの決まり（ドメイン → 長さ・使う文字）
    pub site_rules: BTreeMap<String, crate::rules::SiteRule>,
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
            auto_update: true,
            site_rules: BTreeMap::new(),
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
        self.site_rules = std::mem::take(&mut self.site_rules)
            .into_iter()
            .filter(|(k, _)| {
                !k.is_empty()
                    && k.len() <= 253
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            })
            .take(2000)
            .map(|(k, v)| (k.to_lowercase(), v.sanitized()))
            .collect();
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

    /// 読む。ないときは None。一時的に読めないときは少し待ってやり直す。
    /// 壊れていて読めないときは、消さずに「.broken-日時」へ名前を変えて取っておき、None を返す。
    fn read_opt<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Result<Option<T>, String> {
        let path = self.dir.join(name);
        let mut last = String::new();
        for i in 0..5 {
            match std::fs::read(&path) {
                Ok(b) => {
                    return match serde_json::from_slice(&b) {
                        Ok(v) => Ok(Some(v)),
                        Err(e) => {
                            let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
                            let _ = std::fs::rename(
                                &path,
                                self.dir.join(format!("{name}.broken-{ts}")),
                            );
                            Err(format!("broken:{e}"))
                        }
                    };
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => {
                    last = e.to_string();
                    std::thread::sleep(std::time::Duration::from_millis(80 * (i + 1)));
                }
            }
        }
        Err(last)
    }
    fn read<T: for<'de> Deserialize<'de> + Default>(&self, name: &str) -> T {
        self.read_opt(name).ok().flatten().unwrap_or_default()
    }

    /// 同じフォルダを使うほかのプロセス（MCP・自動診断）やスレッドと、書き換えがぶつからないようにする。
    /// 同じスレッドの中では入れ子にしてよい。
    pub fn locked<R>(&self, f: impl FnOnce() -> Result<R, String>) -> Result<R, String> {
        thread_local! { static DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
        let outer = DEPTH.with(|d| d.get()) == 0;
        let guard = if outer {
            let _ = std::fs::create_dir_all(&self.dir);
            let f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(self.dir.join(".lock"))
                .map_err(|e| e.to_string())?;
            f.lock().map_err(|e| e.to_string())?;
            Some(f)
        } else {
            None
        };
        DEPTH.with(|d| d.set(d.get() + 1));
        let r = f();
        DEPTH.with(|d| d.set(d.get() - 1));
        drop(guard); // 閉じると鍵が外れる
        r
    }
    fn write<T: Serialize>(&self, name: &str, v: &T) -> Result<(), String> {
        write_atomic(
            &self.dir.join(name),
            &serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?,
        )
    }

    pub fn settings(&self) -> Settings {
        match self.read_opt::<Settings>("settings.json") {
            Ok(Some(s)) => s.sanitized(),
            Ok(None) => Settings::default(),
            // 読めないときは外へ問い合わせない側に倒す
            Err(_) => Settings {
                offline: true,
                ..Settings::default()
            },
        }
    }
    pub fn save_settings(&self, s: Settings) -> Result<Settings, String> {
        let s = s.sanitized();
        self.locked(|| self.write("settings.json", &s))?;
        Ok(s)
    }
    pub fn state(&self) -> State {
        self.read("state.json")
    }
    /// 読んで、変えて、書く（鍵をかけて）。一時的に読めないときは書かずにエラーにする。
    pub fn update_state<R>(&self, f: impl FnOnce(&mut State) -> R) -> Result<R, String> {
        self.locked(|| {
            let mut s = match self.read_opt::<State>("state.json") {
                Ok(v) => v.unwrap_or_default(),
                Err(e) if e.starts_with("broken:") => State::default(),
                Err(e) => return Err(e),
            };
            let r = f(&mut s);
            self.write("state.json", &s)?;
            Ok(r)
        })
    }
    pub fn report(&self) -> Option<Report> {
        self.read_opt("report.json").ok().flatten()
    }
    pub fn save_report(&self, r: &Report) -> Result<(), String> {
        self.write("report.json", r)
    }

    /// 診断結果を保存し、履歴を足す。新しく見つかった問題の数を返す。
    pub fn record_scan(&self, r: &Report) -> Result<usize, String> {
        self.locked(|| {
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
        })
    }

    /// パスワードを変えた項目の札を外し、記録する
    pub fn mark_changed(&self, id: &str) -> Result<Option<Report>, String> {
        self.locked(|| {
            let Some(mut r) = self.report() else {
                return Ok(None);
            };
            let mut title = String::new();
            let mut groups: Vec<(Kind, u32)> = vec![];
            let mut changed = false;
            if let Some(it) = r.items.iter_mut().find(|i| i.id == id) {
                title = it.title.clone();
                changed = it
                    .issues
                    .iter()
                    .any(|x| x.kind().fixed_by_password_change());
                groups = it
                    .issues
                    .iter()
                    .filter_map(|x| x.group().map(|g| (x.kind(), g)))
                    .collect();
                it.issues.retain(|x| !x.kind().fixed_by_password_change());
            }
            // 同じ（似た）パスワードの仲間が1件だけになったら、その札も外す
            for (kind, g) in groups {
                regroup(&mut r, kind, g);
            }
            r.recompute();
            self.save_report(&r)?;
            if !changed {
                return Ok(Some(r));
            }
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
        })
    }

    pub fn set_in_use(&self, id: &str, on: bool) -> Result<Option<Report>, String> {
        self.locked(|| {
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
        })
    }

    pub fn cancel_add(&self, id: &str) -> Result<(), String> {
        self.locked(|| {
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
                // 退会日は最初に「退会した」にした日のまま（記録を書き足しても変わらない）
                let was_left = e.state == "left";
                e.state = state.to_string();
                match state {
                    "left" if !was_left || e.left_at.is_none() => e.left_at = Some(now()),
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
        self.locked(|| {
            if let Some(mut r) = self.report() {
                r.items.retain(|i| i.id != id);
                r.recompute();
                self.save_report(&r)?;
            }
            Ok(())
        })
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

/// まとまり g の残りの仲間を数え直す
fn regroup(r: &mut Report, kind: Kind, g: u32) {
    let mates: Vec<usize> = r
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            i.issues
                .iter()
                .any(|x| x.kind() == kind && x.group() == Some(g))
        })
        .map(|(n, _)| n)
        .collect();
    for &n in &mates {
        let names: Vec<String> = mates
            .iter()
            .filter(|&&m| m != n)
            .map(|&m| r.items[m].title.clone())
            .collect();
        let it = &mut r.items[n];
        if names.is_empty() {
            it.issues
                .retain(|x| !(x.kind() == kind && x.group() == Some(g)));
            continue;
        }
        for x in it.issues.iter_mut().filter(|x| x.kind() == kind) {
            if let crate::model::Issue::Reuse {
                group,
                others,
                total,
            }
            | crate::model::Issue::Similar {
                group,
                others,
                total,
            } = x
                && *group == g
            {
                *total = names.len() as u32;
                *others = names.iter().take(3).cloned().collect();
            }
        }
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("tmp{}-{n}", std::process::id()));
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}
