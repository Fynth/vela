use std::{
    collections::HashSet,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRootConfig {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
    pub cookie_secure: bool,
    pub max_file_bytes: u64,
    pub max_storage_bytes: u64,
    pub scan_roots: Vec<ScanRootConfig>,
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let data_dir =
            PathBuf::from(std::env::var("VELA_DATA_DIR").unwrap_or_else(|_| "data".into()));
        let scan_roots = match std::env::var("VELA_SCAN_ROOTS") {
            Ok(value) => {
                anyhow::ensure!(
                    value.len() <= 32 * 1024,
                    "scan root configuration exceeds size limit"
                );
                let roots: Vec<ScanRootConfig> = serde_json::from_str(&value)?;
                anyhow::ensure!(
                    roots.iter().all(|r| r.path.is_absolute()),
                    "configured scan roots must be absolute paths"
                );
                roots
            }
            Err(std::env::VarError::NotPresent) => vec![ScanRootConfig {
                id: "music".into(),
                name: "Music folder".into(),
                path: data_dir.join("sources"),
            }],
            Err(error) => return Err(error.into()),
        }
        .into_iter()
        .map(|mut root| {
            root.path = absolute(&root.path)?;
            Ok(root)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
        let config = Self {
            bind: std::env::var("VELA_BIND")
                .unwrap_or_else(|_| "127.0.0.1:3210".into())
                .parse()?,
            data_dir,
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
            scan_roots,
        };
        anyhow::ensure!(
            config.max_file_bytes > 0 && config.max_file_bytes <= 4 * 1024 * 1024 * 1024,
            "file limit must be in 1..=4 GiB"
        );
        anyhow::ensure!(
            config.max_storage_bytes >= config.max_file_bytes,
            "storage budget must accommodate at least one file"
        );
        config.validate_scan_roots()?;
        Ok(config)
    }
    pub fn validate_scan_roots(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.scan_roots.len() <= 32,
            "at most32 scan roots are supported"
        );
        let data = absolute(&self.data_dir)?;
        let default = data.join("sources");
        let mut ids = HashSet::new();
        let mut paths = Vec::<PathBuf>::new();
        for root in &self.scan_roots {
            anyhow::ensure!(
                (1..=48).contains(&root.id.len())
                    && root
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
                "invalid scan root identifier"
            );
            anyhow::ensure!(ids.insert(&root.id), "duplicate scan root identifier");
            anyhow::ensure!(
                !root.name.trim().is_empty()
                    && root.name.len() <= 120
                    && !root.name.chars().any(char::is_control),
                "invalid scan root name"
            );
            let path = absolute(&root.path)?;
            anyhow::ensure!(path.parent().is_some(), "filesystem root cannot be scanned");
            anyhow::ensure!(
                !["/proc", "/sys", "/dev"]
                    .iter()
                    .any(|p| path.starts_with(p)),
                "system pseudo-filesystems cannot be scanned"
            );
            anyhow::ensure!(
                path == default || (!path.starts_with(&data) && !data.starts_with(&path)),
                "scan root overlaps managed application data"
            );
            anyhow::ensure!(
                !paths
                    .iter()
                    .any(|p| path.starts_with(p) || p.starts_with(&path)),
                "scan roots must not overlap"
            );
            paths.push(path);
        }
        Ok(())
    }
    pub fn default_source_path(&self) -> anyhow::Result<PathBuf> {
        Ok(absolute(&self.data_dir)?.join("sources"))
    }
    /// Detect configured aliases into managed storage after its directories exist.
    pub async fn validate_scan_aliases(&self) -> anyhow::Result<()> {
        let data = tokio::fs::canonicalize(&self.data_dir).await?;
        let default = data.join("sources");
        let mut roots = Vec::<PathBuf>::new();
        for root in &self.scan_roots {
            let path = match tokio::fs::canonicalize(&root.path).await {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            anyhow::ensure!(
                path == default || (!path.starts_with(&data) && !data.starts_with(&path)),
                "scan root alias overlaps managed application data"
            );
            anyhow::ensure!(
                path.parent().is_some()
                    && !["/proc", "/sys", "/dev"]
                        .iter()
                        .any(|p| path.starts_with(p)),
                "scan root alias reaches a forbidden filesystem"
            );
            anyhow::ensure!(
                !roots
                    .iter()
                    .any(|p| path.starts_with(p) || p.starts_with(&path)),
                "scan root aliases overlap"
            );
            roots.push(path);
        }
        Ok(())
    }
}
fn absolute(path: &Path) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "parent traversal is not permitted in scan storage configuration"
    );
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(path
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(path: &Path) -> Config {
        Config {
            bind: "127.0.0.1:0".parse().unwrap(),
            data_dir: path.join("data"),
            web_dir: path.join("web"),
            cookie_secure: false,
            max_file_bytes: 1024,
            max_storage_bytes: 4096,
            scan_roots: Vec::new(),
        }
    }
    #[test]
    fn scan_configuration_rejects_managed_system_and_overlapping_roots() {
        let directory = tempfile::tempdir().unwrap();
        let mut c = config(directory.path());
        let root = |path: PathBuf| ScanRootConfig {
            id: "music".into(),
            name: "Music folder".into(),
            path,
        };
        for path in [
            PathBuf::from("/"),
            PathBuf::from("/proc/music"),
            PathBuf::from("/sys/music"),
            PathBuf::from("/dev/music"),
            c.data_dir.clone(),
            c.data_dir.join("media"),
            directory.path().to_owned(),
            directory.path().join("../music"),
        ] {
            c.scan_roots = vec![root(path)];
            assert!(c.validate_scan_roots().is_err());
        }
        c.scan_roots = vec![root(c.default_source_path().unwrap())];
        assert!(c.validate_scan_roots().is_ok());
        let music = directory.path().join("music");
        c.scan_roots = vec![
            root(music.clone()),
            ScanRootConfig {
                id: "nested".into(),
                name: "Nested".into(),
                path: music.join("child"),
            },
        ];
        assert!(c.validate_scan_roots().is_err());
        c.scan_roots = vec![
            root(music.clone()),
            ScanRootConfig {
                id: "music".into(),
                name: "Another".into(),
                path: directory.path().join("other"),
            },
        ];
        assert!(c.validate_scan_roots().is_err());
        c.scan_roots = vec![root(music)];
        assert!(c.validate_scan_roots().is_ok());
        assert!(absolute(Path::new("data/sources")).unwrap().is_absolute());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn scan_aliases_cannot_expose_managed_storage() {
        let directory = tempfile::tempdir().unwrap();
        let mut c = config(directory.path());
        tokio::fs::create_dir_all(c.data_dir.join("media"))
            .await
            .unwrap();
        let alias = directory.path().join("allowed-looking");
        std::os::unix::fs::symlink(c.data_dir.join("media"), &alias).unwrap();
        c.scan_roots = vec![ScanRootConfig {
            id: "alias".into(),
            name: "Alias".into(),
            path: alias,
        }];
        assert!(c.validate_scan_roots().is_ok());
        assert!(c.validate_scan_aliases().await.is_err());
    }
}
