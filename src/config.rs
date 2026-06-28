use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub max_results: usize,
    /// Requests per IP per reset_interval_ms before rate-limiting kicks in
    pub limit: usize,
    pub reset_interval_ms: u64,
    pub latest_version: String,
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            max_results: 5000,
            limit: 20000,
            reset_interval_ms: 300_000,
            latest_version: "1.4".to_string(),
            data_dir: PathBuf::from("data"),
        }
    }
}
