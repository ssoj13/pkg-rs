//! Rez-compatible configuration loading (rezconfig.py + overrides).

use pyo3::prelude::*;
use pyo3::types::PyDict;
use regex::Regex;
use serde_json::{Map, Value as JsonValue};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs};

static CONFIG: OnceLock<Config> = OnceLock::new();
static CONFIG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static OVERRIDE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

const EMBEDDED_REZCONFIG_PATH: &str = "<embedded:rezconfig.py>";
const DEFAULT_REZCONFIG_SOURCE: &str = include_str!("../config/rezconfig.py");
const MODIFY_LIST_KEY: &str = "__modify_list__";

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
    let mut data = base.data.clone();
    deep_merge(&mut data, override_json);
    normalize_pkg_rs(&mut data);
    expand_config_vars(&mut data);

    Ok(Config {
        data,
        filepaths: base.filepaths.clone(),
        sourced_filepaths: base.sourced_filepaths.clone(),
    })
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
    let mut data = load_default_rezconfig()?;
    let override_paths = resolve_override_paths(override_path);
    ensure_default_rezconfig(override_path)?;
    let mut filepaths = Vec::new();
    filepaths.push(PathBuf::from(EMBEDDED_REZCONFIG_PATH));
    filepaths.extend(override_paths.iter().cloned());

    let mut sourced = vec![PathBuf::from(EMBEDDED_REZCONFIG_PATH)];
    for path in override_paths {
        if let Some((doc, sourced_path)) = load_config_file(&path)? {
            deep_merge(&mut data, doc);
            sourced.push(sourced_path);
        }
    }

    normalize_pkg_rs(&mut data);
    apply_env_overrides(&mut data);
    expand_config_vars(&mut data);

    Ok(Config {
        data,
        filepaths,
        sourced_filepaths: sourced,
    })
}

/// Ordered locations for rezconfig.py (fallback: later in list overrides earlier when merged).
fn resolve_override_paths(override_path: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = override_path {
        return vec![path.to_path_buf()];
    }

    let mut out = env::var("REZ_CONFIG_FILE")
        .or_else(|_| env::var("REZ_CONFIG_PATH"))
        .ok()
        .map(|raw| env::split_paths(&raw).collect::<Vec<_>>())
        .unwrap_or_default();

    if let Some(path) = rezconfig_next_to_exe() {
        out.push(path);
    }
    if let Some(home) = dirs::home_dir() {
        out.push(home.join(".pkg-rs").join("rezconfig.py"));
    }

    out
}

/// Create ~/.pkg-rs/rezconfig.py from embedded default when no other config exists.
fn ensure_default_rezconfig(override_path: Option<&Path>) -> Result<(), ConfigError> {
    if override_path.is_some() {
        return Ok(());
    }
    if env::var("REZ_CONFIG_FILE").is_ok() || env::var("REZ_CONFIG_PATH").is_ok() {
        return Ok(());
    }
    if rezconfig_next_to_exe().is_some() {
        return Ok(());
    }

    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };
    let path = home.join(".pkg-rs").join("rezconfig.py");
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError {
            path: Some(path.clone()),
            reason: e.to_string(),
        })?;
    }

    fs::write(&path, DEFAULT_REZCONFIG_SOURCE).map_err(|e| ConfigError {
        path: Some(path.clone()),
        reason: e.to_string(),
    })?;

    Ok(())
}

fn rezconfig_next_to_exe() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("rezconfig.py");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

fn load_default_rezconfig() -> Result<JsonValue, ConfigError> {
    load_rezconfig_source(EMBEDDED_REZCONFIG_PATH, DEFAULT_REZCONFIG_SOURCE)
}

/// Load only rezconfig.py (no YAML/JSON).
fn load_config_file(path: &Path) -> Result<Option<(JsonValue, PathBuf)>, ConfigError> {
    let candidate = if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("py")) {
        path.to_path_buf()
    } else {
        path.with_extension("py")
    };
    if !candidate.exists() {
        return Ok(None);
    }
    let doc = load_rezconfig_file(&candidate)?;
    Ok(Some((doc, candidate)))
}

