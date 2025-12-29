//! pkg CLI - Command-line interface for package management.
//!
//! # Commands
//!
//! - `ls` - List available packages
//! - `info <package>` - Show package details
//! - `env <packages> [-- cmd]` - Setup environment and run command
//! - `scan [paths...]` - Scan locations for packages
//! - `sh` - Interactive shell

mod cli;
mod commands;
mod python;
mod shell;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Commands};
use log::{debug, info, trace};
use pkg_lib::Storage;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, &cli.log_file);

    info!("pkg v{} starting", pkg_lib::VERSION);
    trace!("CLI args: repos={:?}, exclude={:?}", cli.repos, cli.exclude);

    // Launch GUI if requested
    if cli.gui {
        debug!("Launching GUI");
        let storage = match build_storage(&cli.repos, &cli.exclude, cli.user_packages) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        };
        return match pkg_lib::gui::PkgApp::run(storage) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("GUI error: {}", e);
                ExitCode::FAILURE
            }
        };
    }

    // Show help if no command
    let Some(command) = cli.command else {
        print_usage();
        return ExitCode::SUCCESS;
    };

    // Commands that don't need storage
    if let Commands::Python { script, args } = command {
        return python::cmd_python(script, args, cli.verbose > 0);
    }
    if let Commands::Completions { shell } = command {
        return cmd_completions(shell);
    }
    if let Commands::GenPkg { package_id } = command {
        debug!("cmd: gen-pkg package_id={}", package_id);
        return commands::cmd_gen_pkg(&package_id);
    }

    // Build storage with custom repos if provided
    debug!(
        "Building storage with {} extra repos, user_packages={}",
        cli.repos.len(),
        cli.user_packages
    );
    let storage = match build_storage(&cli.repos, &cli.exclude, cli.user_packages) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Storage error: {}", e);
            eprintln!("Error scanning packages: {}", e);
            return ExitCode::FAILURE;
        }
    };
    info!(
        "Loaded {} packages from {} locations",
        storage.count(),
        storage.locations().len()
    );

    // Log warnings
    for w in &storage.warnings {
        log::warn!("{}", w);
    }

    // Print warnings in verbose mode
    if cli.verbose > 0 && !storage.warnings.is_empty() {
        eprintln!("Warnings:");
        for w in &storage.warnings {
            eprintln!("  - {}", w);
        }
    }

    match command {
        Commands::List {
            patterns,
            tags,
            latest,
            json,
        } => {
            debug!("cmd: ls patterns={:?} tags={:?} latest={}", patterns, tags, latest);
            commands::cmd_list(&storage, patterns, tags, latest, json)
        }
        Commands::Info { package, json } => {
            debug!("cmd: info package={}", package);
            commands::cmd_info(&storage, &package, json)
        }
        Commands::Env {
            packages,
            command,
            env_name,
            format,
            expand,
            output,
            dry_run,
            stamp,
        } => {
            debug!(
                "cmd: env packages={:?} command={:?} env_name={:?}",
                packages, command, env_name
            );
            commands::cmd_env(
                &storage,
                packages,
                command,
                env_name,
                &format,
                expand,
                output,
                dry_run,
                stamp,
                cli.verbose > 0,
            )
        }
        Commands::Graph {
            packages,
            format,
            depth,
            reverse,
        } => {
            debug!(
                "cmd: graph packages={:?} format={} depth={} reverse={}",
                packages, format, depth, reverse
            );
            commands::cmd_graph(&storage, packages, &format, depth, reverse)
        }
        Commands::Scan { paths } => {
            debug!("cmd: scan paths={:?}", paths);
            commands::cmd_scan(&paths)
        }
        Commands::GenerateRepo {
            output,
            small,
            medium: _,
            large,
            stress,
            packages,
            versions,
            depth,
            dep_rate,
            seed,
        } => {
            // Resolve preset or custom values
            let (pkg_count, ver_count) = if small {
                (10, 2)
            } else if large {
                (200, 5)
            } else if stress {
                (1000, 10)
            } else {
                // medium is default
                (50, 3)
            };
            // Custom values override preset
            let pkg_count = packages.unwrap_or(pkg_count);
            let ver_count = versions.unwrap_or(ver_count);

            debug!(
                "cmd: gen-repo output={:?} packages={} versions={}",
                output, pkg_count, ver_count
            );
            commands::cmd_generate_repo(output, pkg_count, ver_count, depth, dep_rate, seed)
        }
        Commands::Version => {
            println!("pkg {}", pkg_lib::VERSION);
            ExitCode::SUCCESS
        }
        Commands::Shell => {
            debug!("cmd: shell");
            shell::cmd_shell(storage)
        }
        Commands::Python { .. } => unreachable!(),
        Commands::Completions { .. } => unreachable!(),
        Commands::GenPkg { .. } => unreachable!(),
    }
}

/// Initialize logging based on verbosity and optional log file.
fn init_logging(verbosity: u8, log_file: &Option<Option<PathBuf>>) {
    use std::io::Write;

    let level = match verbosity {
        0 => log::LevelFilter::Warn,  // default: warnings only
        1 => log::LevelFilter::Info,  // -v: info
        2 => log::LevelFilter::Debug, // -vv: debug
        _ => log::LevelFilter::Trace, // -vvv: trace
    };

    let mut builder = env_logger::Builder::new();
    builder.filter_level(level);
    builder.format(|buf, record| {
        writeln!(
            buf,
            "[{} {}] {}",
            record.level(),
            record.target(),
            record.args()
        )
    });

    // If log file requested
    if let Some(maybe_path) = log_file {
        let log_path = match maybe_path {
            Some(p) => p.clone(),
            None => {
                // Default: pkg.log next to binary
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("pkg.log")))
                    .unwrap_or_else(|| PathBuf::from("pkg.log"))
            }
        };

        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
            eprintln!("Logging to: {}", log_path.display());
        }
    }

    builder.init();
}

/// Print usage help.
fn print_usage() {
    // Use clap's auto-generated long help - includes examples
    Cli::command().print_long_help().unwrap();
}

/// Generate shell completions.
fn cmd_completions(shell: clap_complete::Shell) -> ExitCode {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "pkg", &mut std::io::stdout());
    ExitCode::SUCCESS
}

/// Build storage with optional custom repos, exclude patterns, and user packages.
fn build_storage(
    extra_repos: &[PathBuf],
    exclude: &[String],
    user_packages: bool,
) -> Result<Storage, String> {
    let mut all_paths = Vec::new();

    // Add user packages first (highest priority - overrides)
    if user_packages {
        if let Some(user_dir) = Storage::user_packages_dir() {
            if user_dir.exists() {
                debug!("Adding user packages: {}", user_dir.display());
                all_paths.push(user_dir);
            }
        }
    }

    // Add extra repos
    all_paths.extend(extra_repos.iter().cloned());

    // Add defaults if no explicit repos
    if extra_repos.is_empty() {
        if let Ok(default_storage) = Storage::scan_impl(None) {
            for loc in default_storage.locations() {
                all_paths.push(PathBuf::from(loc));
            }
        }
    }

    let mut storage = if all_paths.is_empty() {
        Storage::scan_impl(None).map_err(|e| e.to_string())?
    } else {
        Storage::scan_impl(Some(&all_paths)).map_err(|e| e.to_string())?
    };

    // Apply exclude patterns (filter out matching packages)
    if !exclude.is_empty() {
        storage.exclude_packages(exclude);
    }

    Ok(storage)
}
