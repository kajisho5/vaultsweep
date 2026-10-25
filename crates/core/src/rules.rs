//! サイトごとのパスワードの決まり。
//! - Apple Password Manager Resources（MIT）の quirks/password-rules.json を読む
//!   書き方の例 "minlength: 8; maxlength: 20; required: lower, upper; required: digit; allowed: [!$*?@|]; max-consecutive: 3;"
//! - 決まりから 1Password CLI の作り方（"letters,digits,symbols,24" など）を決める
//! - 作ったパスワードが決まりに合うかを確かめる（値は呼ぶ側で使ったらすぐ消す）

use crate::domain::candidates;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

pub const RULES_URL: &str = "https://raw.githubusercontent.com/apple/password-manager-resources/main/quirks/password-rules.json";

/// アプリに同梱する控え（取りに行けないとき・オフラインのとき用）
pub const BUNDLED: &[u8] = include_bytes!("../data/password-rules.json");

/// 1Password CLI が使う記号（選べない）。公式の説明の既定の記号
pub const OP_SYMBOLS: &str = "!@.-_*";

/// Password Rules の "special"（ASCII の記号と空白）
const SPECIAL: &str = "-~!@#$%^&*_+=`|(){}[:;\"'<>,.?]/ ";

/// 文字の集まり
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharSet {
    pub lower: bool,
    pub upper: bool,
    pub digit: bool,
    pub symbols: BTreeSet<char>,
    /// "unicode"：何でもよい
    pub any: bool,
}

impl CharSet {
    fn add_class(&mut self, name: &str) -> bool {
        match name {
            "lower" => self.lower = true,
            "upper" => self.upper = true,
            "digit" => self.digit = true,
            "special" => self.symbols.extend(SPECIAL.chars()),
            "ascii-printable" => {
                self.lower = true;
                self.upper = true;
                self.digit = true;
                self.symbols.extend(SPECIAL.chars());
            }
            "unicode" => self.any = true,
            _ => return false,
        }
        true
    }
    fn add_chars(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                'a'..='z' => self.lower = true,
                'A'..='Z' => self.upper = true,
                '0'..='9' => self.digit = true,
                _ => {
                    self.symbols.insert(c);
                }
            }
        }
    }
    fn union(&mut self, o: &CharSet) {
        self.lower |= o.lower;
        self.upper |= o.upper;
        self.digit |= o.digit;
        self.any |= o.any;
        self.symbols.extend(o.symbols.iter().copied());
    }
    pub fn contains(&self, c: char) -> bool {
        self.any
            || match c {
                'a'..='z' => self.lower,
                'A'..='Z' => self.upper,
                '0'..='9' => self.digit,
                _ => self.symbols.contains(&c),
            }
    }
    fn is_empty(&self) -> bool {
        !self.lower && !self.upper && !self.digit && !self.any && self.symbols.is_empty()
    }
}

/// サイトの決まり
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rule {
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub max_consecutive: Option<u32>,
    /// どれも1文字以上入れる
    pub required: Vec<CharSet>,
    /// 使える文字（required も含む）。None なら制限なし
    pub allowed: Option<CharSet>,
}

/// 決まりの文字列を読む。分からない部分は無視する（読めた部分だけで判断する）
pub fn parse(s: &str) -> Rule {
    let mut r = Rule::default();
    let mut allowed: Option<CharSet> = None;
    for part in split_rules(s) {
        let Some((name, value)) = part.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "minlength" => r.min = value.parse().ok(),
            "maxlength" => r.max = value.parse().ok(),
            "max-consecutive" => r.max_consecutive = value.parse().ok(),
            "required" | "allowed" => {
                let cs = parse_classes(value);
                if cs.is_empty() {
                    continue;
                }
                if name == "required" {
                    r.required.push(cs);
                } else {
                    allowed.get_or_insert_with(CharSet::default).union(&cs);
                }
            }
            _ => {}
        }
    }
    if let Some(mut a) = allowed {
        for q in &r.required {
            a.union(q);
        }
        r.allowed = Some(a);
    } else if !r.required.is_empty() {
        // 必ず入れる種類だけ書かれている：使える文字はその合計（Password Rules の決まり）
        let mut a = CharSet::default();
        for q in &r.required {
            a.union(q);
        }
        r.allowed = Some(a);
    }
    r
}