fn load_rezconfig_file(path: &Path) -> Result<JsonValue, ConfigError> {
    let source = fs::read_to_string(path).map_err(|e| ConfigError {
        path: Some(path.to_path_buf()),
        reason: e.to_string(),
    })?;
    load_rezconfig_source(&path.to_string_lossy(), &source)
}

fn load_rezconfig_source(name: &str, source: &str) -> Result<JsonValue, ConfigError> {
    let _ = Python::initialize();
    Python::attach(|py| {
        let globals = PyDict::new(py);
        globals
            .set_item("__file__", name)
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig bootstrap failed: {e}"),
            })?;
        globals
            .set_item("__name__", "rezconfig")
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig bootstrap failed: {e}"),
            })?;
        globals
            .set_item("rez_version", crate::VERSION)
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig bootstrap failed: {e}"),
            })?;

        let bootstrap = r#"
class ModifyList(object):
    def __init__(self, append=None, prepend=None):
        self.append = append
        self.prepend = prepend

    def apply(self, v):
        if v is None:
            v = []
        if not isinstance(v, list):
            raise ValueError("ModifyList can only apply to list")
        return (self.prepend or []) + v + (self.append or [])

class DelayLoad:
    def __init__(self, value):
        self._value = value
    def get_value(self):
        return self._value
"#;
        let bootstrap = std::ffi::CString::new(bootstrap).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;
        py.run(bootstrap.as_c_str(), Some(&globals), None).map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig bootstrap failed: {e}"),
        })?;

        let code = std::ffi::CString::new(source).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;
        py.run(code.as_c_str(), Some(&globals), None).map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig exec failed: {e}"),
        })?;

        let helper_code = std::ffi::CString::new(r#"
import json
import inspect

def _is_class(obj):
    return isinstance(obj, type)

def _should_skip(name, obj):
    if name.startswith("__"):
        return True
    if inspect.ismodule(obj) or inspect.isfunction(obj) or inspect.ismethod(obj):
        return True
    return False

def _obj_dict(obj):
    if isinstance(obj, dict):
        return obj
    if _is_class(obj):
        return vars(obj)
    if hasattr(obj, "__dict__"):
        return vars(obj)
    return None

def _normalize(obj, modify_list_cls):
    if modify_list_cls is not None:
        try:
            if isinstance(obj, modify_list_cls):
                return {"__modify_list__": {
                    "prepend": _normalize(getattr(obj, "prepend", None), modify_list_cls),
                    "append": _normalize(getattr(obj, "append", None), modify_list_cls),
                }}
        except Exception:
            pass
    if hasattr(obj, "get_value") and callable(getattr(obj, "get_value")):
        obj = obj.get_value()
    if isinstance(obj, list) or isinstance(obj, tuple) or isinstance(obj, set):
        return [_normalize(x, modify_list_cls) for x in obj]
    if isinstance(obj, dict):
        return {k: _normalize(v, modify_list_cls) for k, v in obj.items()}
    if _is_class(obj) or hasattr(obj, "__dict__"):
        data = {}
        raw = _obj_dict(obj) or {}
        for k, v in raw.items():
            if _should_skip(k, v):
                continue
            data[k] = _normalize(v, modify_list_cls)
        return data
    return obj

def extract_config(globals_dict, modify_list_cls=None):
    result = {}
    reserved = {"__builtins__", "__name__", "__file__", "rez_version", "ModifyList", "DelayLoad"}
    for k, v in globals_dict.items():
        if k in reserved:
            continue
        if _should_skip(k, v):
            continue
        result[k] = _normalize(v, modify_list_cls)
    return result
"#).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;
        let helper_name = std::ffi::CString::new("__pkg_config_helper__.py").map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;
        let helper_module = std::ffi::CString::new("__pkg_config_helper__").map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;
        let helper = PyModule::from_code(
            py,
            helper_code.as_c_str(),
            helper_name.as_c_str(),
            helper_module.as_c_str(),
        )
        .map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig helper failed: {e}"),
        })?;

        let extract = helper
            .getattr("extract_config")
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig helper failed: {e}"),
            })?;
        let modify_list: Py<PyAny> = match globals.get_item("ModifyList") {
            Ok(Some(value)) => value.unbind(),
            Ok(None) => py.None(),
            Err(_) => py.None(),
        };
        let result = extract
            .call1((globals, modify_list))
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig extract failed: {e}"),
            })?;

        let json_mod = py.import("json").map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig json failed: {e}"),
        })?;
        let dumps = json_mod.getattr("dumps").map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig json failed: {e}"),
        })?;
        let kwargs = PyDict::new(py);
        let builtins = py.import("builtins").map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig json failed: {e}"),
        })?;
        let default_fn = builtins.getattr("str").map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig json failed: {e}"),
        })?;
        kwargs.set_item("default", default_fn).map_err(|e| ConfigError {
            path: None,
            reason: format!("rezconfig json failed: {e}"),
        })?;

        let json = dumps
            .call((result,), Some(&kwargs))
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig json failed: {e}"),
            })?
            .extract::<String>()
            .map_err(|e| ConfigError {
                path: None,
                reason: format!("rezconfig json failed: {e}"),
            })?;

        let doc: JsonValue = serde_json::from_str(&json).map_err(|e| ConfigError {
            path: None,
            reason: e.to_string(),
        })?;

        Ok(doc)
    })
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

