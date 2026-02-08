//! Rez diff command (native filesystem diff).

use crate::cli::DiffArgs;
use pkg_lib::{config, Package, Storage};
use similar::TextDiff;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const MAX_TEXT_DIFF_BYTES: u64 = 2 * 1024 * 1024; // 2 MiB

pub fn cmd_rez_diff(storage: &Storage, args: &DiffArgs) -> ExitCode {
    let pkg1 = match resolve_package(storage, &args.pkg1) {
        Ok(pkg) => pkg,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let pkg2 = match resolve_diff_target(storage, &pkg1, args.pkg2.as_deref()) {
        Ok(pkg) => pkg,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let root1 = match package_root(&pkg1) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };
    let root2 = match package_root(&pkg2) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if let Some(tool) = resolve_difftool() {
        if run_difftool(&tool, &root1, &root2) {
            return ExitCode::SUCCESS;
        }
        eprintln!("rez diff: failed to launch difftool '{}', falling back to unified diff", tool);
    }

    print_diff(&pkg1, &pkg2, &root1, &root2)
}

fn resolve_package(storage: &Storage, request: &str) -> Result<Package, String> {
    storage
        .resolve(request)
        .ok_or_else(|| format!("package not found: {}", request))
}

fn resolve_diff_target(
    storage: &Storage,
    pkg1: &Package,
    pkg2_request: Option<&str>,
) -> Result<Package, String> {
    if let Some(req) = pkg2_request {
        return resolve_package(storage, req);
    }

    let versions = storage.versions(&pkg1.base);
    let idx = versions
        .iter()
        .position(|name| name == &pkg1.name)
        .ok_or_else(|| format!("package not found in storage: {}", pkg1.name))?;
    let next_idx = idx + 1;
    if next_idx >= versions.len() {
        return Err(format!(
            "No package to diff with - {} is the earliest package version",
            pkg1.name
        ));
    }

    let next_name = &versions[next_idx];
    storage
        .get(next_name)
        .ok_or_else(|| format!("package not found: {}", next_name))
}

fn package_root(pkg: &Package) -> Result<PathBuf, String> {
    let source = pkg
        .package_source
        .as_ref()
        .ok_or_else(|| format!("package {} has no source path", pkg.name))?;
    let path = PathBuf::from(source);
    if path.is_dir() {
        Ok(path)
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| format!("package {} has invalid source path", pkg.name))
    }
}

fn resolve_difftool() -> Option<String> {
    if let Ok(cfg) = config::get() {
        if let Some(tool) = config::get_str(cfg, "difftool") {
            if !tool.trim().is_empty() {
                return Some(tool);
            }
        }
    }

    platform_difftool()
}

fn platform_difftool() -> Option<String> {
    if cfg!(windows) {
        Some("fc".to_string())
    } else if cfg!(target_os = "macos") {
        Some("diff".to_string())
    } else {
        Some("diff".to_string())
    }
}

fn run_difftool(tool: &str, left: &Path, right: &Path) -> bool {
    let status = Command::new(tool)
        .arg(left)
        .arg(right)
        .status();

    match status {
        Ok(status) => status.success(),
        Err(err) => {
            eprintln!("rez diff: {}", err);
            false
        }
    }
}

fn print_diff(pkg1: &Package, pkg2: &Package, root1: &Path, root2: &Path) -> ExitCode {
    println!("Diffing {} -> {}", pkg1.name, pkg2.name);
    let map1 = match collect_files(root1) {
        Ok(map) => map,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };
    let map2 = match collect_files(root2) {
        Ok(map) => map,
        Err(err) => {
            eprintln!("rez diff: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let mut all: HashSet<String> = HashSet::new();
    for key in map1.keys() {
        all.insert(key.clone());
    }
    for key in map2.keys() {
        all.insert(key.clone());
    }

    let mut all: Vec<String> = all.into_iter().collect();
    all.sort();

    let mut had_changes = false;

    for rel in all {
        match (map1.get(&rel), map2.get(&rel)) {
            (Some(_path1), None) => {
                had_changes = true;
                println!("D  {}", rel);
            }
            (None, Some(_)) => {
                had_changes = true;
                println!("A  {}", rel);
            }
            (Some(path1), Some(path2)) => {
                match diff_files(path1, path2, &rel) {
                    Ok(changed) => {
                        if changed {
                            had_changes = true;
                        }
                    }
                    Err(err) => {
                        eprintln!("rez diff: {}", err);
                        return ExitCode::FAILURE;
                    }
                }
            }
            _ => {}
        }
    }

    if !had_changes {
        println!("No differences found.");
    }

    ExitCode::SUCCESS
}

fn collect_files(root: &Path) -> Result<HashMap<String, PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_impl(root, &mut files).map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        map.insert(rel, path);
    }

    Ok(map)
}

fn collect_files_impl(root: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            collect_files_impl(&path, out)?;
        } else if meta.is_file() {
            out.push(path);
        }
    }

    Ok(())
}

fn diff_files(path1: &Path, path2: &Path, rel: &str) -> Result<bool, String> {
    let meta1 = fs::metadata(path1).map_err(|e| e.to_string())?;
    let meta2 = fs::metadata(path2).map_err(|e| e.to_string())?;

    if meta1.len() == meta2.len() && meta1.len() == 0 {
        return Ok(false);
    }

    let bytes1 = fs::read(path1).map_err(|e| e.to_string())?;
    let bytes2 = fs::read(path2).map_err(|e| e.to_string())?;

    if bytes1 == bytes2 {
        return Ok(false);
    }

    if meta1.len() > MAX_TEXT_DIFF_BYTES || meta2.len() > MAX_TEXT_DIFF_BYTES {
        println!("M  {} (large/binary)", rel);
        return Ok(true);
    }

    let text1 = String::from_utf8(bytes1);
    let text2 = String::from_utf8(bytes2);
    match (text1, text2) {
        (Ok(a), Ok(b)) => {
            let diff = TextDiff::from_lines(&a, &b);
            let unified = diff
                .unified_diff()
                .header(&format!("a/{}", rel), &format!("b/{}", rel))
                .to_string();
            if unified.trim().is_empty() {
                Ok(false)
            } else {
                println!("{}", unified);
                Ok(true)
            }
        }
        _ => {
            println!("M  {} (binary)", rel);
            Ok(true)
        }
    }
}
