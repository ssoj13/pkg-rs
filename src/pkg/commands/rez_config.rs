//! Rez-compatible config command.

use crate::cli::RezConfigArgs;
use pkg_lib::config;
use serde_json::Value as JsonValue;
use std::process::ExitCode;

pub fn cmd_rez_config(args: &RezConfigArgs) -> ExitCode {
    let cfg = match config::get() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("Config error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if args.search_list {
        for path in &cfg.filepaths {
            println!("{}", path.display());
        }
        return ExitCode::SUCCESS;
    }

    if args.source_list {
        for path in &cfg.sourced_filepaths {
            println!("{}", path.display());
        }
        return ExitCode::SUCCESS;
    }

    let mut data = &cfg.data;
    if let Some(field) = &args.field {
        for part in field.split('.') {
            match data.get(part) {
                Some(value) => data = value,
                None => {
                    eprintln!("no such setting: {}", field);
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    match data {
        JsonValue::Object(_) | JsonValue::Array(_) => {
            if args.json {
                match serde_json::to_string(data) {
                    Ok(txt) => println!("{}", txt),
                    Err(err) => {
                        eprintln!("Failed to serialize config as JSON: {}", err);
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                match serde_yaml::to_string(data) {
                    Ok(txt) => println!("{}", txt.trim()),
                    Err(err) => {
                        eprintln!("Failed to serialize config as YAML: {}", err);
                        return ExitCode::FAILURE;
                    }
                }
            }
        }
        JsonValue::String(value) => println!("{}", value),
        JsonValue::Number(value) => println!("{}", value),
        JsonValue::Bool(value) => println!("{}", value),
        JsonValue::Null => println!("null"),
    }

    ExitCode::SUCCESS
}