fn deep_merge(target: &mut JsonValue, patch: JsonValue) {
    match (target, patch) {
        (JsonValue::Object(target_map), JsonValue::Object(patch_map)) => {
            let keys: Vec<String> = target_map.keys().cloned().collect();
            for key in keys {
                if !patch_map.contains_key(&key) {
                    if let Some(value) = target_map.get(&key).cloned() {
                        target_map.insert(key, flatten_value(value));
                    }
                }
            }

            for (key, patch_val) in patch_map {
                let existing = target_map.remove(&key).unwrap_or(JsonValue::Null);
                let merged = merge_values(existing, patch_val);
                target_map.insert(key, merged);
            }
        }
        (target_value, patch_value) => {
            let existing = target_value.clone();
            *target_value = merge_values(existing, patch_value);
        }
    }
}

fn merge_values(existing: JsonValue, patch: JsonValue) -> JsonValue {
    if let Some(mod_list) = modify_list_value(&patch) {
        return apply_modify_list(existing, mod_list);
    }

    match (existing, patch) {
        (JsonValue::Object(existing_map), JsonValue::Object(patch_map)) => {
            let mut merged = JsonValue::Object(existing_map);
            deep_merge(&mut merged, JsonValue::Object(patch_map));
            merged
        }
        (_, other) => flatten_value(other),
    }
}

#[derive(Debug, Clone)]
struct ModifyListValue {
    prepend: Vec<JsonValue>,
    append: Vec<JsonValue>,
}

fn modify_list_value(value: &JsonValue) -> Option<ModifyListValue> {
    let obj = value.as_object()?;
    let payload = obj.get(MODIFY_LIST_KEY)?;
    let payload = payload.as_object()?;

    let prepend = payload
        .get("prepend")
        .map(extract_modify_list_items)
        .unwrap_or_default();
    let append = payload
        .get("append")
        .map(extract_modify_list_items)
        .unwrap_or_default();

    Some(ModifyListValue { prepend, append })
}

fn extract_modify_list_items(value: &JsonValue) -> Vec<JsonValue> {
    match value {
        JsonValue::Array(items) => items.clone(),
        JsonValue::Null => Vec::new(),
        other => vec![other.clone()],
    }
}

fn apply_modify_list(existing: JsonValue, mod_list: ModifyListValue) -> JsonValue {
    let mut base = match flatten_value(existing) {
        JsonValue::Array(items) => items,
        JsonValue::Null => Vec::new(),
        _ => Vec::new(),
    };

    let mut out = Vec::new();
    out.extend(mod_list.prepend.into_iter().map(flatten_value));
    out.append(&mut base);
    out.extend(mod_list.append.into_iter().map(flatten_value));
    JsonValue::Array(out)
}

