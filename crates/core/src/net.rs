//! 外部への問い合わせ。決めた相手（下の ALLOWED）にしか送らない。

pub const ALLOWED: &[&str] = &[
    "api.pwnedpasswords.com",
    "haveibeenpwned.com",
    "api.2fa.directory",
    "passkeys-api.2fa.directory",
    "raw.githubusercontent.com",
];

pub trait Fetcher: Send + Sync {
    /// (状態コード, 本文)
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<(u16, Vec<u8>), String>;
}

pub struct HttpFetcher {
    client: reqwest::blocking::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("vaultsweep/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            // 転送先も許可した相手だけ
            .redirect(reqwest::redirect::Policy::custom(|a| {
                if a.previous().len() < 5 && allowed(a.url().as_str()) {
                    a.follow()
                } else {
                    a.stop()
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(HttpFetcher { client })
    }
}

pub fn allowed(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .filter(|u| u.scheme() == "https")
        .and_then(|u| u.host_str().map(|h| ALLOWED.contains(&h)))
        .unwrap_or(false)
}

impl Fetcher for HttpFetcher {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<(u16, Vec<u8>), String> {
        if !allowed(url) {
            return Err(format!("blocked host: {url}"));
        }
        let mut rb = self.client.get(url);
        for (k, v) in headers {
            rb = rb.header(*k, *v);
        }
        let r = rb.send().map_err(|e| e.to_string())?;
        let status = r.status().as_u16();
        let body = r.bytes().map_err(|e| e.to_string())?.to_vec();
        Ok((status, body))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn allow_list() {
        assert!(super::allowed("https://api.pwnedpasswords.com/range/ABCDE"));
        assert!(!super::allowed("http://api.pwnedpasswords.com/range/ABCDE"));
        assert!(!super::allowed("https://evil.example/range"));
    }
}
