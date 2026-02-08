//! rez pkg-ignore command (native).

use crate::cli::PkgIgnoreArgs;
use pkg_lib::config;
use pkg_lib::repo_ops;
use pkg_lib::Package;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn cmd_rez_pkg_ignore(args: &PkgIgnoreArgs) -> ExitCode {
    let (base, version) = match Package::parse_name(&args.pkg) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("rez pkg-ignore: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if args.path.is_none() {
        if args.allow_missing {
            if let Err(err) = list_repos() {
                eprintln!("rez pkg-ignore: {}", err);
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }

        let repos = match repos_containing_pkg(&base, &version) {
            Ok(repos) => repos,
            Err(err) => {
                eprintln!("rez pkg-ignore: {}", err);
                return ExitCode::FAILURE;
            }
        };
        if repos.is_empty() {
            eprintln!("Package not found");
            return ExitCode::FAILURE;
        }
        if let Err(err) = list_repos_containing_pkg(&repos) {
            eprintln!("rez pkg-ignore: {}", err);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    let repo = match resolve_repo(args, &base, &version) {
        Ok(repo) => repo,
        Err(err) => {
            eprintln!("rez pkg-ignore: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let result = if args.unignore {
        repo_ops::unignore_package(&repo, &base, &version)
    } else {
        repo_ops::ignore_package(&repo, &base, &version, args.allow_missing)
    };

    match result {
        Ok(1) => {
            if args.unignore {
                println!("Package is now visible to resolves once more");
            } else {
                println!("Package is now ignored and will not be visible to resolves");
            }
            ExitCode::SUCCESS
        }
        Ok(0) => {
            if args.unignore {
                println!("No action taken - package was already visible");
            } else {
                println!("No action taken - package was already ignored");
            }
            ExitCode::SUCCESS
        }
        Ok(-1) => {
            eprintln!("Package not found");
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("rez pkg-ignore: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn resolve_repo(args: &PkgIgnoreArgs, base: &str, version: &str) -> Result<PathBuf, String> {
    if let Some(path) = args.path.clone() {
        return Ok(path);
    }

    let cfg = config::get().map_err(|e| e.to_string())?;
    for path in config::packages_path(cfg) {
        if repo_ops::package_exists(&path, base, version) {
            return Ok(path);
        }
    }

    Err("Package not found in any configured repository.".to_string())
}

fn list_repos() -> Result<(), String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    println!("No action taken. Run again, and set PATH to one of:");
    for path in config::packages_path(cfg) {
        println!("{}", path.display());
    }
    Ok(())
}

fn list_repos_containing_pkg(repos: &[PathBuf]) -> Result<(), String> {
    println!("No action taken. Run again, and set PATH to one of:");
    for repo in repos {
        println!("{}", repo.display());
    }
    Ok(())
}

fn repos_containing_pkg(base: &str, version: &str) -> Result<Vec<PathBuf>, String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    let mut matching = Vec::new();
    for path in config::packages_path(cfg) {
        if repo_ops::package_exists(&path, base, version) {
            matching.push(path);
        }
    }
    Ok(matching)
}
