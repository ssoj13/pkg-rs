//! rez plugins command (native).

use crate::cli::PluginsArgs;
use pkg_lib::config;
use pkg_lib::Storage;
use std::process::ExitCode;

pub fn cmd_rez_plugins(storage: &Storage, args: &PluginsArgs) -> ExitCode {
    let storage = match build_storage_override(storage, args) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("rez plugins: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let mut plugins = Vec::new();
    for pkg in storage.packages() {
        if pkg
            .plugin_for
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&args.pkg))
        {
            plugins.push(pkg.name.clone());
        }
    }

    if plugins.is_empty() {
        eprintln!("package '{}' has no plugins.", args.pkg);
        return ExitCode::SUCCESS;
    }

    plugins.sort();
    println!("{}", plugins.join("\n"));
    ExitCode::SUCCESS
}

fn build_storage_override(storage: &Storage, args: &PluginsArgs) -> Result<Storage, String> {
    if args.paths.is_none() {
        return Ok(storage.clone());
    }

    let mut paths = Vec::new();
    if let Some(raw) = args.paths.as_deref() {
        paths.extend(std::env::split_paths(raw));
    }

    if paths.is_empty() {
        let cfg = config::get().map_err(|e| e.to_string())?;
        paths = config::packages_path(cfg);
    }

    if paths.is_empty() {
        return Err("no package search paths available".to_string());
    }

    Storage::scan_impl(Some(&paths)).map_err(|e| e.to_string())
}
