//! Configuration loading for pkg-rs (TOML).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();
static CONFIG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static OVERRIDE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub path: Option<PathBuf>,
    pub reason: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{}: {}", path.display(), self.reason)
        } else {
            write!(f, "{}", self.reason)
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub repos: RepoConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Repository roots to scan (ordered).
    #[serde(default)]
    pub paths: Vec<PathBuf>,
    /// Local install repository (build --process local).
    pub local_path: Option<PathBuf>,
    /// Release install repository (build --process central).
    pub release_path: Option<PathBuf>,
    /// Include user packages in scans.
    pub user_packages: Option<bool>,
    /// Optional scan depth limit.
    pub scan_depth: Option<usize>,
}

pub fn init(override_path: Option<PathBuf>) -> Result<&'static Config, ConfigError> {
    let _ = OVERRIDE_PATH.set(override_path);
    load_config()
}

pub fn get() -> Result<&'static Config, ConfigError> {
    if CONFIG.get().is_some() {
        return Ok(CONFIG.get().unwrap());
    }
    load_config()
}

pub fn config_path() -> Option<&'static Path> {
    CONFIG_PATH
        .get()
        .and_then(|p| p.as_ref().map(|p| p.as_path()))
}

fn load_config() -> Result<&'static Config, ConfigError> {
    let override_path = OVERRIDE_PATH.get().and_then(|p| p.clone());
    let path = resolve_config_path(override_path.as_ref())?;
    let (path, config) = match &path {
        Some(p) => (Some(p.clone()), load_from_path(p)?),
        None => {
            let (created_path, created_config) = create_default_config()?;
            (Some(created_path), created_config)
        }
    };

    let _ = CONFIG_PATH.set(path);
    let _ = CONFIG.set(config);
    Ok(CONFIG.get().unwrap())
}

fn resolve_config_path(override_path: Option<&PathBuf>) -> Result<Option<PathBuf>, ConfigError> {
    if let Some(path) = override_path {
        return ensure_exists(path).map(Some);
    }

    if let Ok(raw) = std::env::var("PKG_RS_CONFIG") {
        let path = PathBuf::from(raw);
        return ensure_exists(&path).map(Some);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("pkg-rs.toml");
            if candidate.exists() {
                return Ok(Some(candidate));
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".pkg-rs").join("pkg-rs.toml");
        if candidate.exists() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

fn ensure_exists(path: &Path) -> Result<PathBuf, ConfigError> {
    if path.exists() {
        Ok(path.to_path_buf())
    } else {
        Err(ConfigError {
            path: Some(path.to_path_buf()),
            reason: "config file not found".to_string(),
        })
    }
}

fn load_from_path(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError {
        path: Some(path.to_path_buf()),
        reason: e.to_string(),
    })?;
    toml::from_str(&content).map_err(|e| ConfigError {
        path: Some(path.to_path_buf()),
        reason: e.to_string(),
    })
}

fn create_default_config() -> Result<(PathBuf, Config), ConfigError> {
    let path = default_config_path().ok_or_else(|| ConfigError {
        path: None,
        reason: "unable to determine default config path".to_string(),
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError {
            path: Some(path.clone()),
            reason: e.to_string(),
        })?;
    }

    if !path.exists() {
        std::fs::write(&path, default_config_template()).map_err(|e| ConfigError {
            path: Some(path.clone()),
            reason: e.to_string(),
        })?;
    }

    let config = load_from_path(&path)?;
    Ok((path, config))
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        return Some(home.join(".pkg-rs").join("pkg-rs.toml"));
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("pkg-rs.toml")))
}

fn default_config_template() -> String {
    let mut out = String::new();
    out.push_str("# pkg-rs configuration\n");
    out.push_str("[repos]\n");
    out.push_str("paths = []\n");
    out.push_str("# local_path = \"D:/packages-local\"\n");
    out.push_str("# release_path = \"//server/packages\"\n");
    out.push_str("# user_packages = true\n");
    out.push_str("# scan_depth = 4\n");
    out
}

pub fn repo_scan_paths(config: &Config) -> Vec<PathBuf> {
    let mut out = Vec::new();

    for path in &config.repos.paths {
        if !out.contains(path) {
            out.push(path.clone());
        }
    }

    if let Some(path) = &config.repos.local_path {
        if !out.contains(path) {
            out.push(path.clone());
        }
    }

    if let Some(path) = &config.repos.release_path {
        if !out.contains(path) {
            out.push(path.clone());
        }
    }

    out
}
