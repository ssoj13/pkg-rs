//! Pip import command.

use pkg_lib::pip::{import_pip_package, PipOptions};
use pkg_lib::Storage;
use std::path::PathBuf;
use std::process::ExitCode;

/// Import a pip package into a repository layout.
pub fn cmd_pip(
    storage: &Storage,
    package: String,
    python_version: Option<String>,
    no_deps: bool,
    min_deps: bool,
    install: bool,
    release: bool,
    prefix: Option<PathBuf>,
    extra: Option<String>,
    extra_args: Vec<String>,
) -> ExitCode {
    if !install {
        eprintln!("Expected one of: --install");
        return ExitCode::FAILURE;
    }

    let mut merged_extra = parse_args(extra);
    merged_extra.extend(extra_args);

    let install_mode = if no_deps {
        pkg_lib::pip::PipInstallMode::NoDeps
    } else if min_deps {
        pkg_lib::pip::PipInstallMode::MinDeps
    } else {
        pkg_lib::pip::PipInstallMode::MinDeps
    };

    let options = PipOptions {
        python_version,
        install,
        release,
        prefix,
        extra_args: merged_extra,
        install_mode,
    };

    match import_pip_package(storage, &package, &options) {
        Ok(report) => {
            println!("Pip package imported:");
            println!("  name: {}", report.name);
            println!("  version: {}", report.version);
            println!("  python: {}", report.python);
            println!("  install: {}", report.install_path.display());
            if !report.entry_points.is_empty() {
                println!("  entry_points: {}", report.entry_points.join(", "));
            }
            if !report.requirements.is_empty() {
                println!("  requirements: {}", report.requirements.join(", "));
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Pip import failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn parse_args(args: Option<String>) -> Vec<String> {
    let Some(args) = args else { return Vec::new() };
    match shell_words::split(&args) {
        Ok(split) => split,
        Err(_) => args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
    }
}
