//! rez cp command (native).

use crate::cli::CpArgs;
use filetime::{set_file_times, FileTime};
use pkg_lib::config;
use pkg_lib::dep::DepSpec;
use pkg_lib::repo_ops;
use pkg_lib::{Package, Storage};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone)]
struct VariantInfo {
    index: Option<usize>,
    subpath: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CopyResult {
    copied: Vec<(String, PathBuf)>,
    skipped: Vec<(String, PathBuf)>,
    pub(crate) dest_pkg_root: PathBuf,
    is_varianted: bool,
}

pub fn cmd_rez_cp(storage: &Storage, args: &CpArgs) -> ExitCode {
    if args.variant_uri.is_some() {
        eprintln!("rez cp: --variant-uri is not supported yet");
        return ExitCode::FAILURE;
    }

    let Some(pkg_req) = args.pkg.as_deref() else {
        eprintln!("rez cp: Expected PKG");
        return ExitCode::FAILURE;
    };

    let storage = match build_storage_override(storage, args) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("rez cp: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let pkg = match resolve_package(&storage, pkg_req) {
        Ok(pkg) => pkg,
        Err(err) => {
            eprintln!("rez cp: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let cfg = config::get().ok();
    let src_pkg_root = match package_root(&pkg) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez cp: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let src_repo = match find_repo_root(&storage, &src_pkg_root) {
        Some(path) => path,
        None => {
            eprintln!("rez cp: could not determine source repository");
            return ExitCode::FAILURE;
        }
    };

    let dest_repo = match resolve_dest_repo(args, &src_repo) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez cp: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let dest_name = args.rename.clone().unwrap_or_else(|| pkg.base.clone());
    let dest_version = args
        .reversion
        .clone()
        .unwrap_or_else(|| pkg.version.clone());

    if dest_repo == src_repo
        && dest_name == pkg.base
        && dest_version == pkg.version
    {
        eprintln!("rez cp: cannot copy package over itself");
        return ExitCode::FAILURE;
    }

    if !args.force && !package_is_relocatable(&pkg, cfg) {
        eprintln!(
            "rez cp: package is not relocatable (use --force to override)"
        );
        return ExitCode::FAILURE;
    }

    if !args.allow_empty {
        match repo_ops::repo_is_empty(&dest_repo) {
            Ok(true) => {
                eprintln!(
                    "rez cp: destination repository appears empty; use --allow-empty to continue"
                );
                return ExitCode::FAILURE;
            }
            Ok(false) => {}
            Err(err) => {
                eprintln!("rez cp: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    let result = match copy_package(
        &pkg,
        &src_pkg_root,
        &dest_repo,
        &dest_name,
        &dest_version,
        args,
    ) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("rez cp: {}", err);
            return ExitCode::FAILURE;
        }
    };

    print_copy_result(&pkg, &result, args.dry_run);
    ExitCode::SUCCESS
}

fn build_storage_override(storage: &Storage, args: &CpArgs) -> Result<Storage, String> {
    if args.paths.is_none() && !args.no_local {
        return Ok(storage.clone());
    }

    let mut paths: Vec<PathBuf> = Vec::new();

    if let Some(raw) = args.paths.as_deref() {
        paths.extend(std::env::split_paths(raw));
    } else if args.no_local {
        let cfg = config::get().map_err(|e| e.to_string())?;
        paths = config::packages_path(cfg);
        if let Some(local) = config::local_packages_path(cfg) {
            paths.retain(|p| p != &local);
        }
    }

    if paths.is_empty() {
        return Err("no package search paths available".to_string());
    }

    Storage::scan_impl(Some(&paths)).map_err(|e| e.to_string())
}

pub(crate) fn resolve_package(storage: &Storage, req: &str) -> Result<Package, String> {
    let spec = DepSpec::parse_impl(req).map_err(|e| e.to_string())?;
    let mut matches = Vec::new();

    for pkg in storage.packages() {
        if pkg.base != spec.base {
            continue;
        }
        if spec.matches_impl(&pkg.version).unwrap_or(false) {
            matches.push(pkg);
        }
    }

    if matches.is_empty() {
        return Err("No matching packages found.".to_string());
    }

    if matches.len() > 1 {
        let mut names = matches
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        let list = names.join(", ");
        return Err(format!(
            "More than one package matches, please choose: {}",
            list
        ));
    }

    Ok(matches.remove(0))
}

pub(crate) fn package_root(pkg: &Package) -> Result<PathBuf, String> {
    let source = pkg
        .package_source
        .as_ref()
        .ok_or_else(|| "package has no source path".to_string())?;
    let path = PathBuf::from(source);
    let root = path
        .parent()
        .ok_or_else(|| "package source has no parent directory".to_string())?;
    Ok(root.to_path_buf())
}

fn find_repo_root(storage: &Storage, pkg_root: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for repo in storage.location_paths() {
        if pkg_root.starts_with(repo) {
            let replace = best
                .as_ref()
                .map(|b| repo.components().count() > b.components().count())
                .unwrap_or(true);
            if replace {
                best = Some(repo.clone());
            }
        }
    }
    best
}

fn resolve_dest_repo(args: &CpArgs, src_repo: &Path) -> Result<PathBuf, String> {
    if let Some(dest) = args.dest_path.clone() {
        return Ok(dest);
    }

    if args.rename.is_some() || args.reversion.is_some() {
        return Ok(src_repo.to_path_buf());
    }

    Err("--dest-path must be specified unless --rename or --reversion are used".to_string())
}

fn package_is_relocatable(pkg: &Package, cfg: Option<&config::Config>) -> bool {
    if let Some(value) = pkg.relocatable {
        return value;
    }
    if let Some(cfg) = cfg {
        if let Some(value) = config::get_bool(cfg, "default_relocatable") {
            return value;
        }
    }
    true
}

pub(crate) fn copy_package(
    pkg: &Package,
    src_pkg_root: &Path,
    dest_repo: &Path,
    dest_name: &str,
    dest_version: &str,
    args: &CpArgs,
) -> Result<CopyResult, String> {
    let dest_pkg_root = dest_repo.join(dest_name).join(dest_version);

    let variants = collect_variants(pkg);
    let selected = select_variants(&variants, &args.variants)?;
    let is_varianted = !(selected.len() == 1 && selected[0].index.is_none());

    let rewrite_package = dest_name != pkg.base || dest_version != pkg.version;

    let mut copied = Vec::new();
    let mut skipped = Vec::new();

    for variant in selected {
        let src_variant_root = variant_root(src_pkg_root, &variant);
        let dest_variant_root = variant_root(&dest_pkg_root, &variant);

        if dest_variant_root.exists() {
            if args.overwrite {
                if !args.dry_run {
                    std::fs::remove_dir_all(&dest_variant_root).map_err(|e| e.to_string())?;
                }
            } else {
                skipped.push((variant_label(pkg, &variant), dest_variant_root));
                continue;
            }
        }

        if !args.dry_run {
            if args.shallow {
                copy_dir_shallow(&src_variant_root, &dest_variant_root, args.follow_symlinks)?;
            } else {
                copy_dir_recursive(&src_variant_root, &dest_variant_root, args.follow_symlinks)?;
            }
        }

        if !args.dry_run && args.keep_timestamp {
            copy_timestamp(&src_variant_root, &dest_variant_root);
        }

        copied.push((variant_label(pkg, &variant), dest_variant_root));
    }

    if !args.dry_run {
        std::fs::create_dir_all(&dest_pkg_root).map_err(|e| e.to_string())?;
        if rewrite_package || !dest_pkg_root.join("package.py").exists() || args.overwrite {
            copy_package_definition(
                src_pkg_root,
                &dest_pkg_root,
                dest_name,
                dest_version,
                rewrite_package,
            )?;
        }
    }

    Ok(CopyResult {
        copied,
        skipped,
        dest_pkg_root,
        is_varianted,
    })
}

fn print_copy_result(pkg: &Package, result: &CopyResult, dry_run: bool) {
    let verb = if dry_run { "would be" } else { "were" };

    if !result.is_varianted {
        if result.copied.is_empty() && !result.skipped.is_empty() {
            println!(
                "Target package already exists: {}. Use --overwrite to replace it.",
                result.dest_pkg_root.display()
            );
            return;
        }
        if dry_run {
            println!(
                "Would copy {} to {}",
                pkg.name,
                result.dest_pkg_root.display()
            );
        } else {
            println!(
                "Copied {} to {}",
                pkg.name,
                result.dest_pkg_root.display()
            );
        }
        return;
    }

    if !result.copied.is_empty() {
        println!("{} variants {} copied:", result.copied.len(), verb);
        for (label, dest) in &result.copied {
            println!("  {} -> {}", label, dest.display());
        }
    }

    if !result.skipped.is_empty() {
        println!("{} variants {} skipped (target exists):", result.skipped.len(), verb);
        for (label, dest) in &result.skipped {
            println!("  {} !-> {}", label, dest.display());
        }
    }
}

fn collect_variants(pkg: &Package) -> Vec<VariantInfo> {
    if pkg.variants.is_empty() {
        return vec![VariantInfo {
            index: None,
            subpath: None,
        }];
    }

    let mut variants = Vec::new();
    for (idx, reqs) in pkg.variants.iter().enumerate() {
        let subpath = compute_variant_subpath(pkg, reqs);
        variants.push(VariantInfo {
            index: Some(idx),
            subpath,
        });
    }
    variants
}

fn select_variants(all: &[VariantInfo], requested: &[usize]) -> Result<Vec<VariantInfo>, String> {
    if requested.is_empty() {
        return Ok(all.to_vec());
    }

    let mut selected = Vec::new();
    let valid: std::collections::HashSet<usize> = all
        .iter()
        .filter_map(|v| v.index)
        .collect();

    if valid.is_empty() {
        if requested.iter().all(|v| *v == 0) {
            return Ok(all.to_vec());
        }
        return Err("package has no variants; only variant 0 is valid".to_string());
    }

    for idx in requested {
        if !valid.contains(idx) {
            return Err(format!("variant index {} is not valid for this package", idx));
        }
        if let Some(v) = all.iter().find(|v| v.index == Some(*idx)) {
            selected.push(v.clone());
        }
    }
    Ok(selected)
}

fn compute_variant_subpath(pkg: &Package, requires: &[String]) -> Option<String> {
    if pkg.hashed_variants {
        let list_repr = python_list_repr(requires);
        let mut hasher = Sha1::new();
        hasher.update(list_repr.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        return Some(hash);
    }

    if requires.is_empty() {
        return None;
    }

    let mut path = PathBuf::new();
    for req in requires {
        path.push(req);
    }
    Some(path.to_string_lossy().to_string())
}

fn python_list_repr(items: &[String]) -> String {
    let mut out = String::from("[");
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push('\'');
        for ch in item.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '\'' => out.push_str("\\'"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
                other => out.push(other),
            }
        }
        out.push('\'');
    }
    out.push(']');
    out
}

fn variant_root(root: &Path, variant: &VariantInfo) -> PathBuf {
    match &variant.subpath {
        Some(subpath) if !subpath.is_empty() => root.join(subpath),
        _ => root.to_path_buf(),
    }
}

fn variant_label(pkg: &Package, variant: &VariantInfo) -> String {
    if let Some(idx) = variant.index {
        format!("{}[{}]", pkg.name, idx)
    } else {
        pkg.name.clone()
    }
}

fn copy_package_definition(
    src_pkg_root: &Path,
    dest_pkg_root: &Path,
    dest_name: &str,
    dest_version: &str,
    rewrite: bool,
) -> Result<(), String> {
    let src_package_py = src_pkg_root.join("package.py");
    if !src_package_py.exists() {
        return Err("source package.py not found".to_string());
    }

    if !rewrite {
        let dest_package_py = dest_pkg_root.join("package.py");
        std::fs::copy(&src_package_py, &dest_package_py).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let orig_name = "package_orig.py";
    let dest_orig = dest_pkg_root.join(orig_name);
    std::fs::copy(&src_package_py, &dest_orig).map_err(|e| e.to_string())?;

    let wrapper = format!(
        "from importlib import util\n\n\ndef get_package(*args, **kwargs):\n    spec = util.spec_from_file_location('pkg_orig', __file__.replace('package.py', '{orig}'))\n    mod = util.module_from_spec(spec)\n    spec.loader.exec_module(mod)\n    pkg = mod.get_package(*args, **kwargs)\n    pkg.base = '{name}'\n    pkg.version = '{version}'\n    return pkg\n",
        orig = orig_name,
        name = escape_py_string(dest_name),
        version = escape_py_string(dest_version)
    );
    let dest_package_py = dest_pkg_root.join("package.py");
    std::fs::write(&dest_package_py, wrapper).map_err(|e| e.to_string())?;
    Ok(())
}

fn escape_py_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn copy_dir_recursive(src: &Path, dest: &Path, follow_symlinks: bool) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("source path does not exist: {}", src.display()));
    }

    if src.is_file() {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        copy_entry(src, dest, follow_symlinks)?;
        return Ok(());
    }

    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name();
        let src_path = entry.path();
        let dest_path = dest.join(file_name);
        let meta = std::fs::symlink_metadata(&src_path).map_err(|e| e.to_string())?;
        if meta.file_type().is_dir() {
            copy_dir_recursive(&src_path, &dest_path, follow_symlinks)?;
        } else {
            copy_entry(&src_path, &dest_path, follow_symlinks)?;
        }
    }
    Ok(())
}

fn copy_dir_shallow(src: &Path, dest: &Path, follow_symlinks: bool) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let meta = std::fs::symlink_metadata(&src_path).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            copy_entry(&src_path, &dest_path, follow_symlinks)?;
            continue;
        }

        if create_symlink(&src_path, &dest_path).is_err() {
            if meta.is_dir() {
                copy_dir_recursive(&src_path, &dest_path, follow_symlinks)?;
            } else {
                copy_entry(&src_path, &dest_path, follow_symlinks)?;
            }
        }
    }

    Ok(())
}

