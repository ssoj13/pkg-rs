//! Rez yaml2py command.

use crate::cli::Yaml2pyArgs;
use pkg_lib::rez_version::{Version, VersionRange};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_yaml2py(args: &Yaml2pyArgs) -> ExitCode {
    let path = match resolve_path(args.path.as_ref()) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez yaml2py: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let content = match fs::read_to_string(&path) {
        Ok(txt) => txt,
        Err(err) => {
            eprintln!("rez yaml2py: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let yaml_value: JsonValue = match serde_yaml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("rez yaml2py: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let output = match yaml_to_py(&yaml_value) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("rez yaml2py: {}", err);
            return ExitCode::FAILURE;
        }
    };

    print!("{}", output);
    ExitCode::SUCCESS
}

fn resolve_path(path: Option<&PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = path {
        return Ok(expand_path(path));
    }

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(cwd.join("package.yaml"))
}

fn expand_path(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest.trim_start_matches(|c| c == '/' || c == '\\'));
        }
    }
    path.to_path_buf()
}

fn yaml_to_py(value: &JsonValue) -> Result<String, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "YAML root must be a mapping".to_string())?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'name' in YAML".to_string())?
        .to_string();

    if let Some(versions) = obj.get("versions") {
        let version_list = versions
            .as_array()
            .ok_or_else(|| "'versions' must be a list".to_string())?;
        let mut versions: Vec<String> = version_list
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if versions.is_empty() {
            return Err("'versions' list is empty".to_string());
        }
        versions.sort();
        let override_map = obj.get("version_overrides");
        let mut entries = Vec::new();
        for version in &versions {
            let mut data = obj.clone();
            let version_data = apply_version_overrides(&mut data, version, override_map)?;
            entries.push(version_data);
        }
        return Ok(render_multi_package_py(&name, &entries, versions.last().unwrap()));
    }

    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'version' in YAML".to_string())?
        .to_string();

    let mut data = obj.clone();
    data.insert("version".to_string(), JsonValue::String(version));
    Ok(render_single_package_py(&name, &data))
}

fn apply_version_overrides(
    data: &mut serde_json::Map<String, JsonValue>,
    version: &str,
    overrides: Option<&JsonValue>,
) -> Result<serde_json::Map<String, JsonValue>, String> {
    let mut out = data.clone();
    out.remove("versions");
    out.remove("version_overrides");
    out.insert("version".to_string(), JsonValue::String(version.to_string()));

    if let Some(overrides) = overrides {
        if let Some(map) = overrides.as_object() {
            let version_obj = Version::parse(version)
                .map_err(|e| format!("Invalid version '{}': {}", version, e))?;
            for (range_expr, override_val) in map {
                let range = VersionRange::parse(range_expr)
                    .map_err(|e| format!("Invalid version range '{}': {}", range_expr, e))?;
                if range.contains_version(&version_obj) {
                    if let Some(override_map) = override_val.as_object() {
                        deep_merge(&mut out, override_map.clone());
                    }
                }
            }
        }
    }

    Ok(out)
}

fn render_single_package_py(_name: &str, data: &serde_json::Map<String, JsonValue>) -> String {
    let mut out = String::new();
    out.push_str("from pkg import Package\n\n");
    out.push_str("def get_package(*args, **kwargs):\n");
    out.push_str("    data = ");
    out.push_str(&to_py_literal(&JsonValue::Object(data.clone())));
    out.push('\n');
    out.push_str("    data['base'] = data.pop('name')\n");
    out.push_str("    return Package.from_dict(data)\n");
    out
}

fn render_multi_package_py(_name: &str, entries: &[serde_json::Map<String, JsonValue>], default_version: &str) -> String {
    let mut out = String::new();
    out.push_str("from pkg import Package\n");
    out.push_str("import os\n\n");
    out.push_str("_PKGS = [\n");
    for entry in entries {
        out.push_str("    ");
        out.push_str(&to_py_literal(&JsonValue::Object(entry.clone())));
        out.push_str(",\n");
    }
    out.push_str("]\n\n");
    out.push_str("def _select_version():\n");
    out.push_str("    return (\n");
    out.push_str("        os.getenv('REZ_BUILD_VERSION')\n");
    out.push_str("        or os.getenv('REZ_PACKAGE_VERSION')\n");
    out.push_str("    )\n\n");
    out.push_str("def get_package(*args, **kwargs):\n");
    out.push_str("    version = kwargs.get('version') if kwargs else None\n");
    out.push_str("    if not version:\n");
    out.push_str("        version = _select_version()\n");
    out.push_str("    if not version:\n");
    out.push_str(&format!("        version = '{}'\n", default_version));
    out.push_str("    for data in _PKGS:\n");
    out.push_str("        if data.get('version') == version:\n");
    out.push_str("            payload = dict(data)\n");
    out.push_str("            payload['base'] = payload.pop('name')\n");
    out.push_str("            return Package.from_dict(payload)\n");
    out.push_str("    raise RuntimeError('Version not found: %s' % version)\n");
    out
}

fn to_py_literal(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "None".to_string(),
        JsonValue::Bool(v) => if *v { "True" } else { "False" }.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        JsonValue::Array(items) => {
            let inner = items.iter().map(to_py_literal).collect::<Vec<_>>().join(", ");
            format!("[{}]", inner)
        }
        JsonValue::Object(map) => {
            let mut items = Vec::new();
            for (k, v) in map {
                items.push(format!("'{}': {}", k.replace('\\', "\\\\").replace('\'', "\\'"), to_py_literal(v)));
            }
            format!("{{{}}}", items.join(", "))
        }
    }
}

fn deep_merge(target: &mut serde_json::Map<String, JsonValue>, patch: serde_json::Map<String, JsonValue>) {
    for (key, patch_val) in patch {
        match (target.get_mut(&key), patch_val) {
            (Some(JsonValue::Object(existing)), JsonValue::Object(patch_obj)) => {
                deep_merge(existing, patch_obj);
            }
            (_, patch_val) => {
                target.insert(key, patch_val);
            }
        }
    }
}
