//! Rez-compatible configuration loading (rezconfig.py + overrides).

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::py::ensure_rez_on_sys_path;

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

#[derive(Debug, Clone)]
pub struct Config {
    pub data: JsonValue,
    pub filepaths: Vec<PathBuf>,
    pub sourced_filepaths: Vec<PathBuf>,
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

pub fn repo_scan_paths(config: &Config) -> Vec<PathBuf> {
    packages_path(config)
}

pub fn packages_path(config: &Config) -> Vec<PathBuf> {
    json_to_paths(get_value(&config.data, "packages_path"))
}

pub fn local_packages_path(config: &Config) -> Option<PathBuf> {
    json_to_path(get_value(&config.data, "local_packages_path"))
}

pub fn release_packages_path(config: &Config) -> Option<PathBuf> {
    json_to_path(get_value(&config.data, "release_packages_path"))
}

pub fn get_str(config: &Config, key: &str) -> Option<String> {
    get_value_path(&config.data, key).and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub fn get_bool(config: &Config, key: &str) -> Option<bool> {
    get_value_path(&config.data, key).and_then(|v| v.as_bool())
}

pub fn get_json(config: &Config, key: &str) -> Option<JsonValue> {
    get_value_path(&config.data, key).cloned()
}

pub fn resolver_backend(config: &Config) -> Option<String> {
    get_str(config, "plugins.pkg_rs.resolver_backend")
}

pub fn apply_package_override(
    base: &Config,
    override_value: &toml::Value,
) -> Result<Config, ConfigError> {
    let override_json = toml_to_json(override_value);
    let override_str = serde_json::to_string(&override_json).map_err(|e| ConfigError {
        path: None,
        reason: e.to_string(),
    })?;

    load_rez_config_with_overrides(&base.filepaths, Some(&override_str))
}

fn load_config() -> Result<&'static Config, ConfigError> {
    let override_path = OVERRIDE_PATH.get().and_then(|p| p.clone());
    let config = load_rez_config(override_path.as_deref())?;
    let primary_path = config.sourced_filepaths.last().cloned();

    let _ = CONFIG_PATH.set(primary_path);
    let _ = CONFIG.set(config);
    Ok(CONFIG.get().unwrap())
}

fn load_rez_config(override_path: Option<&Path>) -> Result<Config, ConfigError> {
    let _ = Python::initialize();

    Python::attach(|py| {
        ensure_rez_on_sys_path(py).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;

        let rez_config = py.import("rez.config").map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to import rez.config: {e}"),
        })?;

        let cfg = if let Some(path) = override_path {
            let filepaths = build_rez_filepaths(py, &rez_config, Some(path))?;
            let filepaths_py = PyList::new(py, &filepaths).map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to build config file list: {e}"),
            })?;
            let config_cls = rez_config.getattr("Config").map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to access rez.config.Config: {e}"),
            })?;
            config_cls
                .call1((filepaths_py,))
                .map_err(|e| ConfigError {
                    path: Some(path.to_path_buf()),
                    reason: format!("failed to create rez Config: {e}"),
                })?
        } else {
            rez_config.getattr("config").map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to access rez.config.config: {e}"),
            })?
        };

        let payload_json = config_to_json(py, &cfg)?;
        let (data, filepaths, sourced_filepaths) = parse_config_payload(&payload_json)?;
        Ok(Config {
            data,
            filepaths,
            sourced_filepaths,
        })
    })
}

fn load_rez_config_with_overrides(
    filepaths: &[PathBuf],
    overrides_json: Option<&str>,
) -> Result<Config, ConfigError> {
    let _ = Python::initialize();

    Python::attach(|py| {
        ensure_rez_on_sys_path(py).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;

        let rez_config = py.import("rez.config").map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to import rez.config: {e}"),
        })?;

        let filepaths_vec = filepaths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let filepaths_py = PyList::new(py, &filepaths_vec).map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to build config file list: {e}"),
        })?;

        let config_cls = rez_config.getattr("Config").map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to access rez.config.Config: {e}"),
        })?;

        let cfg = if let Some(raw) = overrides_json {
            let json_mod = py.import("json").map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to import json: {e}"),
            })?;
            let overrides_py = json_mod
                .getattr("loads")
                .map_err(|e| ConfigError {
                    path: None,
                    reason: format!("failed to access json.loads: {e}"),
                })?
                .call1((raw,))
                .map_err(|e| ConfigError {
                    path: None,
                    reason: format!("failed to parse overrides json: {e}"),
                })?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("overrides", overrides_py).map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to set overrides: {e}"),
            })?;

            config_cls
                .call((filepaths_py,), Some(&kwargs))
                .map_err(|e| ConfigError {
                    path: None,
                    reason: format!("failed to create rez Config with overrides: {e}"),
                })?
        } else {
            config_cls
                .call1((filepaths_py,))
                .map_err(|e| ConfigError {
                    path: None,
                    reason: format!("failed to create rez Config: {e}"),
                })?
        };

        let payload_json = config_to_json(py, &cfg)?;
        let (data, filepaths, sourced_filepaths) = parse_config_payload(&payload_json)?;
        Ok(Config {
            data,
            filepaths,
            sourced_filepaths,
        })
    })
}