/// ";" で区切る。ただし [ ] の中の ";" は区切りにしない
fn split_rules(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut in_br = false;
    while i < chars.len() {
        let c = chars[i];
        if in_br {
            cur.push(c);
            if c == ']' {
                // "]]" の最初の "]" は文字としての "]"
                if chars.get(i + 1) == Some(&']') {
                    cur.push(']');
                    i += 1;
                }
                in_br = false;
            }
        } else if c == '[' {
            in_br = true;
            cur.push(c);
        } else if c == ';' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
        i += 1;
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// "lower, upper, [-!#]" を読む
fn parse_classes(v: &str) -> CharSet {
    let mut cs = CharSet::default();
    let chars: Vec<char> = v.chars().collect();
    let mut i = 0;
    let mut word = String::new();
    let flush = |w: &mut String, cs: &mut CharSet| {
        let t = w.trim().to_ascii_lowercase();
        if !t.is_empty() {
            cs.add_class(&t);
        }
        w.clear();
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '[' {
            flush(&mut word, &mut cs);
            let mut j = i + 1;
            let mut inner = String::new();
            while j < chars.len() {
                if chars[j] == ']' {
                    if chars.get(j + 1) == Some(&']') {
                        inner.push(']');
                        j += 1;
                    }
                    break;
                }
                inner.push(chars[j]);
                j += 1;
            }
            cs.add_chars(&inner);
            i = j + 1;
            continue;
        }
        if c == ',' {
            flush(&mut word, &mut cs);
        } else {
            word.push(c);
        }
        i += 1;
    }
    flush(&mut word, &mut cs);
    cs
}

/// 作ったパスワードが決まりに合うか
pub fn satisfies(pw: &str, r: &Rule) -> bool {
    let n = pw.chars().count() as u32;
    if r.min.is_some_and(|m| n < m) || r.max.is_some_and(|m| n > m) {
        return false;
    }
    if let Some(a) = &r.allowed
        && !pw.chars().all(|c| a.contains(c))
    {
        return false;
    }
    if !r.required.iter().all(|q| pw.chars().any(|c| q.contains(c))) {
        return false;
    }
    if let Some(mc) = r.max_consecutive
        && mc > 0
    {
        let mut run = 0u32;
        let mut prev = None;
        for c in pw.chars() {
            run = if Some(c) == prev { run + 1 } else { 1 };
            if run > mc {
                return false;
            }
            prev = Some(c);
        }
    }
    true
}

/// 使う文字（自分で決めるとき）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chars {
    /// 英字＋数字＋記号
    All,
    /// 英字＋数字
    Alnum,
    /// 数字だけ
    Digits,
}

/// 本人が決めた、そのサイトの決まり（パソコンの設定に、ドメインと一緒に保存する）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteRule {
    pub length: u32,
    pub chars: Chars,
}

impl SiteRule {
    pub fn sanitized(self) -> SiteRule {
        SiteRule {
            length: self.length.clamp(4, 64),
            chars: self.chars,
        }
    }
}

/// 実際に 1Password に頼む作り方
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub length: u32,
    pub chars: Chars,
    /// どこから決めたか："rule"（Apple の公開データ）/ "site"（本人が決めた）/ "default"（設定どおり）
    pub source: String,
    /// 自動では決まりに合わせられない
    pub manual: bool,
}

impl Plan {
    /// 1Password CLI の作り方の文字列
    pub fn recipe(&self) -> String {
        let n = self.length.clamp(4, 64);
        match self.chars {
            Chars::All => format!("letters,digits,symbols,{n}"),
            Chars::Alnum => format!("letters,digits,{n}"),
            Chars::Digits => format!("digits,{n}"),
        }
    }
}

/// 作り方を決める。本人の決まり → Apple の決まり → 設定、の順に使う
pub fn plan(pw_length: u32, pw_symbols: bool, rule: Option<&Rule>, site: Option<SiteRule>) -> Plan {
    let default_chars = if pw_symbols { Chars::All } else { Chars::Alnum };
    if let Some(s) = site {
        let s = s.sanitized();
        return Plan {
            length: s.length,
            chars: s.chars,
            source: "site".into(),
            manual: false,
        };
    }
    let Some(r) = rule else {
        return Plan {
            length: pw_length.clamp(12, 64),
            chars: default_chars,
            source: "default".into(),
            manual: false,
        };
    };
    let op_sym: BTreeSet<char> = OP_SYMBOLS.chars().collect();
    let allowed = r.allowed.clone();
    let ok = |f: &dyn Fn(&CharSet) -> bool| allowed.as_ref().map(f).unwrap_or(true);
    let letters_ok = ok(&|a| a.any || (a.lower && a.upper));
    let digits_ok = ok(&|a| a.any || a.digit);
    let symbols_ok = ok(&|a| a.any || op_sym.is_subset(&a.symbols));

    let chars = if letters_ok && digits_ok {
        if pw_symbols && symbols_ok {
            Chars::All
        } else {
            Chars::Alnum
        }
    } else if digits_ok && !letters_ok && allowed.as_ref().is_some_and(|a| !a.lower && !a.upper) {
        Chars::Digits
    } else {
        // 英字の片方だけ、などは 1Password では作れない
        return manual_plan(pw_length, r);
    };
    // 必ず入れる種類を、選んだ文字で満たせるか
    let mut have = CharSet::default();
    match chars {
        Chars::All => {
            have.lower = true;
            have.upper = true;
            have.digit = true;
            have.symbols = op_sym.clone();
        }
        Chars::Alnum => {
            have.lower = true;
            have.upper = true;
            have.digit = true;
        }
        Chars::Digits => have.digit = true,
    }
    let mut chars = chars;
    for q in &r.required {
        let hit = (q.lower && have.lower)
            || (q.upper && have.upper)
            || (q.digit && have.digit)
            || q.any
            || q.symbols.iter().any(|c| have.symbols.contains(c));
        if !hit {
            // 記号が必要なのに記号を外していた：使えるなら記号ありに戻す
            if chars == Chars::Alnum && symbols_ok && q.symbols.iter().any(|c| op_sym.contains(c)) {
                chars = Chars::All;
                have.symbols = op_sym.clone();
                continue;
            }
            return manual_plan(pw_length, r);
        }
    }
    let base = if chars == Chars::Digits {
        8
    } else {
        pw_length.clamp(12, 64)
    };
    let mut len = base;
    if let Some(mx) = r.max {
        len = len.min(mx);
    }
    if let Some(mn) = r.min {
        len = len.max(mn);
    }
    if !(4..=64).contains(&len) || r.max.is_some_and(|mx| r.min.is_some_and(|mn| mn > mx)) {
        return manual_plan(pw_length, r);
    }
    Plan {
        length: len,
        chars,
        source: "rule".into(),
        manual: false,
    }
}

