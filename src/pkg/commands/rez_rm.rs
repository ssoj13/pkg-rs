//! rez rm command (native).

use crate::cli::RmArgs;
use pkg_lib::config;
use pkg_lib::repo_ops;
use pkg_lib::Package;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn cmd_rez_rm(args: &RmArgs) -> ExitCode {
    if args.package.is_some() {
        return remove_package(args);
    }
    if args.family.is_some() || args.force_family {
        return remove_family(args);
    }
    if args.ignored_since.is_some() {
        return remove_ignored(args);
    }

    eprintln!("rez rm: Must specify either --package or --ignored-since");
    ExitCode::FAILURE
}

fn remove_package(args: &RmArgs) -> ExitCode {
    if args.dry_run {
        eprintln!("rez rm: --dry-run is not supported with --package");
        return ExitCode::FAILURE;
    }

    let Some(pkg) = args.package.as_deref() else {
        eprintln!("rez rm: --package is required");
        return ExitCode::FAILURE;
    };
    let Some(repo) = args.path.as_ref() else {
        eprintln!("rez rm: Must specify PATH with --package");
        return ExitCode::FAILURE;
    };

    let (base, version) = match Package::parse_name(pkg) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("rez rm: {}", err);
            return ExitCode::FAILURE;
        }
    };

    match repo_ops::remove_package(repo, &base, &version) {
        Ok(true) => {
            println!("Package removed.");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            eprintln!("Package not found.");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("rez rm: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn remove_family(args: &RmArgs) -> ExitCode {
    if args.dry_run {
        eprintln!("rez rm: --dry-run is not supported with --family");
        return ExitCode::FAILURE;
    }

    let name = args.family.as_deref().unwrap_or("");
    if name.is_empty() {
        eprintln!("rez rm: --family is required with --force-family");
        return ExitCode::FAILURE;
    }

    let Some(repo) = args.path.as_ref() else {
        eprintln!("rez rm: Must specify PATH with --family");
        return ExitCode::FAILURE;
    };

    match repo_ops::remove_package_family(repo, name, args.force_family) {
        Ok(true) => {
            println!("Package family removed.");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            eprintln!("Package family not found.");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("rez rm: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn remove_ignored(args: &RmArgs) -> ExitCode {
    let days = args.ignored_since.unwrap_or(0);
    let paths = match resolve_paths(args) {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("rez rm: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let mut total = 0usize;
    for repo in paths {
        match repo_ops::remove_ignored_since(&repo, days, args.dry_run, true) {
            Ok(count) => total += count,
            Err(err) => {
                eprintln!("rez rm: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    if total == 0 {
        println!("No packages were removed.");
    } else if args.dry_run {
        println!("{} packages would be removed.", total);
    } else {
        println!("{} packages were removed.", total);
    }

    ExitCode::SUCCESS
}

fn resolve_paths(args: &RmArgs) -> Result<Vec<PathBuf>, String> {
    if let Some(path) = args.path.clone() {
        return Ok(vec![path]);
    }

    let cfg = config::get().map_err(|e| e.to_string())?;
    let paths = config::packages_path(cfg);
    if paths.is_empty() {
        return Err("No configured package paths available".to_string());
    }
    Ok(paths)
}