fn build_rez_filepaths(
    _py: Python<'_>,
    rez_config: &Bound<'_, PyAny>,
    override_path: Option<&Path>,
) -> Result<Vec<String>, ConfigError> {
    let mut filepaths = Vec::new();

    let root_config = rez_config
        .getattr("get_module_root_config")
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to access rez.config.get_module_root_config: {e}"),
        })?
        .call0()
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to resolve rezconfig.py path: {e}"),
        })?
        .extract::<String>()
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to parse rezconfig.py path: {e}"),
        })?;

    filepaths.push(root_config);

    if let Some(path) = override_path {
        filepaths.push(path.to_string_lossy().to_string());
    } else if let Ok(raw) = std::env::var("REZ_CONFIG_FILE") {
        for path in std::env::split_paths(&raw) {
            filepaths.push(path.to_string_lossy().to_string());
        }
    }

    if !home_config_disabled() {
        if let Some(home) = dirs::home_dir() {
            filepaths.push(home.join(".rezconfig").to_string_lossy().to_string());
        }
    }

    Ok(filepaths)
}

fn config_to_json(py: Python<'_>, cfg: &Bound<'_, PyAny>) -> Result<String, ConfigError> {
    let json_mod = py.import("json").map_err(|e| ConfigError {
        path: None,
        reason: format!("failed to import json: {e}"),
    })?;

    let payload = PyDict::new(py);
    payload
        .set_item("data", cfg.getattr("data").map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to access config.data: {e}"),
        })?)
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to set payload.data: {e}"),
        })?;
    payload
        .set_item(
            "filepaths",
            cfg.getattr("filepaths").map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to access config.filepaths: {e}"),
            })?,
        )
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to set payload.filepaths: {e}"),
        })?;
    payload
        .set_item(
            "sourced_filepaths",
            cfg.getattr("sourced_filepaths").map_err(|e| ConfigError {
                path: None,
                reason: format!("failed to access config.sourced_filepaths: {e}"),
            })?,
        )
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to set payload.sourced_filepaths: {e}"),
        })?;

    let dumps = json_mod.getattr("dumps").map_err(|e| ConfigError {
        path: None,
        reason: format!("failed to access json.dumps: {e}"),
    })?;

    let kwargs = PyDict::new(py);
    let builtins = py.import("builtins").map_err(|e| ConfigError {
        path: None,
        reason: format!("failed to import builtins: {e}"),
    })?;
    let default_fn = builtins.getattr("str").map_err(|e| ConfigError {
        path: None,
        reason: format!("failed to resolve str function: {e}"),
    })?;
    kwargs.set_item("default", default_fn).map_err(|e| ConfigError {
        path: None,
        reason: format!("failed to set json default: {e}"),
    })?;

    dumps
        .call((payload,), Some(&kwargs))
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to serialize config: {e}"),
        })?
        .extract::<String>()
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("failed to decode config json: {e}"),
        })
}

fn parse_config_payload(
    payload: &str,
) -> Result<(JsonValue, Vec<PathBuf>, Vec<PathBuf>), ConfigError> {
    let doc: JsonValue = serde_json::from_str(payload).map_err(|e| ConfigError {
        path: None,
        reason: e.to_string(),
    })?;

    let data = doc.get("data").cloned().unwrap_or(JsonValue::Null);
    let filepaths = json_to_paths(doc.get("filepaths"));
    let sourced = json_to_paths(doc.get("sourced_filepaths"));

    Ok((data, filepaths, sourced))
}

fn home_config_disabled() -> bool {
    let value = std::env::var("REZ_DISABLE_HOME_CONFIG")
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(value.as_str(), "1" | "t" | "true")
}

fn get_value<'a>(data: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    data.get(key)
}

fn get_value_path<'a>(data: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut current = data;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn json_to_paths(value: Option<&JsonValue>) -> Vec<PathBuf> {
    let Some(value) = value else {
        return Vec::new();
    };

    match value {
        JsonValue::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(PathBuf::from)
            .collect(),
        JsonValue::String(value) => vec![PathBuf::from(value)],
        _ => Vec::new(),
    }
}

fn json_to_path(value: Option<&JsonValue>) -> Option<PathBuf> {
    match value {
        Some(JsonValue::String(value)) => Some(PathBuf::from(value)),
        _ => None,
    }
}

fn toml_to_json(value: &toml::Value) -> JsonValue {
    match value {
        toml::Value::String(s) => JsonValue::String(s.clone()),
        toml::Value::Integer(v) => JsonValue::Number((*v).into()),
        toml::Value::Float(v) => serde_json::Number::from_f64(*v)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        toml::Value::Boolean(v) => JsonValue::Bool(*v),
        toml::Value::Datetime(v) => JsonValue::String(v.to_string()),
        toml::Value::Array(values) => {
            JsonValue::Array(values.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k.clone(), toml_to_json(v));
            }
            JsonValue::Object(map)
        }
    }
}
