//! rez mv command (native).

use crate::cli::{CpArgs, MvArgs};
use crate::commands::rez_cp::{copy_package as copy_package_impl, package_root as pkg_root_impl, resolve_package as resolve_pkg_impl};
use pkg_lib::config;
use pkg_lib::repo_ops;
use pkg_lib::{Package, Storage};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_mv(_storage: &Storage, args: &MvArgs) -> ExitCode {
    let (base, version) = match Package::parse_name(&args.pkg) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("rez mv: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let src_repo = if let Some(path) = args.path.as_ref() {
        path.clone()
    } else {
        match list_repos_containing_pkg(&base, &version) {
            Ok(repos) => {
                if repos.is_empty() {
                    eprintln!("Package not found.");
                    return ExitCode::FAILURE;
                }
                println!("No action taken. Run again, and set PATH to one of:");
                for repo in repos {
                    println!("{}", repo.display());
                }
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("rez mv: {}", err);
                return ExitCode::FAILURE;
            }
        }
    };

    if !repo_ops::package_exists(&src_repo, &base, &version) {
        eprintln!("Package not found.");
        return ExitCode::FAILURE;
    }

    let pkg = match load_package_from_repo(&src_repo, &args.pkg) {
        Ok(pkg) => pkg,
        Err(err) => {
            eprintln!("rez mv: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let src_pkg_root = match pkg_root_impl(&pkg) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez mv: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if repo_ops::package_exists(&args.dest_path, &base, &version) {
        eprintln!("rez mv: package already exists at destination");
        return ExitCode::FAILURE;
    }

    let cp_args = CpArgs {
        dest_path: Some(args.dest_path.clone()),
        paths: None,
        no_local: false,
        reversion: None,
        rename: None,
        overwrite: false,
        shallow: false,
        follow_symlinks: false,
        keep_timestamp: args.keep_timestamp,
        force: args.force,
        allow_empty: true,
        dry_run: false,
        variants: Vec::new(),
        variant_uri: None,
        pkg: Some(args.pkg.clone()),
    };

    let dest_repo = args.dest_path.clone();

    if let Err(err) = repo_ops::ignore_package(&dest_repo, &base, &version, true) {
        eprintln!("rez mv: {}", err);
        return ExitCode::FAILURE;
    }

    let copy_result = match copy_package_impl(
        &pkg,
        &src_pkg_root,
        &dest_repo,
        &base,
        &version,
        &cp_args,
    ) {
        Ok(res) => res,
        Err(err) => {
            let _ = repo_ops::unignore_package(&dest_repo, &base, &version);
            eprintln!("rez mv: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let _ = repo_ops::unignore_package(&dest_repo, &base, &version);

    match repo_ops::ignore_package(&src_repo, &base, &version, false) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("rez mv: {}", err);
            return ExitCode::FAILURE;
        }
    }

    println!(
        "Package {} moved to {}",
        pkg.name,
        copy_result.dest_pkg_root.display()
    );

    ExitCode::SUCCESS
}

fn list_repos_containing_pkg(base: &str, version: &str) -> Result<Vec<PathBuf>, String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    let mut matches = Vec::new();
    for path in config::packages_path(cfg) {
        if repo_ops::package_exists(&path, base, version) {
            matches.push(path);
        }
    }
    Ok(matches)
}

fn load_package_from_repo(repo: &Path, pkg_req: &str) -> Result<Package, String> {
    let storage = Storage::scan_impl(Some(&[repo.to_path_buf()])).map_err(|e| e.to_string())?;
    resolve_pkg_impl(&storage, pkg_req)
}
