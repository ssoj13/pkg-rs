//! Rez bind command. Uses bind_module registry (native + Python strategy); add/remove via config.

use crate::cli::RezStubArgs;
use pkg_lib::bind_module::{self, BindStrategy};
use pkg_lib::config;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn cmd_rez_bind(args: &RezStubArgs) -> ExitCode {
    let parsed = parse_bind_args(&args.args);

    if parsed.list {
        return cmd_bind_list();
    }
    if parsed.search {
        return cmd_bind_search(parsed.pkg.as_deref().unwrap_or(""));
    }

    if !parsed.quickstart {
        if !parsed.unknown.is_empty() {
            eprintln!("rez bind: unsupported arguments: {:?}", parsed.unknown);
            return ExitCode::FAILURE;
        }
        if let Some(pkg) = parsed.pkg.as_deref() {
            let path = match resolve_install_path(&parsed) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = bind_module::bind_one(pkg, &path, parsed.no_deps) {
                eprintln!("rez bind: {}", e);
                return ExitCode::FAILURE;
            }
            println!("Bound {} into {}", pkg, path.display());
            return ExitCode::SUCCESS;
        }
        eprintln!("rez bind: specify a package (rez bind --list) or --quickstart, or --list / --search");
        return ExitCode::FAILURE;
    }

    cmd_quickstart(parsed)
}

fn cmd_bind_list() -> ExitCode {
    let reg = bind_module::registry();
    println!("Bind modules (config: plugins.pkg_rs.bind_modules_extra / bind_modules_remove):");
    for m in &reg {
        let tag = match m.strategy {
            BindStrategy::Native => " [native]",
            BindStrategy::Python => " [python]",
        };
        println!("  {}{}", m.name, tag);
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

#[derive(Debug, Default)]
struct BindQuickstartArgs {
    quickstart: bool,
    list: bool,
    search: bool,
    release: bool,
    no_deps: bool,
    install_path: Option<PathBuf>,
    unknown: Vec<String>,
    pkg: Option<String>,
}

fn parse_bind_args(args: &[String]) -> BindQuickstartArgs {
    let mut parsed = BindQuickstartArgs::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--quickstart" => {
                parsed.quickstart = true;
                i += 1;
            }
            "--release" | "-r" => {
                parsed.release = true;
                i += 1;
            }
            "--no-deps" => {
                parsed.no_deps = true;
                i += 1;
            }
            "--list" | "-l" => {
                parsed.list = true;
                i += 1;
            }
            "--search" | "-s" => {
                parsed.search = true;
                i += 1;
            }
            "--install-path" | "-i" => {
                if i + 1 < args.len() {
                    parsed.install_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    parsed.unknown.push(args[i].clone());
                    i += 1;
                }
            }
            _ => {
                if parsed.pkg.is_none() && !args[i].starts_with('-') {
                    parsed.pkg = Some(args[i].clone());
                } else {
                    parsed.unknown.push(args[i].clone());
                }
                i += 1;
            }
        }
    }

    parsed
}

fn resolve_install_path(args: &BindQuickstartArgs) -> Result<PathBuf, String> {
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

fn cmd_quickstart(args: BindQuickstartArgs) -> ExitCode {
    let install_path = match resolve_install_path(&args) {
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

    if !args.unknown.is_empty() {
        eprintln!("rez bind --quickstart: ignoring extra args: {:?}", args.unknown);
    }

    let registry = bind_module::registry();
    let mut python_names = Vec::new();

    for module in &registry {
        if bind_module::package_already_installed(&install_path, &module.name) {
            println!("Skipping {} (already installed)", module.name);
            continue;
        }

        match module.strategy {
            BindStrategy::Native => {
                println!("Binding {} into {}...", module.name, install_path.display());
                if let Err(e) = bind_module::bind_one(&module.name, &install_path, args.no_deps) {
                    eprintln!("rez bind: {}", e);
                    return ExitCode::FAILURE;
                }
            }
            BindStrategy::Python => {
                python_names.push(module.name.clone());
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
