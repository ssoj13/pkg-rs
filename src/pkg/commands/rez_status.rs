//! Rez status command (native subset with fallback).

use crate::cli::RezStubArgs;
use crate::commands::cmd_rez_passthrough;
use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn cmd_rez_status(args: &RezStubArgs) -> ExitCode {
    let parsed = match parse_status_args(&args.args) {
        Ok(parsed) => parsed,
        Err(_) => return cmd_rez_passthrough("status", &args.args),
    };

    if parsed.fallback {
        return cmd_rez_passthrough("status", &args.args);
    }

    run_status()
}

#[derive(Debug, Default)]
struct StatusArgs {
    fallback: bool,
}

fn parse_status_args(args: &[String]) -> Result<StatusArgs, ()> {
    let mut parsed = StatusArgs::default();
    for arg in args {
        if arg == "-t" || arg == "--tools" {
            parsed.fallback = true;
        } else if !arg.starts_with('-') {
            parsed.fallback = true;
        } else {
            parsed.fallback = true;
        }
    }
    Ok(parsed)
}

fn run_status() -> ExitCode {
    let rez_version = detect_rez_version().unwrap_or_else(|| "unknown".to_string());
    println!("Using Rez v{}\n", rez_version);

    let rxt_path = env::var("REZ_RXT_FILE").ok().map(PathBuf::from);
    if let Some(path) = rxt_path {
        if path.exists() {
            println!("Active context: {}\n", path.display());
        } else {
            println!("No active context.\n");
        }
    } else {
        println!("No active context.\n");
    }

    let suites = visible_suite_paths();
    if suites.is_empty() {
        println!("No visible suites.");
    } else {
        println!("Visible suites:");
        for suite in suites {
            println!("{}", suite.display());
        }
    }

    ExitCode::SUCCESS
}

fn detect_rez_version() -> Option<String> {
    let root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))?;
    let mut cur = Some(root.as_path());
    while let Some(path) = cur {
        let rez_root = path.join("python").join("rez");
        let version_file = rez_root.join("utils").join("_version.py");
        if version_file.is_file() {
            if let Ok(content) = std::fs::read_to_string(&version_file) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("_rez_version") {
                        let parts: Vec<&str> = line.split('=').collect();
                        if parts.len() == 2 {
                            return Some(
                                parts[1]
                                    .trim()
                                    .trim_matches('\"')
                                    .trim_matches('\'')
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }

        let init_file = rez_root.join("__init__.py");
        if init_file.is_file() {
            if let Ok(content) = std::fs::read_to_string(&init_file) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("__version__") {
                        let parts: Vec<&str> = line.split('=').collect();
                        if parts.len() == 2 {
                            return Some(
                                parts[1]
                                    .trim()
                                    .trim_matches('\"')
                                    .trim_matches('\'')
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }

        cur = path.parent();
    }
    None
}

fn visible_suite_paths() -> Vec<PathBuf> {
    let mut suites = BTreeSet::new();
    let path_var = env::var("PATH").unwrap_or_default();
    for entry in env::split_paths(&path_var) {
        if entry.as_os_str().is_empty() {
            continue;
        }
        if let Some(parent) = entry.parent() {
            let suite_file = parent.join("suite.yaml");
            if suite_file.is_file() {
                suites.insert(parent.to_path_buf());
            }
        }
    }
    suites.into_iter().collect()
}
