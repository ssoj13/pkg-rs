//! Rez context command (native subset with fallback).

use crate::cli::RezStubArgs;
use crate::commands::cmd_rez_passthrough;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_context(args: &RezStubArgs) -> ExitCode {
    let parsed = match parse_context_args(&args.args) {
        Ok(parsed) => parsed,
        Err(_) => return cmd_rez_passthrough("context", &args.args),
    };

    if parsed.fallback {
        return cmd_rez_passthrough("context", &args.args);
    }

    run_context(parsed)
}

#[derive(Debug, Default)]
struct ContextArgs {
    rxt: Option<PathBuf>,
    print_request: bool,
    print_resolve: bool,
    show_uris: bool,
    print_graph: bool,
    write_graph: Option<PathBuf>,
    format: Option<String>,
    interpret: bool,
    which: Option<String>,
    no_env: bool,
    fallback: bool,
}

fn parse_context_args(args: &[String]) -> Result<ContextArgs, ()> {
    let mut parsed = ContextArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--req" | "--print-request" => {
                parsed.print_request = true;
                i += 1;
            }
            "--res" | "--print-resolve" => {
                parsed.print_resolve = true;
                i += 1;
            }
            "--su" | "--show-uris" => {
                parsed.show_uris = true;
                i += 1;
            }
            "--pg" | "--print-graph" => {
                parsed.print_graph = true;
                i += 1;
            }
            "--wg" | "--write-graph" => {
                if i + 1 >= args.len() {
                    return Err(());
                }
                parsed.write_graph = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "-f" | "--format" => {
                if i + 1 >= args.len() {
                    return Err(());
                }
                parsed.format = Some(args[i + 1].clone());
                i += 2;
            }
            "-i" | "--interpret" => {
                parsed.interpret = true;
                i += 1;
            }
            "--which" => {
                if i + 1 >= args.len() {
                    return Err(());
                }
                parsed.which = Some(args[i + 1].clone());
                i += 2;
            }
            "--no-env" => {
                parsed.no_env = true;
                i += 1;
            }
            // Known flags we don't implement yet -> fallback to python
            "--tools"
            | "--graph"
            | "--dependency-graph"
            | "--diff"
            | "--fetch"
            | "--so"
            | "--source-order"
            | "--pp"
            | "--prune-package"
            | "-g"
            | "-d"
            | "-t" => {
                parsed.fallback = true;
                i += 1;
            }
            "--" => {
                parsed.fallback = true;
                break;
            }
            _ if arg.starts_with('-') => {
                parsed.fallback = true;
                i += 1;
            }
            _ => {
                if parsed.rxt.is_none() {
                    parsed.rxt = Some(PathBuf::from(arg));
                } else {
                    parsed.fallback = true;
                }
                i += 1;
            }
        }
    }

    Ok(parsed)
}

fn run_context(args: ContextArgs) -> ExitCode {
    let rxt_path = match args.rxt {
        Some(path) => path,
        None => match env::var("REZ_RXT_FILE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                eprintln!("not in a resolved environment context.");
                return ExitCode::FAILURE;
            }
        },
    };

    let doc = match load_rxt(&rxt_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Failed to load context: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if args.print_request {
        for req in get_string_list(&doc, "package_requests") {
            println!("{}", req);
        }
        return ExitCode::SUCCESS;
    }

    if args.print_resolve {
        for entry in resolved_packages(&doc, args.show_uris) {
            println!("{}", entry);
        }
        return ExitCode::SUCCESS;
    }

    if args.print_graph || args.write_graph.is_some() {
        let graph = doc
            .get("graph")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        if let Some(path) = args.write_graph {
            if let Err(err) = std::fs::write(&path, graph) {
                eprintln!("Failed to write graph: {}", err);
                return ExitCode::FAILURE;
            }
        } else {
            println!("{}", graph);
        }
        return ExitCode::SUCCESS;
    }

    if args.which.is_some()
        || args.interpret
        || matches!(args.format.as_deref(), Some("dict" | "table" | "json"))
    {
        let env_map = get_env_map(&doc);
        if let Some(cmd) = args.which {
            if let Some(found) = find_in_path(&env_map, &cmd) {
                println!("{}", found.display());
                return ExitCode::SUCCESS;
            }
            eprintln!("'{}' not found in the context", cmd);
            return ExitCode::FAILURE;
        }

        let format = args.format.unwrap_or_else(|| "dict".to_string());
        if format == "json" {
            match serde_json::to_string_pretty(&env_map) {
                Ok(txt) => println!("{}", txt),
                Err(err) => {
                    eprintln!("Failed to serialize env: {}", err);
                    return ExitCode::FAILURE;
                }
            }
        } else if format == "table" {
            let mut rows: Vec<(String, String)> =
                env_map.into_iter().collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in rows {
                println!("{:<30} {}", k, v);
            }
        } else {
            let mut rows: Vec<(String, String)> =
                env_map.into_iter().collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in rows {
                println!("{}: {}", k, v);
            }
        }
        return ExitCode::SUCCESS;
    }

    print_context_summary(&doc);
    ExitCode::SUCCESS
}

fn load_rxt(path: &Path) -> Result<JsonValue, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn get_string_list(doc: &JsonValue, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn resolved_packages(doc: &JsonValue, show_uris: bool) -> Vec<String> {
    let mut out = Vec::new();
    let Some(list) = doc.get("resolved_packages").and_then(|v| v.as_array()) else {
        return out;
    };
    for item in list {
        let vars = item
            .get("variables")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let name = vars.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let version = vars
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let location = vars
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if show_uris && !location.is_empty() {
            let mut uri = PathBuf::from(location);
            uri.push(name);
            uri.push(version);
            out.push(uri.display().to_string());
        } else {
            out.push(format!("{}-{}", name, version));
        }
    }
    out
}

fn get_env_map(doc: &JsonValue) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(obj) = doc.get("pkg_env").and_then(|v| v.as_object()) else {
        return map;
    };
    for (key, value) in obj {
        if let Some(val) = value.as_str() {
            map.insert(key.clone(), val.to_string());
        }
    }
    map
}

fn find_in_path(env_map: &BTreeMap<String, String>, cmd: &str) -> Option<PathBuf> {
    let path = env_map
        .get("PATH")
        .or_else(|| env_map.get("Path"))
        .or_else(|| env_map.get("path"))?;
    let candidates = if cfg!(windows) && Path::new(cmd).extension().is_none() {
        vec![format!("{}.exe", cmd), cmd.to_string()]
    } else {
        vec![cmd.to_string()]
    };
    for entry in env::split_paths(path) {
        for candidate in &candidates {
            let p = entry.join(candidate);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn print_context_summary(doc: &JsonValue) {
    let user = doc.get("user").and_then(|v| v.as_str()).unwrap_or("unknown");
    let host = doc.get("host").and_then(|v| v.as_str()).unwrap_or("unknown");
    let rez_version = doc
        .get("rez_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    println!(
        "resolved by {}@{}, using Rez v{}",
        user, host, rez_version
    );

    let requested = get_string_list(doc, "package_requests");
    if !requested.is_empty() {
        println!("\nrequested packages:");
        for req in requested {
            println!("{}", req);
        }
    }

    let resolved = resolved_packages(doc, false);
    if !resolved.is_empty() {
        println!("\nresolved packages:");
        for pkg in resolved {
            println!("{}", pkg);
        }
    }
}
