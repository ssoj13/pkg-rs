//! Shell command implementations.

use crate::commands::matches_glob;
use pkg_lib::{SolveStatus, Storage};
use std::process::Command;

/// Show shell help.
pub fn shell_help() {
    println!(
        r#"
Commands:
  list, ls [patterns...]      List packages (glob: maya, cinem*)
  info <package>              Show package details
  run [-f] <package> [app]    Launch app (-f: skip solve check)
  env <package>               Show environment
  solve <package>             Resolve dependencies
  scan                        Rescan locations
  help, ?                     This help
  exit, quit, q               Exit
"#
    );
}

/// List packages in shell (glob patterns).
pub fn shell_list(storage: &Storage, args: &[&str]) {
    let patterns: Vec<&str> = args.iter().copied().collect();
    
    let packages: Vec<_> = if patterns.is_empty() {
        storage.packages()
    } else {
        storage
            .packages()
            .into_iter()
            .filter(|pkg| {
                patterns.iter().any(|pat| {
                    matches_glob(pat, &pkg.base) || matches_glob(pat, &pkg.name)
                })
            })
            .collect()
    };

    if packages.is_empty() {
        println!("No packages found.");
        return;
    }

    println!("Packages ({}):", packages.len());
    for pkg in &packages {
        let tags = if pkg.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", pkg.tags.join(", "))
        };
        println!("  {}{}", pkg.name, tags);
    }
}

/// Show package info in shell.
pub fn shell_info(storage: &Storage, args: &[&str]) {
    if args.is_empty() {
        eprintln!("Usage: info <package>");
        return;
    }

    let pkg = storage.resolve(args[0]);
    let Some(pkg) = pkg else {
        eprintln!("Package not found: {}", args[0]);
        return;
    };

    println!("Package: {}", pkg.name);
    println!("  Base: {}", pkg.base);
    println!("  Version: {}", pkg.version);

    if !pkg.tags.is_empty() {
        println!("  Tags: {}", pkg.tags.join(", "));
    }
    if !pkg.reqs.is_empty() {
        println!("  Requirements:");
        for req in &pkg.reqs {
            println!("    - {}", req);
        }
    }
    if !pkg.envs.is_empty() {
        println!("  Environments:");
        for env in &pkg.envs {
            println!("    - {} ({} vars)", env.name, env.evars.len());
        }
    }
    if !pkg.apps.is_empty() {
        println!("  Applications:");
        for app in &pkg.apps {
            println!(
                "    - {}: {}",
                app.name,
                app.path.as_deref().unwrap_or("(no path)")
            );
        }
    }
}

/// Run application in shell.
pub fn shell_run(storage: &Storage, args: &[&str]) {
    if args.is_empty() {
        eprintln!("Usage: run [-f] <package> [app] [-- args...]");
        return;
    }

    let force = args.iter().any(|a| *a == "-f" || *a == "--force");
    let args: Vec<&str> = args
        .iter()
        .filter(|a| **a != "-f" && **a != "--force")
        .copied()
        .collect();

    if args.is_empty() {
        eprintln!("Usage: run [-f] <package> [app] [-- args...]");
        return;
    }

    let pkg = storage.resolve(args[0]);
    let Some(pkg) = pkg else {
        eprintln!("Package not found: {}", args[0]);
        return;
    };

    // Check solve status
    if !force && !pkg.reqs.is_empty() {
        match pkg.status() {
            SolveStatus::NotSolved => {
                eprintln!(
                    "Error: Unresolved dependencies. Run 'solve {}' or use -f",
                    pkg.name
                );
                return;
            }
            SolveStatus::Failed => {
                eprintln!("Error: Failed to resolve. Use -f to run anyway.");
                if let Some(err) = &pkg.solve_error {
                    eprintln!("  {}", err);
                }
                return;
            }
            SolveStatus::Solved => {}
        }
    }

    let app_name = args.get(1).copied();
    let app = match app_name {
        Some(name) if name != "--" => pkg._app(name, true),
        _ => pkg.default_app(),
    };

    let Some(app) = app else {
        eprintln!("No app found. Available: {:?}", pkg.app_names());
        return;
    };

    let extra_args: Vec<String> = args
        .iter()
        .skip_while(|a| **a != "--")
        .skip(1)
        .map(|s| s.to_string())
        .collect();

    let env_name = app.env_name.as_deref().unwrap_or("default");
    let env = pkg._env(env_name, true).or_else(|| pkg.default_env());

    let Some(exe_path) = &app.path else {
        eprintln!("No executable path for: {}", app.name);
        return;
    };

    let mut cmd = Command::new(exe_path);

    if let Some(env) = env {
        if let Ok(solved) = env.solve_impl(10, true) {
            for evar in &solved.evars {
                cmd.env(&evar.name, &evar.value);
            }
        }
    }

    let all_args = app.build_args(if extra_args.is_empty() {
        None
    } else {
        Some(extra_args)
    });
    cmd.args(&all_args);

    if let Some(cwd) = app.effective_cwd() {
        cmd.current_dir(cwd);
    }

    println!("Launching: {} {:?}", exe_path, all_args);

    match cmd.spawn() {
        Ok(_) => println!("Started."),
        Err(e) => eprintln!("Failed: {}", e),
    }
}

/// Show environment in shell.
pub fn shell_env(storage: &Storage, args: &[&str]) {
    if args.is_empty() {
        eprintln!("Usage: env <package> [app]");
        return;
    }

    let pkg = storage.resolve(args[0]);
    let Some(pkg) = pkg else {
        eprintln!("Package not found: {}", args[0]);
        return;
    };

    let app_name = args.get(1).copied();
    match pkg.effective_env(app_name) {
        Ok(Some(env)) => {
            println!("Environment for {}:", pkg.name);
            for evar in env.evars_sorted() {
                println!("  {}={}", evar.name, evar.value);
            }
        }
        Ok(None) => println!("No environment defined."),
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Solve dependencies in shell.
pub fn shell_solve(storage: &Storage, args: &[&str]) {
    if args.is_empty() {
        eprintln!("Usage: solve <package>");
        return;
    }

    let pkg = storage.resolve(args[0]);
    let Some(mut pkg) = pkg else {
        eprintln!("Package not found: {}", args[0]);
        return;
    };

    if pkg.reqs.is_empty() {
        println!("{} has no dependencies.", pkg.name);
        return;
    }

    println!("Requirements:");
    for req in &pkg.reqs {
        println!("  - {}", req);
    }

    match pkg.solve(storage.packages()) {
        Ok(()) => {
            println!("\nResolved:");
            for dep in &pkg.deps {
                println!("  - {}", dep.name);
            }
        }
        Err(e) => eprintln!("\nResolution failed: {}", e),
    }
}
