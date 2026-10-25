//! 画面・MCP・保存ファイルで共通に使うデータの形。
//! ここにはパスワード・ユーザー名・メモ欄を一切入れない。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Leak,
    Breach,
    Reuse,
    Similar,
    Weak,
    Nofa,
    Stale,
    Dup,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Leak,
        Kind::Breach,
        Kind::Reuse,
        Kind::Similar,
        Kind::Weak,
        Kind::Nofa,
        Kind::Stale,
        Kind::Dup,
    ];

    /// 危なさ（0〜1）。並び順と安全度の計算に使う。
    pub fn severity(self) -> f64 {
        match self {
            Kind::Leak => 0.9,
            Kind::Breach => 0.82,
            Kind::Reuse => 0.7,
            Kind::Similar => 0.66,
            Kind::Weak => 0.62,
            Kind::Nofa => 0.5,
            Kind::Stale => 0.42,
            Kind::Dup => 0.36,
        }
    }

    /// パスワードを変えれば消える種類か
    pub fn fixed_by_password_change(self) -> bool {
        matches!(
            self,
            Kind::Leak | Kind::Breach | Kind::Reuse | Kind::Similar | Kind::Weak
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Leak => "leak",
            Kind::Breach => "breach",
            Kind::Reuse => "reuse",
            Kind::Similar => "similar",
            Kind::Weak => "weak",
            Kind::Nofa => "nofa",
            Kind::Stale => "stale",
            Kind::Dup => "dup",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// 問題1つ分。文章は画面側で言語に合わせて組み立てる。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Issue {
    /// 流出データに載っていた回数
    Leak { count: u64 },
    /// 事故のあったサイト名と事故の日付（YYYY-MM-DD）
    Breach { breach: String, date: String },
    /// 同じパスワードの他の項目（最大3件の名前と総数）。group は番号だけ（ハッシュは入れない）
    Reuse {
        group: u32,
        others: Vec<String>,
        total: u32,
    },
    /// 1〜2文字だけ違う・数字や記号だけ違うパスワードの他の項目（同じ形）
    Similar {
        group: u32,
        others: Vec<String>,
        total: u32,
    },
    /// 強さ（terrible / weak / fair）
    Weak { strength: String },
    /// 2段階認証の説明ページ
    Nofa { doc: Option<String> },
    /// さわっていない日数
    Stale { days: i64 },
    /// 同じサイト・同じユーザー名の別の項目
    Dup { other: String, other_id: String },
}

impl Issue {
    /// 使い回し・似ているのまとまり番号
    pub fn group(&self) -> Option<u32> {
        match self {
            Issue::Reuse { group, .. } | Issue::Similar { group, .. } => Some(*group),
            _ => None,
        }
    }
    pub fn kind(&self) -> Kind {
        match self {
            Issue::Leak { .. } => Kind::Leak,
            Issue::Breach { .. } => Kind::Breach,
            Issue::Reuse { .. } => Kind::Reuse,
            Issue::Similar { .. } => Kind::Similar,
            Issue::Weak { .. } => Kind::Weak,
            Issue::Nofa { .. } => Kind::Nofa,
            Issue::Stale { .. } => Kind::Stale,
            Issue::Dup { .. } => Kind::Dup,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deletion {
    pub url: Option<String>,
    /// easy / medium / hard / impossible / limited
    pub difficulty: String,
    pub email: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemReport {
    pub id: String,
    pub title: String,
    pub vault: String,
    pub domain: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<String>,
    pub days_since_update: Option<i64>,
    pub issues: Vec<Issue>,
    pub risk: f64,
    pub has_totp: bool,
    /// パスキー対応サイトか（Passkeys Directory）
    pub passkey_site: bool,
    pub twofa_doc: Option<String>,
    pub deletion: Option<Deletion>,
    /// ユーザー名の伏せ字（先頭2文字だけ）。例 "ta•••"
    pub username_hint: Option<String>,
    /// サイトのパスワードの決まり（Apple の公開データ）。例 "minlength: 8; maxlength: 20;"
    #[serde(default)]
    pub pw_rule: Option<String>,
}

impl ItemReport {
    pub fn main_kind(&self) -> Option<Kind> {
        self.issues.iter().map(|i| i.kind()).min_by(|a, b| {
            b.severity()
                .partial_cmp(&a.severity())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
    pub fn has(&self, k: Kind) -> bool {
        self.issues.iter().any(|i| i.kind() == k)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SourcesUsed {
    pub pwned_passwords: bool,
    pub breaches: bool,
    pub twofa: bool,
    pub passkeys: bool,
    pub justdeleteme: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// RFC3339
    pub scanned_at: String,
    pub total: usize,
    pub score: u32,
    pub counts: BTreeMap<Kind, usize>,
    pub items: Vec<ItemReport>,
    pub sources: SourcesUsed,
    /// 画面で知らせる注意（コード）。例 "offline", "hibp_failed"
    pub warnings: Vec<String>,
    /// メールアドレスの漏洩チェック（キーがあるときだけ）
    #[serde(default)]
    pub email_breaches: Vec<EmailBreach>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmailBreach {
    /// 伏せ字にしたメールアドレス
    pub masked: String,
    pub breaches: Vec<String>,
}

impl Report {
    pub fn flagged(&self) -> impl Iterator<Item = &ItemReport> {
        self.items.iter().filter(|i| !i.issues.is_empty())
    }
    pub fn flagged_count(&self) -> usize {
        self.flagged().count()
    }
    pub fn recompute(&mut self) {
        for it in &mut self.items {
            it.risk = crate::audit::risk_of(&it.issues);
        }
        self.items.sort_by(|a, b| {
            b.risk
                .partial_cmp(&a.risk)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        });
        self.total = self.items.len();
        self.counts = Kind::ALL
            .into_iter()
            .map(|k| (k, self.items.iter().filter(|i| i.has(k)).count()))
            .collect();
        self.score = crate::audit::score_of(&self.items);
    }
}