fn copy_entry(src: &Path, dest: &Path, follow_symlinks: bool) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(src).map_err(|e| e.to_string())?;

    if meta.file_type().is_symlink() {
        if follow_symlinks {
            let target = std::fs::read_link(src).map_err(|e| e.to_string())?;
            let resolved = if target.is_absolute() {
                target
            } else {
                src.parent().unwrap_or_else(|| Path::new(".")).join(target)
            };
            if resolved.is_dir() {
                copy_dir_recursive(&resolved, dest, follow_symlinks)?;
            } else {
                std::fs::copy(&resolved, dest).map_err(|e| e.to_string())?;
            }
        } else {
            create_symlink(src, dest).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if meta.is_dir() {
        copy_dir_recursive(src, dest, follow_symlinks)?;
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::copy(src, dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn create_symlink(src: &Path, dest: &Path) -> std::io::Result<()> {
    if dest.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(src, dest)
        } else {
            std::os::windows::fs::symlink_file(src, dest)
        }
    }
}

fn copy_timestamp(src: &Path, dest: &Path) {
    if let Ok(meta) = std::fs::metadata(src) {
        let atime = FileTime::from_last_access_time(&meta);
        let mtime = FileTime::from_last_modification_time(&meta);
        let _ = set_file_times(dest, atime, mtime);
    }
}
