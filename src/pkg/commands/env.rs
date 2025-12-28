//! Environment command.

use pkg_lib::{Package, Storage};
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// Setup environment for package(s) and optionally run command.
/// 
/// Two modes:
/// - Print mode: output env vars to stdout or file
/// - Run mode: apply env and execute command after --
pub fn cmd_env(
    storage: &Storage,
    packages: Vec<String>,
    command: Vec<String>,
    env_name: Option<String>,
    format: &str,
    expand: bool,
    output: Option<PathBuf>,
    dry_run: bool,
    stamp: bool,
    verbose: bool,
) -> ExitCode {
    if packages.is_empty() {
        eprintln!("No packages specified");
        return ExitCode::FAILURE;
    }

    // Build effective package (single or ad-hoc toolset)
    let mut pkg = if packages.len() == 1 {
        let name = &packages[0];
        match storage.resolve(name) {
            Some(p) => p.clone(),
            None => {
                eprintln!("Package not found: {}", name);
                return ExitCode::FAILURE;
            }
        }
    } else {
        // Multiple packages - create ad-hoc toolset
        let mut adhoc = Package::new("_adhoc".to_string(), "0.0.0".to_string());
        for name in &packages {
            adhoc.add_req(name.clone());
        }
        adhoc
    };

    // Solve dependencies
    if !pkg.reqs.is_empty() {
        if let Err(e) = pkg.solve(storage.packages()) {
            eprintln!("Failed to solve dependencies: {}", e);
            return ExitCode::FAILURE;
        }
    }

    let env_name_ref = env_name.as_deref().unwrap_or("default");
    let env = pkg._env(env_name_ref, true).or_else(|| pkg.default_env());
    let Some(mut env) = env else {
        eprintln!("Environment not found: {}", env_name_ref);
        return ExitCode::FAILURE;
    };

    // Add PKG_* stamp variables for each resolved package
    if stamp {
        // Stamp the main package
        for evar in pkg.stamp() {
            env.add(evar);
        }
        // Stamp all dependencies
        for dep in &pkg.deps {
            for evar in dep.stamp() {
                env.add(evar);
            }
        }
    }

    // Expand {TOKEN} references if requested
    if expand {
        match env.solve_impl(10, true) {
            Ok(solved) => env = solved,
            Err(e) => {
                eprintln!("Failed to solve environment: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Run mode: execute command with environment
    if !command.is_empty() {
        return run_with_env(&pkg, &env, &command, dry_run, verbose);
    }

    // Print mode: output environment
    let output_str = generate_env_output(&env, format);
    print!("{}", output_str);
    
    // Write to file if -o specified
    if let Some(path) = output {
        let file_content = generate_env_script(&env, &path);
        if let Err(e) = std::fs::write(&path, &file_content) {
            eprintln!("Failed to write {}: {}", path.display(), e);
            return ExitCode::FAILURE;
        }
        eprintln!("Written to: {}", path.display());
    }

    ExitCode::SUCCESS
}

/// Run command with environment applied.
fn run_with_env(
    pkg: &Package,
    env: &pkg_lib::Env,
    command: &[String],
    dry_run: bool,
    verbose: bool,
) -> ExitCode {
    let (exe_path, args) = if command.is_empty() {
        // No command: use package's default app
        let app = pkg._app(&pkg.base, true).or_else(|| pkg.default_app());
        let Some(app) = app else {
            eprintln!("No application found. Specify command after --");
            return ExitCode::FAILURE;
        };
        let Some(path) = &app.path else {
            eprintln!("No executable path for app: {}", app.name);
            return ExitCode::FAILURE;
        };
        (path.clone(), app.build_args(None))
    } else {
        (command[0].clone(), command[1..].to_vec())
    };

    if dry_run || verbose {
        println!("Environment:");
        for evar in env.evars_sorted() {
            println!("  {}={}", evar.name, evar.value);
        }
    }

    if dry_run {
        println!("\nWould run: {} {:?}", exe_path, args);
        return ExitCode::SUCCESS;
    }

    // Apply environment
    env.commit();

    if verbose {
        println!("Launching: {} {:?}", exe_path, args);
    }

    // Launch process
    let mut cmd = Command::new(&exe_path);
    cmd.args(&args);

    match cmd.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => {
                if status.success() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(status.code().unwrap_or(1) as u8)
                }
            }
            Err(e) => {
                eprintln!("Failed to wait for process: {}", e);
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("Failed to launch {}: {}", exe_path, e);
            ExitCode::FAILURE
        }
    }
}

/// Generate env output for display.
fn generate_env_output(env: &pkg_lib::Env, format: &str) -> String {
    let mut out = String::new();
    match format {
        "json" => {
            out = env.to_json().unwrap_or_default();
        }
        "export" => {
            for evar in env.evars_sorted() {
                out.push_str(&format!("export {}=\"{}\"\n", evar.name, evar.value));
            }
        }
        "set" => {
            for evar in env.evars_sorted() {
                out.push_str(&format!("set {}={}\n", evar.name, evar.value));
            }
        }
        _ => {
            for evar in env.evars_sorted() {
                out.push_str(&format!("{}={}\n", evar.name, evar.value));
            }
        }
    }
    out
}

/// Generate platform-specific script based on file extension.
fn generate_env_script(env: &pkg_lib::Env, path: &std::path::Path) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut out = String::new();
    
    match ext {
        "cmd" | "bat" => {
            out.push_str("@echo off\n");
            out.push_str(&format!("REM Environment for {}\n", env.name));
            out.push_str(&format!("REM Generated by pkg v{}\n\n", pkg_lib::VERSION));
            for evar in env.evars_sorted() {
                let action = evar.action();
                match action {
                    "append" => {
                        out.push_str(&format!("set {}=%{}%;{}\n", evar.name, evar.name, evar.value));
                    }
                    "insert" => {
                        out.push_str(&format!("set {}={};%{}%\n", evar.name, evar.value, evar.name));
                    }
                    _ => {
                        out.push_str(&format!("set {}={}\n", evar.name, evar.value));
                    }
                }
            }
        }
        "ps1" => {
            out.push_str(&format!("# Environment for {}\n", env.name));
            out.push_str(&format!("# Generated by pkg v{}\n\n", pkg_lib::VERSION));
            for evar in env.evars_sorted() {
                let action = evar.action();
                match action {
                    "append" => {
                        out.push_str(&format!("$env:{} = \"$env:{};{}\"\n", evar.name, evar.name, evar.value));
                    }
                    "insert" => {
                        out.push_str(&format!("$env:{} = \"{};$env:{}\"\n", evar.name, evar.value, evar.name));
                    }
                    _ => {
                        out.push_str(&format!("$env:{} = \"{}\"\n", evar.name, evar.value));
                    }
                }
            }
        }
        _ => {
            out.push_str("#!/bin/bash\n");
            out.push_str(&format!("# Environment for {}\n", env.name));
            out.push_str(&format!("# Generated by pkg v{}\n\n", pkg_lib::VERSION));
            for evar in env.evars_sorted() {
                let action = evar.action();
                match action {
                    "append" => {
                        out.push_str(&format!("export {}=\"${}:{}\"\n", evar.name, evar.name, evar.value));
                    }
                    "insert" => {
                        out.push_str(&format!("export {}=\"{}:${}\"\n", evar.name, evar.value, evar.name));
                    }
                    _ => {
                        out.push_str(&format!("export {}=\"{}\"\n", evar.name, evar.value));
                    }
                }
            }
        }
    }
    out
}
