//! Rez release command.

use crate::cli::ReleaseArgs;
use pkg_lib::build::{build_package, BuildOptions, BuildType};
use pkg_lib::config;
use pkg_lib::{Loader, Storage};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_release(storage: &Storage, args: &ReleaseArgs) -> ExitCode {
    let package_path = PathBuf::from("package.py");
    if !package_path.exists() {
        eprintln!("package.py not found in current directory");
        return ExitCode::FAILURE;
    }

    let mut loader = Loader::new(Some(false));
    let mut pkg = match loader.load_path(&package_path) {
        Ok(pkg) => pkg,
        Err(e) => {
            eprintln!("Failed to load package.py: {}", e);
            return ExitCode::FAILURE;
        }
    };
    pkg.package_source = Some(package_path.to_string_lossy().to_string());

    let release_repo = match resolve_release_repo(storage, args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez release: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if !args.no_latest {
        if let Err(err) = ensure_latest_release(&pkg, &release_repo) {
            eprintln!("rez release: {}", err);
            return ExitCode::FAILURE;
        }
    }

    let vcs = match resolve_vcs(args, args.skip_repo_errors) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("rez release: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if let Some(vcs) = &vcs {
        if let Err(err) = vcs.validate_tag(&pkg, args.ignore_existing_tag) {
            if args.skip_repo_errors {
                eprintln!("rez release: warning: {}", err);
            } else {
                eprintln!("rez release: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    let mut merged_build_args = pkg.build_args.clone();
    merged_build_args.extend(parse_args(args.build_args.clone()));

    let build_type = BuildType::Central;
    let options = BuildOptions {
        build_system: args.build_system.clone(),
        build_args: merged_build_args,
        child_build_args: parse_args(args.child_build_args.clone()),
        variants: args.variants.clone(),
        clean: false,
        install: true,
        prefix: Some(release_repo.clone()),
        scripts: false,
        build_type,
        extra_args: args.extra_args.clone(),
    };

    let report = match build_package(&pkg, &package_path, storage, &options) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("rez release: build failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if let Some(vcs) = &vcs {
        if let Err(err) = vcs.create_tag(&pkg, args.message.as_deref()) {
            if args.skip_repo_errors {
                eprintln!("rez release: warning: {}", err);
            } else {
                eprintln!("rez release: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    println!("Release complete: {} variant(s)", report.built_variants);
    if let Some(path) = report.install_path {
        println!("Installed to: {}", path.display());
    }

    ExitCode::SUCCESS
}

fn parse_args(args: Option<String>) -> Vec<String> {
    let Some(args) = args else { return Vec::new() };
    match shell_words::split(&args) {
        Ok(split) => split,
        Err(_) => args.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

fn resolve_release_repo(storage: &Storage, _args: &ReleaseArgs) -> Result<PathBuf, String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    if let Some(path) = config::release_packages_path(cfg) {
        return Ok(path);
    }
    if let Some(first) = storage.location_paths().first() {
        return Ok(first.clone());
    }
    Err("release_packages_path is not set; configure in rezconfig.py".to_string())
}

fn ensure_latest_release(pkg: &pkg_lib::Package, release_repo: &Path) -> Result<(), String> {
    let repo = Storage::scan_paths(vec![release_repo.to_string_lossy().to_string()])
        .map_err(|e| e.to_string())?;
    let versions = repo.versions(&pkg.base);
    if let Some(latest) = versions.first() {
        if latest != &pkg.name {
            return Err(format!("{} is not the latest release (latest is {})", pkg.name, latest));
        }
    }
    Ok(())
}

struct GitVcs {
    root: PathBuf,
}

impl GitVcs {
    fn validate_tag(&self, pkg: &pkg_lib::Package, ignore_existing: bool) -> Result<(), String> {
        let tag = tag_name(pkg)?;
        if ignore_existing {
            return Ok(());
        }
        let output = std::process::Command::new("git")
            .args(["tag", "--list", &tag])
            .current_dir(&self.root)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.stdout.is_empty() {
            return Err(format!("tag already exists: {}", tag));
        }
        Ok(())
    }

    fn create_tag(&self, pkg: &pkg_lib::Package, message: Option<&str>) -> Result<(), String> {
        let tag = tag_name(pkg)?;
        let mut cmd = std::process::Command::new("git");
        cmd.arg("tag");
        if let Some(msg) = message {
            cmd.args(["-a", &tag, "-m", msg]);
        } else {
            cmd.arg(&tag);
        }
        let status = cmd.current_dir(&self.root).status().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("failed to create git tag".to_string())
        }
    }
}

fn resolve_vcs(args: &ReleaseArgs, skip_errors: bool) -> Result<Option<GitVcs>, String> {
    let Some(vcs) = args.vcs.as_deref() else {
        if let Some(vcs) = find_git_vcs() {
            return Ok(Some(vcs));
        }
        if skip_errors {
            return Ok(None);
        }
        return Err("git repo not found".to_string());
    };
    if !vcs.eq_ignore_ascii_case("git") {
        return Err(format!("unsupported vcs: {}", vcs));
    }
    find_git_vcs().ok_or_else(|| "git repo not found".to_string()).map(Some)
}

fn find_git_vcs() -> Option<GitVcs> {
    let cwd = std::env::current_dir().ok()?;
    let mut current = cwd.as_path();
    loop {
        if current.join(".git").exists() {
            return Some(GitVcs { root: current.to_path_buf() });
        }
        current = current.parent()?;
    }
}

fn tag_name(pkg: &pkg_lib::Package) -> Result<String, String> {
    let template = config::get().ok()
        .and_then(|cfg| config::get_str(cfg, "plugins.release_vcs.tag_name"))
        .unwrap_or_else(|| "{qualified_name}".to_string());

    let qualified = pkg.name.clone();
    Ok(template
        .replace("{qualified_name}", &qualified)
        .replace("{name}", &pkg.base)
        .replace("{version}", &pkg.version))
}