fn flatten_value(value: JsonValue) -> JsonValue {
    if let Some(mod_list) = modify_list_value(&value) {
        return apply_modify_list(JsonValue::Array(Vec::new()), mod_list);
    }

    match value {
        JsonValue::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k, flatten_value(v));
            }
            JsonValue::Object(out)
        }
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(flatten_value).collect())
        }
        other => other,
    }
}

fn normalize_pkg_rs(data: &mut JsonValue) {
    let Some(map) = data.as_object_mut() else {
        return;
    };

    let pkg_rs = map.remove("pkg_rs");
    let Some(pkg_rs) = pkg_rs else {
        return;
    };

    let plugins = map.entry("plugins").or_insert_with(|| JsonValue::Object(Map::new()));
    if !plugins.is_object() {
        *plugins = JsonValue::Object(Map::new());
    }

    if let JsonValue::Object(plugin_map) = plugins {
        let entry = plugin_map
            .entry("pkg_rs")
            .or_insert_with(|| JsonValue::Object(Map::new()));
        deep_merge(entry, pkg_rs);
    }
}

fn apply_env_overrides(data: &mut JsonValue) {
    let Some(map) = data.as_object_mut() else {
        return;
    };

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if key == "plugins" || key == "pkg_rs" {
            continue;
        }

        let env_key = format!("REZ_{}", key.to_uppercase());
        if let Ok(raw) = env::var(&env_key) {
            let override_value = if key == "packages_path" {
                JsonValue::Array(
                    env::split_paths(&raw)
                        .map(|p| JsonValue::String(p.to_string_lossy().to_string()))
                        .collect(),
                )
            } else {
                parse_env_override(&raw, map.get(&key))
            };
            map.insert(key.clone(), override_value);
        }

        let json_key = format!("{env_key}_JSON");
        if let Ok(raw) = env::var(&json_key) {
            if let Ok(json_val) = serde_json::from_str(&raw) {
                map.insert(key.clone(), json_val);
            }
        }
    }
}

fn parse_env_override(raw: &str, template: Option<&JsonValue>) -> JsonValue {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("null") || raw.eq_ignore_ascii_case("none") {
        return JsonValue::Null;
    }

    match template {
        Some(JsonValue::Array(_)) => JsonValue::Array(
            split_env_list(raw)
                .into_iter()
                .map(JsonValue::String)
                .collect(),
        ),
        Some(JsonValue::Object(_)) => JsonValue::Object(parse_env_dict(raw)),
        Some(JsonValue::Bool(_)) => JsonValue::Bool(parse_env_bool(raw)),
        Some(JsonValue::Number(_)) => parse_env_number(raw).unwrap_or_else(|| {
            JsonValue::String(raw.to_string())
        }),
        Some(JsonValue::String(_)) => JsonValue::String(raw.to_string()),
        _ => JsonValue::String(raw.to_string()),
    }
}

fn split_env_list(raw: &str) -> Vec<String> {
    let parts: Vec<&str> = if raw.contains(',') {
        raw.split(',').collect()
    } else {
        raw.split_whitespace().collect()
    };
    parts
        .into_iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn parse_env_dict(raw: &str) -> Map<String, JsonValue> {
    let mut map = Map::new();
    let parts: Vec<&str> = if raw.contains(',') {
        raw.split(',').collect()
    } else {
        raw.split_whitespace().collect()
    };

    for part in parts {
        let mut iter = part.splitn(2, ':');
        let key = iter.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }
        let value = iter.next().unwrap_or("").trim();
        map.insert(key.to_string(), JsonValue::String(value.to_string()));
    }

    map
}

fn parse_env_bool(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "y"
    )
}

fn parse_env_number(raw: &str) -> Option<JsonValue> {
    if let Ok(int_val) = raw.parse::<i64>() {
        return Some(JsonValue::Number(int_val.into()));
    }
    if let Ok(float_val) = raw.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(float_val) {
            return Some(JsonValue::Number(num));
        }
    }
    None
}

fn expand_config_vars(data: &mut JsonValue) {
    let system = SystemInfo::current();
    expand_value(data, &system);
}

