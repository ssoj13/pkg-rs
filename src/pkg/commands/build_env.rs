//! Build environment command (internal).

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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
    if cfg!(windows) {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/K");
        cmd.current_dir(build_path);
        cmd.envs(env_map);
        let status = cmd.status().map_err(|e| e.to_string())?;
        return Ok(if status.success() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let mut cmd = Command::new(shell);
    cmd.current_dir(build_path);
    cmd.envs(env_map);
    let status = cmd.status().map_err(|e| e.to_string())?;

    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
