//! Rez interpret command (minimal Rex interpreter).

use crate::cli::InterpretArgs;
use pkg_lib::config;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn cmd_rez_interpret(args: &InterpretArgs) -> ExitCode {
    let code = match fs::read_to_string(&args.file) {
        Ok(txt) => txt,
        Err(err) => {
            eprintln!("rez interpret: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let shell = resolve_shell(args.format.as_deref());

    let mut env_map = if args.no_env { BTreeMap::new() } else { current_env() };
    let parent_vars = normalize_parent_vars(&args.parent_vars);

    if !args.no_env && !parent_vars.is_empty() {
        // keep only parent vars if explicitly set and not "all"
        if !parent_vars.contains_key("*") {
            env_map = env_map
                .into_iter()
                .filter(|(k, _)| parent_vars.contains_key(&k.to_ascii_uppercase()))
                .collect();
        }
    }

    let mut aliases: BTreeMap<String, String> = BTreeMap::new();

    for line in code.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            if let Some((name, value)) = split_assign(rest) {
                let value = expand_tokens(value, &env_map);
                env_map.insert(name.to_string(), value);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("setenv ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            if !name.is_empty() {
                let value = expand_tokens(value, &env_map);
                env_map.insert(name.to_string(), value);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("unset ") {
            let name = rest.trim();
            env_map.remove(name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("unsetenv ") {
            let name = rest.trim();
            env_map.remove(name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("alias ") {
            if let Some((name, value)) = split_alias(rest) {
                aliases.insert(name.to_string(), value.to_string());
            }
            continue;
        }
    }

    match shell.as_str() {
        "dict" => {
            print_dict(&env_map);
        }
        "table" => {
            print_table(&env_map);
        }
        _ => {
            print_shell(&shell, &env_map, &aliases);
        }
    }

    ExitCode::SUCCESS
}

fn current_env() -> BTreeMap<String, String> {
    env::vars().collect()
}

fn normalize_parent_vars(vars: &[String]) -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    if vars.is_empty() {
        return out;
    }
    for v in vars {
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        if v.eq_ignore_ascii_case("all") {
            out.insert("*".to_string(), true);
            return out;
        }
        out.insert(v.to_ascii_uppercase(), true);
    }
    out
}

fn split_assign(text: &str) -> Option<(&str, &str)> {
    let mut parts = text.splitn(2, '=');
    let name = parts.next()?.trim();
    let value = parts.next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some((name, value))
    }
}

fn split_alias(text: &str) -> Option<(&str, &str)> {
    let mut parts = text.splitn(2, '=');
    let name = parts.next()?.trim();
    let value = parts.next().unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }
    Some((name, strip_quotes(value)))
}

fn strip_quotes(value: &str) -> &str {
    let s = value.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn expand_tokens(value: &str, env_map: &BTreeMap<String, String>) -> String {
    let mut out = value.to_string();
    let tokens = ["ROOT", "VERSION", "NAME", "BASE"]; // common rex tokens
    for token in tokens {
        let needle = format!("!{}!", token);
        if out.contains(&needle) {
            let replace = env_map.get(token).cloned().unwrap_or_default();
            out = out.replace(&needle, &replace);
        }
    }
    out
}

fn resolve_shell(format: Option<&str>) -> String {
    if let Some(fmt) = format {
        return fmt.to_ascii_lowercase();
    }
    if let Ok(cfg) = config::get() {
        if let Some(shell) = config::get_str(cfg, "default_shell") {
            if !shell.trim().is_empty() {
                return shell.to_ascii_lowercase();
            }
        }
    }
    if cfg!(windows) {
        return "powershell".to_string();
    }
    if let Ok(shell) = env::var("SHELL") {
        if let Some(name) = Path::new(&shell).file_name().and_then(|s| s.to_str()) {
            return name.to_ascii_lowercase();
        }
    }
    "bash".to_string()
}

fn print_dict(env_map: &BTreeMap<String, String>) {
    let json = serde_json::to_string_pretty(env_map).unwrap_or_else(|_| "{}".to_string());
    println!("{}", json);
}

fn print_table(env_map: &BTreeMap<String, String>) {
    let width = env_map.keys().map(|k| k.len()).max().unwrap_or(0);
    for (k, v) in env_map {
        println!("{k:width$}  {v}", width = width);
    }
}

fn print_shell(shell: &str, env_map: &BTreeMap<String, String>, aliases: &BTreeMap<String, String>) {
    match shell {
        "cmd" => {
            for (k, v) in env_map {
                println!("set {}={}", k, v);
            }
            for (k, v) in aliases {
                println!("doskey {}={}", k, v);
            }
        }
        "powershell" | "pwsh" => {
            for (k, v) in env_map {
                println!("$env:{} = \"{}\"", k, escape_ps(v));
            }
            for (k, v) in aliases {
                println!("Set-Alias {} {}", k, v);
            }
        }
        _ => {
            for (k, v) in env_map {
                println!("export {}=\"{}\"", k, escape_sh(v));
            }
            for (k, v) in aliases {
                println!("alias {}='{}'", k, v.replace('\\', "\\\\").replace('\'', "'\\''"));
            }
        }
    }
}

fn escape_sh(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_ps(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}
