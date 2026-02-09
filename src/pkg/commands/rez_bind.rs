//! Rez bind command. Uses bind_module registry (native + Python strategy); add/remove via config.

use crate::cli::BindArgs;
use pkg_lib::bind_module;
use pkg_lib::config;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn cmd_rez_bind(args: &BindArgs) -> ExitCode {
    if args.list {
        return cmd_bind_list();
    }
    if args.search {
        return cmd_bind_search(args.package.as_deref().unwrap_or(""));
    }

    if !args.quickstart {
        if let Some(pkg) = args.package.as_deref() {
            let path = match resolve_install_path(args) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = bind_module::bind_one(pkg, &path, args.no_deps) {
                eprintln!("rez bind: {}", e);
                return ExitCode::FAILURE;
            }
            println!("Bound {} into {}", pkg, path.display());
            return ExitCode::SUCCESS;
        }
        eprintln!("rez bind: укажите пакет (pkg bind --list), или --quickstart, или --list / --search");
        return ExitCode::FAILURE;
    }

    cmd_quickstart(args)
}

fn cmd_bind_list() -> ExitCode {
    let reg = bind_module::registry();
    println!("Bind modules (config: plugins.pkg_rs.bind_modules_extra / bind_modules_remove):");
    for m in &reg {
        println!("  {} [{}]", m.name, m.strategy_label());
    }
    ExitCode::SUCCESS
}

fn cmd_bind_search(pattern: &str) -> ExitCode {
    let found = bind_module::search_names(pattern);
    if found.is_empty() {
        println!("No matching bind modules.");
    } else {
        for name in found {
            println!("{}", name);
        }
    }
    ExitCode::SUCCESS
}

fn resolve_install_path(args: &BindArgs) -> Result<PathBuf, String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    if let Some(path) = &args.install_path {
        return Ok(path.clone());
    }
    if args.release {
        return config::release_packages_path(cfg)
            .ok_or_else(|| "rez bind --release: release_packages_path is not set".to_string());
    }
    config::local_packages_path(cfg)
        .ok_or_else(|| "rez bind: local_packages_path is not set".to_string())
}

fn cmd_quickstart(args: &BindArgs) -> ExitCode {
    let install_path = match resolve_install_path(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = std::fs::create_dir_all(&install_path) {
        eprintln!("rez bind: failed to create install path {}: {}", install_path.display(), e);
        return ExitCode::FAILURE;
    }

    let registry = bind_module::registry();
    let mut python_names = Vec::new();

    for module in &registry {
        if bind_module::package_already_installed(&install_path, &module.name) {
            println!("Skipping {} (already installed)", module.name);
            continue;
        }

        if module.strategy_label() == "python" {
            python_names.push(module.name.clone());
        } else {
            println!("Binding {} into {}...", module.name, install_path.display());
            if let Err(e) = bind_module::bind_one(&module.name, &install_path, args.no_deps) {
                eprintln!("rez bind: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    if !python_names.is_empty() {
        for name in &python_names {
            println!("Binding {} into {}...", name, install_path.display());
        }
        if let Err(e) = bind_module::run_python_bind_batch(
            &python_names,
            &install_path,
            args.no_deps,
            true,
        ) {
            eprintln!("Rez bind quickstart error: {}", e);
            return ExitCode::FAILURE;
        }
        println!(
            "\nTo bind other software, use 'rez bind --list' then 'rez bind <name>'.\n"
        );
    }

    ExitCode::SUCCESS
}
