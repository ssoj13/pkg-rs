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
use cli::{Cli, Commands, RezCommands};
use log::{debug, info, trace};
use pkg_lib::{config, Storage};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(err) = config::init(cli.cfg.clone()) {
        eprintln!("Config error: {}", err);
        return ExitCode::FAILURE;
    }

    // Initialize logging
    init_logging(cli.verbose, &cli.log_file);

    info!("pkg v{} starting", pkg_lib::VERSION);
    trace!("CLI args: repos={:?}, exclude={:?}", cli.repos, cli.exclude);

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
    if let Commands::Rez(RezCommands::Config(args)) = &command {
        debug!("cmd: rez config");
        return commands::cmd_rez_config(args);
    }
    if let Commands::Rez(RezCommands::Bind(args)) = &command {
        debug!("cmd: rez bind");
        return commands::cmd_rez_bind(args);
    }
    if let Commands::Rez(RezCommands::Context(args)) = &command {
        debug!("cmd: rez context");
        return commands::cmd_rez_context(args);
    }
    if let Commands::Rez(RezCommands::Status(args)) = &command {
        debug!("cmd: rez status");
        return commands::cmd_rez_status(args);
    }
    if let Commands::Rez(RezCommands::Suite(args)) = &command {
        debug!("cmd: rez suite");
        return commands::cmd_rez_suite(args);
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
        Commands::Env(args) => {
            debug!(
                "cmd: env packages={:?} command={:?} env_name={:?}",
                args.packages, args.command, args.env_name
            );
            commands::cmd_env(
                &storage,
                args.packages,
                args.command,
                args.env_name,
                &args.format,
                args.expand,
                args.output,
                args.dry_run,
                args.stamp,
                cli.verbose > 0,
            )
        }
        Commands::Build(args) => {
            debug!("cmd: build");
            commands::cmd_build(
                &storage,
                args.build_system,
                args.process,
                args.build_args,
                args.child_build_args,
                args.variants,
                args.clean,
                args.install,
                args.prefix,
                args.scripts,
                args.view_pre,
                args.extra_args,
            )
        }
        Commands::BuildEnv {
            build_path,
            variant_index,
            install,
            install_path,
        } => commands::cmd_build_env(build_path, variant_index, install, install_path),
        Commands::Pip(args) => {
            debug!("cmd: pip package={}", args.package);
            commands::cmd_pip(
                &storage,
                args.package,
                args.python_version,
                args.no_deps,
                args.min_deps,
                args.install,
                args.release,
                args.prefix,
                args.extra,
                args.extra_args,
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
        Commands::Rez(rez_cmd) => match rez_cmd {
            RezCommands::Env(args) => {
                debug!(
                    "cmd: rez env packages={:?} command={:?} env_name={:?}",
                    args.packages, args.command, args.env_name
                );
                commands::cmd_env(
                    &storage,
                    args.packages,
                    args.command,
                    args.env_name,
                    &args.format,
                    args.expand,
                    args.output,
                    args.dry_run,
                    args.stamp,
                    cli.verbose > 0,
                )
            }
            RezCommands::Build(args) => {
                debug!("cmd: rez build");
                commands::cmd_build(
                    &storage,
                    args.build_system,
                    args.process,
                    args.build_args,
                    args.child_build_args,
                    args.variants,
                    args.clean,
                    args.install,
                    args.prefix,
                    args.scripts,
                    args.view_pre,
                    args.extra_args,
                )
            }
            RezCommands::Pip(args) => {
                debug!("cmd: rez pip package={}", args.package);
                commands::cmd_pip(
                    &storage,
                    args.package,
                    args.python_version,
                    args.no_deps,
                    args.min_deps,
                    args.install,
                    args.release,
                    args.prefix,
                    args.extra,
                    args.extra_args,
                )
            }
            RezCommands::Bind(args) => commands::cmd_rez_bind(&args),
            RezCommands::Config(args) => commands::cmd_rez_config(&args),
            RezCommands::Context(args) => commands::cmd_rez_context(&args),
            RezCommands::Cp(args) => cmd_rez_stub("rez cp", args.args),
            RezCommands::Depends(args) => cmd_rez_stub("rez depends", args.args),
            RezCommands::Diff(args) => cmd_rez_stub("rez diff", args.args),
            RezCommands::Gui(args) => cmd_rez_stub("rez gui", args.args),
            RezCommands::Help(args) => cmd_rez_stub("rez help", args.args),
            RezCommands::Interpret(args) => cmd_rez_stub("rez interpret", args.args),
            RezCommands::Memcache(args) => cmd_rez_stub("rez memcache", args.args),
            RezCommands::PkgCache(args) => cmd_rez_stub("rez pkg-cache", args.args),
            RezCommands::Plugins(args) => cmd_rez_stub("rez plugins", args.args),
            RezCommands::Python(args) => cmd_rez_stub("rez python", args.args),
            RezCommands::Release(args) => cmd_rez_stub("rez release", args.args),
            RezCommands::Search(args) => cmd_rez_stub("rez search", args.args),
            RezCommands::Selftest(args) => cmd_rez_stub("rez selftest", args.args),
            RezCommands::Status(args) => commands::cmd_rez_status(&args),
            RezCommands::Suite(args) => commands::cmd_rez_suite(&args),
            RezCommands::Test(args) => cmd_rez_stub("rez test", args.args),
            RezCommands::View(args) => cmd_rez_stub("rez view", args.args),
            RezCommands::Yaml2py(args) => cmd_rez_stub("rez yaml2py", args.args),
            RezCommands::Bundle(args) => cmd_rez_stub("rez bundle", args.args),
            RezCommands::Benchmark(args) => cmd_rez_stub("rez benchmark", args.args),
            RezCommands::PkgIgnore(args) => cmd_rez_stub("rez pkg-ignore", args.args),
            RezCommands::Mv(args) => cmd_rez_stub("rez mv", args.args),
            RezCommands::Rm(args) => cmd_rez_stub("rez rm", args.args),
            RezCommands::Complete(args) => cmd_rez_stub("rez _rez-complete", args.args),
            RezCommands::Forward(args) => cmd_rez_stub("rez _rez_fwd", args.args),
        },
        Commands::Version => {
            println!("pkg {}", pkg_lib::VERSION);
            ExitCode::SUCCESS
        }
        Commands::Shell => {
            debug!("cmd: shell");
            shell::cmd_shell(storage)
        }
        Commands::Gui => {
            debug!("cmd: gui");
            match pkg_lib::gui::PkgApp::run(storage) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("GUI error: {}", e);
                    ExitCode::FAILURE
                }
            }
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

/// Stub handler for rez parity commands not implemented yet.
fn cmd_rez_stub(cmd: &str, args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        eprintln!("Rez parity: '{}' is not implemented yet.", cmd);
    } else {
        eprintln!(
            "Rez parity: '{}' is not implemented yet. Args: {:?}",
            cmd, args
        );
    }
    ExitCode::FAILURE
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
    let config = config::get().map_err(|e| e.to_string())?;

    // Add extra repos
    all_paths.extend(extra_repos.iter().cloned());

    // Add defaults if no explicit repos
    if extra_repos.is_empty() {
        all_paths.extend(crate::config::packages_path(config));
    }

    if user_packages {
        if let Some(user_dir) = Storage::user_packages_dir() {
            if user_dir.exists() {
                debug!("Adding user packages: {}", user_dir.display());
                all_paths.push(user_dir);
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
