//! pkg CLI - Command-line interface for package management.
//!
//! # Commands (Rez-style)
//!
//! - `env` - Setup environment and run command
//! - `build` - Build a package
//! - `pip` - Import pip package into repo
//! - `search` - Search packages
//! - `view` - Show package details
//! - `depends` - Show dependencies

mod cli;
mod commands;
mod python;
mod shell;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Commands};
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
    match &command {
        Commands::Python { script, args } => {
            return python::cmd_python(script.clone(), args.clone(), cli.verbose > 0);
        }
        Commands::Completions { shell } => {
            return cmd_completions(shell.clone());
        }
        Commands::Config(args) => {
            debug!("cmd: rez config");
            return commands::cmd_rez_config(args);
        }
        Commands::Bind(args) => {
            debug!("cmd: rez bind");
            return commands::cmd_rez_bind(args);
        }
        Commands::Context(args) => {
            debug!("cmd: rez context");
            return commands::cmd_rez_context(args);
        }
        Commands::Status(args) => {
            debug!("cmd: rez status");
            return commands::cmd_rez_status(args);
        }
        Commands::Suite(args) => {
            debug!("cmd: rez suite");
            return commands::cmd_rez_suite(args);
        }
        Commands::Help => {
            print_usage();
            return ExitCode::SUCCESS;
        }
        _ => {}
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
                args.save_context,
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
        Commands::Search(args) => {
            debug!("cmd: search patterns={:?}", args.patterns);
            commands::cmd_rez_search(&storage, &args)
        }
        Commands::View(args) => {
            debug!("cmd: view package={}", args.package);
            commands::cmd_rez_view(&storage, &args)
        }
        Commands::Depends(args) => {
            debug!("cmd: depends packages={:?}", args.packages);
            commands::cmd_rez_depends(&storage, &args)
        }
        Commands::Bind(args) => commands::cmd_rez_bind(&args),
        Commands::Config(args) => commands::cmd_rez_config(&args),
        Commands::Context(args) => commands::cmd_rez_context(&args),
        Commands::Cp(args) => commands::cmd_rez_cp(&storage, &args),
        Commands::Status(args) => commands::cmd_rez_status(&args),
        Commands::Suite(args) => commands::cmd_rez_suite(&args),
        Commands::Diff(args) => commands::cmd_rez_diff(&storage, &args),
        Commands::Gui => {
            debug!("cmd: rez gui");
            match pkg_lib::gui::PkgApp::run(storage) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("GUI error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Shell => {
            debug!("cmd: shell");
            shell::cmd_shell(storage)
        }
        Commands::Help => {
            print_usage();
            ExitCode::SUCCESS
        }
        Commands::Interpret(args) => commands::cmd_rez_interpret(&args),
        Commands::Memcache(args) => commands::cmd_rez_memcache(&args),
        Commands::PkgCache(args) => commands::cmd_rez_pkg_cache(&args),
        Commands::Plugins(args) => commands::cmd_rez_plugins(&storage, &args),
        Commands::Python { script, args } => python::cmd_python(script, args, cli.verbose > 0),
        Commands::Release(args) => commands::cmd_rez_release(&storage, &args),
        Commands::Selftest(args) => commands::cmd_rez_selftest(&storage, &args),
        Commands::Test(args) => commands::cmd_rez_test(&storage, &args),
        Commands::Yaml2py(args) => commands::cmd_rez_yaml2py(&args),
        Commands::Bundle(args) => commands::cmd_rez_bundle(&storage, &args),
        Commands::Benchmark(args) => commands::cmd_rez_benchmark(&args),
        Commands::PkgIgnore(args) => commands::cmd_rez_pkg_ignore(&args),
        Commands::Mv(args) => commands::cmd_rez_mv(&storage, &args),
        Commands::Rm(args) => commands::cmd_rez_rm(&args),
        Commands::Version => {
            println!("pkg {}", pkg_lib::VERSION);
            ExitCode::SUCCESS
        }
        Commands::Completions { shell } => cmd_completions(shell),
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
