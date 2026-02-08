//! Build environment command (internal).

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pkg_lib::{config, plugins};

pub fn cmd_build_env(
    build_path: PathBuf,
    variant_index: Option<usize>,
    install: bool,
    install_path: Option<PathBuf>,
) -> ExitCode {
    let rxt_path = build_path.join("build.rxt");
    if !rxt_path.exists() {
        eprintln!("build.rxt not found: {}", rxt_path.display());
        return ExitCode::FAILURE;
    }

    let env_map = match load_pkg_env(&rxt_path) {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to load build.rxt: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let mut env_map = env_map;
    if let Some(index) = variant_index {
        env_map
            .entry("REZ_BUILD_VARIANT_INDEX".to_string())
            .or_insert_with(|| index.to_string());
    }
    if install {
        env_map
            .entry("REZ_BUILD_INSTALL".to_string())
            .or_insert_with(|| "1".to_string());
    }
    if let Some(path) = install_path {
        env_map
            .entry("REZ_BUILD_INSTALL_PATH".to_string())
            .or_insert_with(|| path.display().to_string());
    }

    match spawn_shell(&build_path, &env_map) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Failed to spawn build shell: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn load_pkg_env(path: &PathBuf) -> Result<HashMap<String, String>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let env = doc
        .get("pkg_env")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "pkg_env not found in build.rxt".to_string())?;

    let mut map = HashMap::new();
    for (key, value) in env {
        if let Some(val) = value.as_str() {
            map.insert(key.to_string(), val.to_string());
        }
    }

    Ok(map)
}

fn spawn_shell(build_path: &PathBuf, env_map: &HashMap<String, String>) -> Result<ExitCode, String> {
    let shell_name = resolve_shell_name();
    let shell_name = normalize_shell_name(&shell_name);

    let plugin_config = plugins::PluginConfig::from_config(config::get().ok());
    if !plugins::is_enabled(&plugin_config.shells, &shell_name) {
        return Err(format!(
            "shell '{}' is not enabled (enabled: {}). Set plugins.pkg_rs.shells in rezconfig.py",
            shell_name,
            if plugin_config.shells.is_empty() {
                "none".to_string()
            } else {
                plugin_config.shells.join(", ")
            }
        ));
    }

    let (program, args) = shell_command(&shell_name)?;

    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.current_dir(build_path);
    cmd.envs(env_map);
    let status = cmd.status().map_err(|e| e.to_string())?;

    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn resolve_shell_name() -> String {
    if let Ok(raw) = std::env::var("REZ_SHELL") {
        if !raw.trim().is_empty() {
            return raw;
        }
    }

    if let Ok(cfg) = config::get() {
        if let Some(shell) = config::get_str(cfg, "default_shell") {
            if !shell.trim().is_empty() {
                return shell;
            }
        }
    }

    if cfg!(windows) {
        return "powershell".to_string();
    }

    if let Ok(shell) = std::env::var("SHELL") {
        if let Some(name) = Path::new(&shell).file_name().and_then(|s| s.to_str()) {
            if !name.trim().is_empty() {
                return name.to_string();
            }
        }
    }

    "bash".to_string()
}

fn normalize_shell_name(raw: &str) -> String {
    let mut name = raw.trim().to_ascii_lowercase();
    if let Some(stripped) = name.strip_suffix(".exe") {
        name = stripped.to_string();
    }
    name
}

fn shell_command(shell: &str) -> Result<(String, Vec<String>), String> {
    match shell {
        "cmd" => Ok(("cmd.exe".to_string(), vec!["/K".to_string()])),
        "powershell" => Ok((
            "powershell.exe".to_string(),
            vec!["-NoExit".to_string()],
        )),
        "pwsh" => Ok(("pwsh".to_string(), vec!["-NoExit".to_string()])),
        "bash" | "sh" | "zsh" | "csh" | "tcsh" | "gitbash" => {
            Ok((shell.to_string(), Vec::new()))
        }
        _ => Err(format!("unsupported shell: {}", shell)),
    }
}