fn manual_plan(pw_length: u32, r: &Rule) -> Plan {
    let len = r
        .max
        .map(|m| m.min(pw_length))
        .unwrap_or(pw_length)
        .clamp(4, 64);
    Plan {
        length: len,
        chars: Chars::Alnum,
        source: "rule".into(),
        manual: true,
    }
}

/// 本人が決めた決まりを、確かめ用の Rule にする
pub fn site_rule_as_rule(s: SiteRule) -> Rule {
    let s = s.sanitized();
    let mut a = CharSet::default();
    match s.chars {
        Chars::All => {
            a.lower = true;
            a.upper = true;
            a.digit = true;
            a.symbols = OP_SYMBOLS.chars().collect();
        }
        Chars::Alnum => {
            a.lower = true;
            a.upper = true;
            a.digit = true;
        }
        Chars::Digits => a.digit = true,
    }
    Rule {
        min: Some(s.length),
        max: Some(s.length),
        max_consecutive: None,
        required: vec![],
        allowed: Some(a),
    }
}

/// ドメイン → 決まりの文字列
#[derive(Default, Clone)]
pub struct RuleBook {
    map: HashMap<String, (String, bool)>,
}

impl RuleBook {
    pub fn parse(b: &[u8]) -> Option<RuleBook> {
        #[derive(Deserialize)]
        struct E {
            #[serde(rename = "password-rules")]
            rules: String,
            #[serde(rename = "exact-domain-match-only", default)]
            exact: bool,
        }
        let m: HashMap<String, E> = serde_json::from_slice(b).ok()?;
        Some(RuleBook {
            map: m
                .into_iter()
                .map(|(k, e)| (k.to_lowercase(), (e.rules, e.exact)))
                .collect(),
        })
    }
    pub fn bundled() -> RuleBook {
        RuleBook::parse(BUNDLED).unwrap_or_default()
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// サイトの決まりの文字列（"login.example.com" なら "example.com" の決まりも見る）
    pub fn find(&self, host: &str) -> Option<String> {
        let host = host.to_lowercase();
        if let Some((r, _)) = self.map.get(&host) {
            return Some(r.clone());
        }
        candidates(&host)
            .into_iter()
            .skip(1)
            .find_map(|c| match self.map.get(&c) {
                Some((r, false)) => Some(r.clone()),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_examples() {
        let r = parse(
            "minlength: 8; maxlength: 20; max-consecutive: 3; required: lower, upper; required: digit; allowed: [!$*?@|];",
        );
        assert_eq!(
            (r.min, r.max, r.max_consecutive),
            (Some(8), Some(20), Some(3))
        );
        assert_eq!(r.required.len(), 2);
        let a = r.allowed.as_ref().unwrap();
        assert!(
            a.contains('!')
                && a.contains('|')
                && a.contains('a')
                && a.contains('Z')
                && a.contains('5')
        );
        assert!(!a.contains('.') && !a.contains('-'));
    }

    #[test]
    fn brackets_with_semicolons_and_close_bracket() {
        let r = parse(
            "minlength: 8; allowed: lower, upper, digit, [-!\"#$%&'()*+,./:;<=>?@[\\^_`{|}~]];",
        );
        let a = r.allowed.unwrap();
        assert!(a.contains(';') && a.contains(']') && a.contains('[') && a.contains('-'));
        assert_eq!(r.min, Some(8));
    }

    #[test]
    fn plan_drops_symbols_when_op_symbols_not_allowed() {
        let r = parse(
            "minlength: 8; maxlength: 20; required: lower; required: upper; required: digit; allowed: [!$*?@|];",
        );
        let p = plan(24, true, Some(&r), None);
        assert_eq!(p.chars, Chars::Alnum);
        assert_eq!(p.length, 20);
        assert!(!p.manual);
        assert_eq!(p.recipe(), "letters,digits,20");
    }

    #[test]
    fn plan_keeps_symbols_when_all_allowed() {
        let r = parse("minlength: 8; maxlength: 40; allowed: lower, upper, digit, special;");
        let p = plan(24, true, Some(&r), None);
        assert_eq!(p.chars, Chars::All);
        assert_eq!(p.length, 24);
    }

    #[test]
    fn plan_digits_only() {
        let r = parse("minlength: 6; maxlength: 6; allowed: digit; max-consecutive: 3;");
        let p = plan(24, true, Some(&r), None);
        assert_eq!((p.chars, p.length, p.manual), (Chars::Digits, 6, false));
        assert_eq!(p.recipe(), "digits,6");
    }

    #[test]
    fn plan_manual_when_special_required_but_op_symbols_not_allowed() {
        let r = parse(
            "minlength: 8; maxlength: 16; required: lower; required: upper; required: digit; required: [!#$%];",
        );
        let p = plan(24, true, Some(&r), None);
        assert!(p.manual);
    }

    #[test]
    fn plan_special_required_and_allowed() {
        let r = parse(
            "minlength: 8; maxlength: 16; required: lower; required: upper; required: digit; required: special;",
        );
        let p = plan(24, false, Some(&r), None);
        // 記号が必要なので、設定で記号なしでも記号ありで作る
        assert_eq!(p.chars, Chars::All);
        assert_eq!(p.length, 16);
    }

    #[test]
    fn plan_site_rule_wins() {
        let r = parse("maxlength: 15;");
        let p = plan(
            24,
            true,
            Some(&r),
            Some(SiteRule {
                length: 10,
                chars: Chars::Alnum,
            }),
        );
        assert_eq!(
            (p.length, p.chars, p.source.as_str()),
            (10, Chars::Alnum, "site")
        );
    }

    #[test]
    fn plan_default_without_rule() {
        let p = plan(30, false, None, None);
        assert_eq!(
            (p.length, p.chars, p.source.as_str()),
            (30, Chars::Alnum, "default")
        );
    }

    #[test]
    fn plan_respects_minlength_above_setting() {
        let r = parse("minlength: 30;");
        assert_eq!(plan(24, true, Some(&r), None).length, 30);
    }

    #[test]
    fn satisfies_checks() {
        let r = parse(
            "minlength: 8; maxlength: 12; required: digit; allowed: lower, upper; max-consecutive: 2;",
        );
        assert!(satisfies("abcDEF12", &r));
        assert!(!satisfies("abcDEFgh", &r)); // 数字なし
        assert!(!satisfies("abcDEF1!", &r)); // 使えない記号
        assert!(!satisfies("aaaDEF12", &r)); // 連続3
        assert!(!satisfies("ab1", &r)); // 短い
        assert!(!satisfies("abcdefghijk12", &r)); // 長い
        let any = Rule::default();
        assert!(satisfies("x", &any));
    }

    #[test]
    fn site_rule_check() {
        let r = site_rule_as_rule(SiteRule {
            length: 16,
            chars: Chars::Alnum,
        });
        assert!(satisfies("abcdABCD1234efgh", &r));
        assert!(!satisfies("abcdABCD1234efg!", &r));
        let d = site_rule_as_rule(SiteRule {
            length: 6,
            chars: Chars::Digits,
        });
        assert!(satisfies("123456", &d) && !satisfies("12345a", &d));
    }

    #[test]
    fn rulebook_find() {
        let b = RuleBook::parse(br#"{"bank.example":{"password-rules":"maxlength: 20;"},"only.example":{"password-rules":"maxlength: 8;","exact-domain-match-only":true}}"#).unwrap();
        assert_eq!(
            b.find("login.bank.example").as_deref(),
            Some("maxlength: 20;")
        );
        assert_eq!(b.find("only.example").as_deref(), Some("maxlength: 8;"));
        assert_eq!(b.find("www.only.example"), None);
    }

    #[test]
    fn bundled_data_parses_every_rule() {
        let b = RuleBook::bundled();
        assert!(b.len() > 300);
        for (k, (r, _)) in &b.map {
            let p = parse(r);
            // 読めた決まりで、作り方が決まる（壊れない）こと
            let pl = plan(24, true, Some(&p), None);
            assert!((4..=64).contains(&pl.length), "{k}: {r}");
        }
    }
}
