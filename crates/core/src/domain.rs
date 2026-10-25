//! URL からサイトのドメインを取り出す。

/// "https://www.Shop-A.example/login" → "shop-a.example"
pub fn host_of(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let with_scheme = if t.contains("://") {
        t.to_string()
    } else {
        format!("https://{t}")
    };
    let u = url::Url::parse(&with_scheme).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    let h = u.host_str()?.trim_end_matches('.').to_lowercase();
    if h.is_empty() || !h.contains('.') {
        return None;
    }
    Some(h.strip_prefix("www.").unwrap_or(&h).to_string())
}

/// "a.b.example.com" → ["a.b.example.com", "b.example.com", "example.com"]
pub fn candidates(host: &str) -> Vec<String> {
    let labels: Vec<&str> = host.split('.').collect();
    let mut v = vec![];
    for i in 0..labels.len().saturating_sub(1) {
        v.push(labels[i..].join("."));
    }
    v
}

/// まとまりとしてのサイト（重複の判定用）。簡易に末尾2つ（co.jp などは3つ）
pub fn site_key(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').collect();
    let n = labels.len();
    if n <= 2 {
        return host.to_string();
    }
    let second = labels[n - 2];
    let take = if labels[n - 1].len() == 2
        && matches!(
            second,
            "co" | "ne" | "or" | "ac" | "go" | "com" | "net" | "org" | "gr" | "ed" | "lg"
        ) {
        3
    } else {
        2
    };
    labels[n - take.min(n)..].join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hosts() {
        assert_eq!(
            host_of("https://www.Shop-A.example/login").as_deref(),
            Some("shop-a.example")
        );
        assert_eq!(host_of("shop-a.example").as_deref(), Some("shop-a.example"));
        assert_eq!(host_of("android://abc"), None);
        assert_eq!(host_of("localhost"), None);
        assert_eq!(
            candidates("a.b.example.com"),
            vec!["a.b.example.com", "b.example.com", "example.com"]
        );
        assert_eq!(site_key("login.shop.example.co.jp"), "example.co.jp");
        assert_eq!(site_key("accounts.example.com"), "example.com");
    }
}
