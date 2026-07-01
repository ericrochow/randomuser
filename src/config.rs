use std::path::PathBuf;
use std::time::Duration;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub max_results: usize,
    /// Max requests per IP per rate_window before a 429 is returned.
    pub rate_limit: u64,
    pub rate_window: Duration,
    pub latest_version: &'static str,
    pub data_dir: PathBuf,
    /// If set, request stats are written to this MongoDB instance.
    pub mongodb_uri: Option<String>,
    /// When true, extract the real client IP from X-Forwarded-For / X-Real-IP
    /// instead of the TCP peer address. Enable only when the server sits behind
    /// a trusted reverse proxy; set TRUSTED_PROXY=1.
    pub trusted_proxy: bool,
    /// Public base URL for this deployment (e.g. `https://api.example.com`).
    /// Used in the OpenAPI spec contact link and server entry; set BASE_URL.
    pub base_url: Option<String>,
    /// Display name for the API title and contact in the OpenAPI spec; set SITE_NAME.
    pub site_name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            max_results: 5000,
            rate_limit: 20_000,
            rate_window: Duration::from_secs(300),
            latest_version: "1.4",
            data_dir: PathBuf::from("data"),
            mongodb_uri: None,
            trusted_proxy: false,
            base_url: None,
            site_name: None,
        }
    }
}

impl Config {
    /// Build config from environment variables, falling back to defaults.
    /// Invalid values are logged as warnings and the default is retained.
    ///
    /// | Variable          | Field          | Example                    |
    /// |-------------------|----------------|----------------------------|
    /// | PORT              | port           | 3000                       |
    /// | DATA_DIR          | data_dir       | /opt/randomuser/data       |
    /// | MAX_RESULTS       | max_results    | 5000                       |
    /// | RATE_LIMIT        | rate_limit     | 20000                      |
    /// | RATE_WINDOW_SECS  | rate_window    | 300                        |
    /// | MONGODB_URI       | mongodb_uri    | mongodb://localhost         |
    /// | TRUSTED_PROXY     | trusted_proxy  | 1                          |
    /// | BASE_URL          | base_url       | https://api.example.com    |
    /// | SITE_NAME         | site_name      | My Random User API         |
    pub fn from_env() -> Self {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// Like `from_env` but reads variables via the supplied closure instead of
    /// the process environment. This allows tests to inject arbitrary values
    /// without touching global state.
    pub fn from_env_with(env: impl Fn(&str) -> Option<String>) -> Self {
        let mut c = Self::default();

        if let Some(v) = env("PORT") {
            match v.parse() {
                Ok(n) => c.port = n,
                Err(_) => warn!("PORT={v:?} is not a valid u16; using default {}", c.port),
            }
        }
        if let Some(v) = env("DATA_DIR") {
            c.data_dir = PathBuf::from(v);
        }
        if let Some(v) = env("MAX_RESULTS") {
            match v.parse() {
                Ok(n) => c.max_results = n,
                Err(_) => warn!(
                    "MAX_RESULTS={v:?} is not a valid usize; using default {}",
                    c.max_results
                ),
            }
        }
        if let Some(v) = env("RATE_LIMIT") {
            match v.parse() {
                Ok(n) => c.rate_limit = n,
                Err(_) => warn!(
                    "RATE_LIMIT={v:?} is not a valid u64; using default {}",
                    c.rate_limit
                ),
            }
        }
        if let Some(v) = env("RATE_WINDOW_SECS") {
            match v.parse::<u64>() {
                Ok(n) => c.rate_window = Duration::from_secs(n),
                Err(_) => warn!(
                    "RATE_WINDOW_SECS={v:?} is not a valid u64; using default {:?}",
                    c.rate_window
                ),
            }
        }
        if let Some(v) = env("MONGODB_URI") {
            if !v.is_empty() {
                c.mongodb_uri = Some(v);
            }
        }
        if let Some(v) = env("TRUSTED_PROXY") {
            c.trusted_proxy = matches!(v.trim(), "1" | "true" | "yes");
        }
        if let Some(v) = env("BASE_URL") {
            if !v.is_empty() {
                c.base_url = Some(v.trim_end_matches('/').to_string());
            }
        }
        if let Some(v) = env("SITE_NAME") {
            if !v.is_empty() {
                c.site_name = Some(v);
            }
        }

        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_map<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, &str> = pairs.iter().copied().collect();
        move |k| map.get(k).map(|v| v.to_string())
    }

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.port, 3000);
        assert_eq!(c.max_results, 5000);
        assert_eq!(c.rate_limit, 20_000);
        assert_eq!(c.rate_window, Duration::from_secs(300));
        assert!(c.mongodb_uri.is_none());
        assert!(!c.trusted_proxy);
    }

    #[test]
    fn from_env_with_overrides_port() {
        let c = Config::from_env_with(env_map(&[("PORT", "8080")]));
        assert_eq!(c.port, 8080);
    }

    #[test]
    fn from_env_with_mongodb_uri() {
        let c = Config::from_env_with(env_map(&[("MONGODB_URI", "mongodb://localhost:27017")]));
        assert_eq!(c.mongodb_uri.as_deref(), Some("mongodb://localhost:27017"));
    }

    #[test]
    fn from_env_with_empty_mongodb_uri_stays_none() {
        let c = Config::from_env_with(env_map(&[("MONGODB_URI", "")]));
        assert!(c.mongodb_uri.is_none());
    }

    #[test]
    fn from_env_with_trusted_proxy_variants() {
        for val in ["1", "true", "yes"] {
            let c = Config::from_env_with(env_map(&[("TRUSTED_PROXY", val)]));
            assert!(c.trusted_proxy, "TRUSTED_PROXY={val} should be true");
        }
        for val in ["0", "false", "no", ""] {
            let c = Config::from_env_with(env_map(&[("TRUSTED_PROXY", val)]));
            assert!(!c.trusted_proxy, "TRUSTED_PROXY={val} should be false");
        }
    }

    #[test]
    fn from_env_with_invalid_port_uses_default() {
        let c = Config::from_env_with(env_map(&[("PORT", "not_a_number")]));
        assert_eq!(c.port, 3000);
    }

    #[test]
    fn from_env_with_no_vars_returns_defaults() {
        let c = Config::from_env_with(|_| None);
        assert_eq!(c.port, Config::default().port);
        assert_eq!(c.max_results, Config::default().max_results);
    }

    #[test]
    fn from_env_with_base_url() {
        let c = Config::from_env_with(env_map(&[("BASE_URL", "https://api.example.com")]));
        assert_eq!(c.base_url.as_deref(), Some("https://api.example.com"));
    }

    #[test]
    fn from_env_with_base_url_strips_trailing_slash() {
        let c = Config::from_env_with(env_map(&[("BASE_URL", "https://api.example.com/")]));
        assert_eq!(c.base_url.as_deref(), Some("https://api.example.com"));
    }

    #[test]
    fn from_env_with_empty_base_url_stays_none() {
        let c = Config::from_env_with(env_map(&[("BASE_URL", "")]));
        assert!(c.base_url.is_none());
    }

    #[test]
    fn from_env_with_site_name() {
        let c = Config::from_env_with(env_map(&[("SITE_NAME", "My API")]));
        assert_eq!(c.site_name.as_deref(), Some("My API"));
    }

    #[test]
    fn from_env_with_empty_site_name_stays_none() {
        let c = Config::from_env_with(env_map(&[("SITE_NAME", "")]));
        assert!(c.site_name.is_none());
    }
}
