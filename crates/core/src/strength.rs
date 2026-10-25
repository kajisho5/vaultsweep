//! パスワードの強さの目安。1Password が強さを付けていればそちらを優先する。

/// terrible / weak / fair / good / strong
pub fn level_from_1p(s: &str) -> Option<&'static str> {
    Some(match s.to_ascii_uppercase().as_str() {
        "TERRIBLE" => "terrible",
        "WEAK" => "weak",
        "FAIR" => "fair",
        "GOOD" => "good",
        "VERY_GOOD" | "EXCELLENT" | "FANTASTIC" => "strong",
        _ => return None,
    })
}

const COMMON: &[&str] = &[
    "password",
    "123456",
    "12345678",
    "123456789",
    "qwerty",
    "abc123",
    "111111",
    "letmein",
    "iloveyou",
    "admin",
    "welcome",
    "monkey",
    "dragon",
    "baseball",
    "football",
    "sunshine",
    "princess",
    "passw0rd",
    "master",
    "login",
    "starwars",
    "whatever",
    "trustno1",
    "1q2w3e4r",
    "asdfgh",
    "zxcvbn",
];

/// 1Password の値がないときの簡易判定（長さ・文字の種類・よくある語・くり返し）
pub fn estimate(pw: &str) -> &'static str {
    let len = pw.chars().count();
    if len == 0 {
        return "terrible";
    }
    let lower = pw.to_lowercase();
    if COMMON.iter().any(|c| lower.contains(c)) && len < 14 {
        return "terrible";
    }
    let mut pool = 0u32;
    if pw.chars().any(|c| c.is_ascii_lowercase()) {
        pool += 26;
    }
    if pw.chars().any(|c| c.is_ascii_uppercase()) {
        pool += 26;
    }
    if pw.chars().any(|c| c.is_ascii_digit()) {
        pool += 10;
    }
    if pw.chars().any(|c| c.is_ascii_punctuation() || c == ' ') {
        pool += 33;
    }
    if !pw.is_ascii() {
        pool += 100;
    }
    let distinct = {
        let mut v: Vec<char> = pw.chars().collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    // くり返しが多いと実質の長さを減らす
    let eff_len = (len as f64).min(distinct as f64 * 2.0);
    let bits = eff_len * (pool.max(1) as f64).log2();
    if bits < 28.0 {
        "terrible"
    } else if bits < 40.0 {
        "weak"
    } else if bits < 60.0 {
        "fair"
    } else if bits < 80.0 {
        "good"
    } else {
        "strong"
    }
}

/// 設定の基準（"weak" なら terrible/weak、"fair" なら fair まで）以下か
pub fn is_weak(level: &str, threshold: &str) -> bool {
    let rank = |s: &str| match s {
        "terrible" => 0,
        "weak" => 1,
        "fair" => 2,
        "good" => 3,
        _ => 4,
    };
    rank(level) <= rank(if threshold == "fair" { "fair" } else { "weak" })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn levels() {
        assert_eq!(estimate("password1"), "terrible");
        assert_eq!(estimate("aaaaaaaaaaaa"), "terrible");
        assert_eq!(estimate("abcdefgh12"), "fair");
        assert_eq!(estimate("q8$Lz!vR2#pW9@tM6&kN"), "strong");
        assert!(is_weak("weak", "weak"));
        assert!(!is_weak("fair", "weak"));
        assert!(is_weak("fair", "fair"));
        assert_eq!(level_from_1p("FANTASTIC"), Some("strong"));
    }
}
