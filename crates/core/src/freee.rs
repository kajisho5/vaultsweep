//! freee の明細（CSV）と照合して「払っているサービス」を見つける。
//! 明細はパソコンの外に出さない。保存もしない。

use crate::domain::site_key;
use crate::model::{Kind, Report};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayMatch {
    pub id: String,
    pub title: String,
    pub rows: usize,
    pub total: i64,
    pub last_date: Option<String>,
    /// 1Password 側は「使ってなさそう」
    pub stale: bool,
}

/// CSV を行ごと・列ごとに分ける（"" の中のカンマと改行に対応）
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let text = text.trim_start_matches('\u{feff}');
    let mut rows = vec![];
    let mut row = vec![];
    let mut cell = String::new();
    let mut q = false;
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if q {
            if c == '"' {
                if it.peek() == Some(&'"') {
                    cell.push('"');
                    it.next();
                } else {
                    q = false;
                }
            } else {
                cell.push(c);
            }
        } else {
            match c {
                '"' => q = true,
                ',' => row.push(std::mem::take(&mut cell)),
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                _ => cell.push(c),
            }
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    rows
}

fn find_col(head: &[String], keys: &[&str]) -> Vec<usize> {
    head.iter()
        .enumerate()
        .filter(|(_, h)| {
            let h = h.to_lowercase();
            keys.iter().any(|k| h.contains(k))
        })
        .map(|(i, _)| i)
        .collect()
}

fn amount(s: &str) -> Option<i64> {
    let t: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    t.parse().ok()
}

/// 照合に使う言葉（3文字以上）
fn tokens(title: &str, domain: Option<&str>) -> Vec<String> {
    let mut v = vec![];
    let t = title.trim().to_lowercase();
    if t.chars().count() >= 3 {
        v.push(t);
    }
    if let Some(d) = domain {
        let k = site_key(d);
        if let Some(first) = k.split('.').next()
            && first.len() >= 3
        {
            v.push(first.to_string());
        }
    }
    v
}

pub fn match_csv(text: &str, report: &Report) -> Result<Vec<PayMatch>, String> {
    let rows = parse_csv(text);
    if rows.len() < 2 {
        return Err("CSV に明細がありません".into());
    }
    let head = &rows[0];
    let text_cols = find_col(
        head,
        &[
            "取引先",
            "品目",
            "備考",
            "摘要",
            "内容",
            "明細",
            "description",
            "partner",
            "memo",
        ],
    );
    let amt_cols = find_col(head, &["金額", "amount", "支出"]);
    let date_cols = find_col(head, &["日付", "発生日", "取引日", "date"]);
    let text_cols: Vec<usize> = if text_cols.is_empty() {
        (0..head.len()).collect()
    } else {
        text_cols
    };
    let mut out: Vec<PayMatch> = vec![];
    for it in &report.items {
        let toks = tokens(&it.title, it.domain.as_deref());
        if toks.is_empty() {
            continue;
        }
        let mut m = PayMatch {
            id: it.id.clone(),
            title: it.title.clone(),
            rows: 0,
            total: 0,
            last_date: None,
            stale: it.has(Kind::Stale),
        };
        for r in &rows[1..] {
            let hay: String = text_cols
                .iter()
                .filter_map(|&i| r.get(i))
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            if toks.iter().any(|t| hay.contains(t.as_str())) {
                m.rows += 1;
                if let Some(a) = amt_cols
                    .iter()
                    .filter_map(|&i| r.get(i))
                    .find_map(|s| amount(s))
                {
                    m.total += a.abs();
                }
                if let Some(d) = date_cols
                    .iter()
                    .filter_map(|&i| r.get(i))
                    .find(|s| !s.trim().is_empty())
                    && m.last_date
                        .as_deref()
                        .map(|x| d.as_str() > x)
                        .unwrap_or(true)
                {
                    m.last_date = Some(d.trim().to_string());
                }
            }
        }
        if m.rows > 0 {
            out.push(m);
        }
    }
    out.sort_by(|a, b| b.stale.cmp(&a.stale).then(b.total.cmp(&a.total)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv() {
        let r = parse_csv("\u{feff}a,\"b,c\",\"d\"\"e\"\r\n1,2,3\n");
        assert_eq!(r, vec![vec!["a", "b,c", "d\"e"], vec!["1", "2", "3"]]);
    }
}
