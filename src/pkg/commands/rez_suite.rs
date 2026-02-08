//! Rez suite command (native subset with fallback).

use crate::cli::RezStubArgs;
use crate::commands::cmd_rez_passthrough;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_suite(args: &RezStubArgs) -> ExitCode {
    let parsed = match parse_suite_args(&args.args) {
        Ok(parsed) => parsed,
        Err(_) => return cmd_rez_passthrough("suite", &args.args),
    };

    if parsed.fallback {
        return cmd_rez_passthrough("suite", &args.args);
    }

    run_suite(parsed)
}

#[derive(Debug, Default)]
struct SuiteArgs {
    list: bool,
    create: bool,
    dir: Option<PathBuf>,
    fallback: bool,
}

fn parse_suite_args(args: &[String]) -> Result<SuiteArgs, ()> {
    let mut parsed = SuiteArgs::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" | "-l" => {
                parsed.list = true;
                i += 1;
            }
            "--create" => {
                parsed.create = true;
                i += 1;
            }
            "--tools"
            | "--which"
            | "--validate"
            | "--add"
            | "--remove"
            | "--context"
            | "--interactive"
            | "--prefix"
            | "--suffix"
            | "--hide"
            | "--unhide"
            | "--alias"
            | "--unalias"
            | "--bump"
            | "--find-request"
            | "--find-resolve"
            | "--prefix-char"
            | "-t"
            | "-c"
            | "-i"
            | "-a"
            | "-r"
            | "-d"
            | "-p"
            | "-s"
            | "-P"
            | "-b" => {
                parsed.fallback = true;
                i += 1;
            }
            "--" => {
                parsed.fallback = true;
                break;
            }
            arg if arg.starts_with('-') => {
                parsed.fallback = true;
                i += 1;
            }
            _ => {
                if parsed.dir.is_none() {
                    parsed.dir = Some(PathBuf::from(&args[i]));
                } else {
                    parsed.fallback = true;
                }
                i += 1;
            }
        }
    }
    Ok(parsed)
}

fn run_suite(args: SuiteArgs) -> ExitCode {
    if args.list {
        let suites = visible_suite_paths();
        if suites.is_empty() {
            println!("No visible suites.");
        } else {
            for suite in suites {
                println!("{}", suite.display());
            }
        }
        return ExitCode::SUCCESS;
    }

    let dir = match args.dir {
        Some(dir) => dir,
        None => {
            eprintln!("DIR required.");
            return ExitCode::FAILURE;
        }
    };

    if args.create {
        if let Err(err) = create_suite(&dir) {
            eprintln!("Failed to create suite: {}", err);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    match print_suite_info(&dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Failed to read suite: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn create_suite(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err(format!("Cannot save, path exists: {}", path.display()));
    }
    std::fs::create_dir_all(path.join("contexts")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(path.join("bin")).map_err(|e| e.to_string())?;
    let mut root = HashMap::new();
    root.insert("contexts".to_string(), YamlValue::Mapping(Default::default()));
    let yaml = serde_yaml::to_string(&root).map_err(|e| e.to_string())?;
    std::fs::write(path.join("suite.yaml"), yaml).map_err(|e| e.to_string())?;
    Ok(())
}

fn print_suite_info(path: &Path) -> Result<(), String> {
    let suite_file = path.join("suite.yaml");
    if !suite_file.is_file() {
        return Err(format!("Not a suite: {}", path.display()));
    }
    let content = std::fs::read_to_string(&suite_file).map_err(|e| e.to_string())?;
    let doc: YamlValue = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
    let contexts = doc
        .get("contexts")
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();
    if contexts.is_empty() {
        println!("Suite contains 0 contexts.");
        return Ok(());
    }
    println!("Suite contains {} contexts:", contexts.len());
    let mut names: Vec<String> = contexts
        .keys()
        .filter_map(|k| k.as_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    for name in names {
        println!("{}", name);
    }
    Ok(())
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