fn expand_value(value: &mut JsonValue, system: &SystemInfo) {
    match value {
        JsonValue::String(text) => {
            *text = expand_string(text, system);
        }
        JsonValue::Array(items) => {
            for item in items {
                expand_value(item, system);
            }
        }
        JsonValue::Object(map) => {
            for value in map.values_mut() {
                expand_value(value, system);
            }
        }
        _ => {}
    }
}

fn expand_string(input: &str, system: &SystemInfo) -> String {
    let mut value = expand_tilde(input, system);
    value = expand_system_tokens(&value, system);
    value = expand_env_tokens(&value);
    value
}

fn expand_tilde(input: &str, system: &SystemInfo) -> String {
    if system.home.is_empty() {
        return input.to_string();
    }
    if input == "~" {
        return system.home.clone();
    }
    if input.starts_with("~/") || input.starts_with("~\\") {
        return format!("{}{}", system.home, &input[1..]);
    }
    input.to_string()
}

fn expand_system_tokens(input: &str, system: &SystemInfo) -> String {
    static SYSTEM_RE: OnceLock<Regex> = OnceLock::new();
    let re = SYSTEM_RE.get_or_init(|| Regex::new(r"\{system\.([A-Za-z0-9_]+)\}").unwrap());
    re.replace_all(input, |caps: &regex::Captures| {
        system
            .get(caps.get(1).map(|m| m.as_str()).unwrap_or(""))
            .unwrap_or(caps.get(0).map(|m| m.as_str()).unwrap_or(""))
            .to_string()
    })
    .to_string()
}

fn expand_env_tokens(input: &str) -> String {
    static ENV_BRACED: OnceLock<Regex> = OnceLock::new();
    static ENV_DOLLAR: OnceLock<Regex> = OnceLock::new();
    static ENV_PERCENT: OnceLock<Regex> = OnceLock::new();

    let mut value = input.to_string();

    let re = ENV_BRACED.get_or_init(|| Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap());
    value = re
        .replace_all(&value, |caps: &regex::Captures| {
            let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            env::var(key).unwrap_or_else(|_| caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string())
        })
        .to_string();

    let re = ENV_PERCENT.get_or_init(|| Regex::new(r"%([A-Za-z_][A-Za-z0-9_]*)%").unwrap());
    value = re
        .replace_all(&value, |caps: &regex::Captures| {
            let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            env::var(key).unwrap_or_else(|_| caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string())
        })
        .to_string();

    let re = ENV_DOLLAR.get_or_init(|| Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap());
    value = re
        .replace_all(&value, |caps: &regex::Captures| {
            let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            env::var(key).unwrap_or_else(|_| caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string())
        })
        .to_string();

    value
}

#[derive(Debug, Clone)]
struct SystemInfo {
    platform: String,
    arch: String,
    os: String,
    user: String,
    home: String,
    hostname: String,
    fqdn: String,
    domain: String,
    rez_version: String,
}

impl SystemInfo {
    fn current() -> Self {
        let platform = match std::env::consts::OS {
            "windows" => "windows".to_string(),
            "macos" => "osx".to_string(),
            other => other.to_string(),
        };

        let arch = safe_version_string(std::env::consts::ARCH);
        let os = safe_version_string(&platform);

        let user = env::var("USERNAME")
            .or_else(|_| env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string());

        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();

        let hostname = env::var("COMPUTERNAME")
            .or_else(|_| env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        let fqdn = env::var("REZ_FQDN").unwrap_or_else(|_| hostname.clone());
        let domain = fqdn
            .split_once('.')
            .map(|(_, d)| d.to_string())
            .unwrap_or_default();

        Self {
            platform,
            arch,
            os,
            user,
            home,
            hostname,
            fqdn,
            domain,
            rez_version: crate::VERSION.to_string(),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "platform" => Some(&self.platform),
            "arch" => Some(&self.arch),
            "os" => Some(&self.os),
            "user" => Some(&self.user),
            "home" => Some(&self.home),
            "hostname" => Some(&self.hostname),
            "fqdn" => Some(&self.fqdn),
            "domain" => Some(&self.domain),
            "rez_version" => Some(&self.rez_version),
            _ => None,
        }
    }
}

fn safe_version_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out.trim_matches(&['.', '-'][..]).to_string()
}
