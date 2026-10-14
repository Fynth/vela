use std::{net::SocketAddr, path::PathBuf};
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
    pub cookie_secure: bool,
    pub max_file_bytes: u64,
    pub max_storage_bytes: u64,
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = Self {
            bind: std::env::var("VELA_BIND")
                .unwrap_or_else(|_| "127.0.0.1:3210".into())
                .parse()?,
            data_dir: PathBuf::from(
                std::env::var("VELA_DATA_DIR").unwrap_or_else(|_| "data".into()),
            ),
            web_dir: PathBuf::from(std::env::var("VELA_WEB_DIR").unwrap_or_else(|_| "dist".into())),
            cookie_secure: std::env::var("VELA_COOKIE_SECURE")
                .unwrap_or_else(|_| "false".into())
                .parse()?,
            max_file_bytes: std::env::var("VELA_MAX_FILE_BYTES")
                .unwrap_or_else(|_| "262144000".into())
                .parse()?,
            max_storage_bytes: std::env::var("VELA_MAX_STORAGE_BYTES")
                .unwrap_or_else(|_| "10737418240".into())
                .parse()?,
        };
        anyhow::ensure!(
            config.max_file_bytes > 0 && config.max_file_bytes <= 4 * 1024 * 1024 * 1024,
            "file limit must be in 1..=4 GiB"
        );
        anyhow::ensure!(
            config.max_storage_bytes >= config.max_file_bytes,
            "storage budget must accommodate at least one file"
        );
        Ok(config)
    }
}
