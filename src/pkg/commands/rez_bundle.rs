//! Rez bundle command.

use crate::cli::{BundleArgs, CpArgs};
use super::rez_cp::copy_package;
use pkg_lib::{archive, config, Package, Storage};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use jwalk::WalkDir;

pub fn cmd_rez_bundle(storage: &Storage, args: &BundleArgs) -> ExitCode {
    let rxt_path = args.rxt.canonicalize().unwrap_or_else(|_| args.rxt.clone());
    let dest_path = args.dest_dir.clone();
    let dest_is_zip = !args.dir;

    if !rxt_path.exists() {
        eprintln!("rez bundle: file does not exist: {}", rxt_path.display());
        return ExitCode::FAILURE;
    }
    if dest_path.exists() {
        if dest_is_zip {
            eprintln!("rez bundle: zip file already exists: {}", dest_path.display());
        } else {
            eprintln!("rez bundle: dest dir must not exist: {}", dest_path.display());
        }
        return ExitCode::FAILURE;
    }

    if dest_is_zip {
        if let Some(parent) = dest_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("rez bundle: {}", err);
                return ExitCode::FAILURE;
            }
        }
    }

    let mut temp_dir: Option<TempDir> = None;
    let bundle_root = if dest_is_zip {
        match tempfile::tempdir() {
            Ok(tmp) => {
                let root = tmp.path().to_path_buf();
                temp_dir = Some(tmp);
                root
            }
            Err(err) => {
                eprintln!("rez bundle: {}", err);
                return ExitCode::FAILURE;
            }
        }
    } else {
        dest_path.canonicalize().unwrap_or(dest_path.clone())
    };

    let doc = match load_rxt(&rxt_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("rez bundle: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = init_bundle(&bundle_root) {
        eprintln!("rez bundle: {}", err);
        return ExitCode::FAILURE;
    }

    let repo_root = bundle_root.join("packages");
    let mut relocated: HashSet<String> = HashSet::new();
    let mut relocated_roots: Vec<(PathBuf, PathBuf)> = Vec::new();

    let resolved = match resolved_packages(&doc) {
        Ok(list) => list,
        Err(err) => {
            eprintln!("rez bundle: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let cfg = config::get().ok();
    let compression_level = cfg
        .and_then(|cfg| config::get_json(cfg, "archive.zip.compression_level"))
        .and_then(|v| v.as_i64())
        .map(|v| v.clamp(0, 9));

    for (name, version, location) in resolved {
        let pkg_name = format!("{}-{}", name, version);
        let pkg = match storage.get(&pkg_name) {
            Some(pkg) => pkg,
            None => {
                eprintln!("rez bundle: package not found: {}", pkg_name);
                return ExitCode::FAILURE;
            }
        };

        if !is_relocatable(&pkg, cfg) {
            if args.force {
                // continue
            } else if args.skip_non_relocatable {
                continue;
            } else {
                eprintln!("rez bundle: package not relocatable: {}", pkg_name);
                return ExitCode::FAILURE;
            }
        }

        let pkg_root = match package_root(&pkg) {
            Ok(root) => root,
            Err(err) => {
                eprintln!("rez bundle: {}", err);
                return ExitCode::FAILURE;
            }
        };

        let cp_args = CpArgs {
            dest_path: Some(repo_root.clone()),
            paths: None,
            no_local: false,
            reversion: None,
            rename: None,
            overwrite: true,
            shallow: false,
            follow_symlinks: false,
            keep_timestamp: true,
            force: args.force,
            allow_empty: true,
            dry_run: false,
            variants: Vec::new(),
            variant_uri: None,
            pkg: Some(pkg_name.clone()),
        };

        let result = match copy_package(&pkg, &pkg_root, &repo_root, &pkg.base, &pkg.version, &cp_args) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("rez bundle: {}", err);
                return ExitCode::FAILURE;
            }
        };
        relocated_roots.extend(result.copied_pairs);
        relocated.insert(pkg_name);

        if let Some(loc) = location {
            if !Path::new(&loc).exists() {
                eprintln!("rez bundle: warning: source location not found: {}", loc);
            }
        }
    }

    if let Err(err) = write_retargeted_context(&doc, &bundle_root, &repo_root, &relocated, dest_is_zip) {
        eprintln!("rez bundle: {}", err);
        return ExitCode::FAILURE;
    }

    if let Err(err) = write_bundle_meta(&bundle_root, &rxt_path, &relocated, dest_is_zip, compression_level) {
        eprintln!("rez bundle: {}", err);
        return ExitCode::FAILURE;
    }

    if let Err(err) = write_bundle_manifest(&bundle_root) {
        eprintln!("rez bundle: {}", err);
        return ExitCode::FAILURE;
    }

    if !args.no_lib_patch {
        if let Err(e) = pkg_lib::bundle_patch::patch_bundle_libs(&bundle_root, &relocated_roots) {
            log::warn!("rez bundle: lib patching failed: {}", e);
        }
    }

    if dest_is_zip {
        if let Err(err) = archive::zip_dir(&bundle_root, &dest_path, compression_level) {
            eprintln!("rez bundle: {}", err);
            return ExitCode::FAILURE;
        }
    }

    drop(temp_dir);

    ExitCode::SUCCESS
}

fn load_rxt(path: &Path) -> Result<JsonValue, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn resolved_packages(doc: &JsonValue) -> Result<Vec<(String, String, Option<String>)>, String> {
    let list = doc
        .get("resolved_packages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "resolved_packages missing from context".to_string())?;

    let mut out = Vec::new();
    for item in list {
        let vars = item
            .get("variables")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "invalid resolved_packages entry".to_string())?;
        let name = vars
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "resolved_packages entry missing name".to_string())?;
        let version = vars
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "resolved_packages entry missing version".to_string())?;
        let location = vars.get("location").and_then(|v| v.as_str()).map(|s| s.to_string());
        out.push((name.to_string(), version.to_string(), location));
    }

    Ok(out)
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

fn is_relocatable(pkg: &Package, cfg: Option<&config::Config>) -> bool {
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

fn init_bundle(dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let repo_path = dest_dir.join("packages");
    fs::create_dir_all(&repo_path).map_err(|e| e.to_string())?;

    fs::write(dest_dir.join("bundle.yaml"), "logs: []\n").map_err(|e| e.to_string())?;
    fs::write(repo_path.join("settings.yaml"), "disable_memcached: true\n").map_err(|e| e.to_string())?;
    Ok(())
}

fn write_retargeted_context(
    doc: &JsonValue,
    dest_dir: &Path,
    repo_root: &Path,
    relocated: &HashSet<String>,
    relative_paths: bool,
) -> Result<(), String> {
    let mut doc = doc.clone();
    let repo_root_str = if relative_paths {
        "packages".to_string()
    } else {
        repo_root.to_string_lossy().to_string()
    };

    if let Some(obj) = doc.as_object_mut() {
        obj.insert(
            "package_paths".to_string(),
            JsonValue::Array(vec![JsonValue::String(repo_root_str.clone())]),
        );

        if let Some(resolved) = obj.get_mut("resolved_packages") {
            if let Some(list) = resolved.as_array_mut() {
                for item in list {
                    if let Some(vars) = item.get_mut("variables").and_then(|v| v.as_object_mut()) {
                        let name = vars.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let version = vars.get("version").and_then(|v| v.as_str()).unwrap_or("");
                        let pkg_name = format!("{}-{}", name, version);
                        if relocated.contains(&pkg_name) {
                            vars.insert(
                                "location".to_string(),
                                JsonValue::String(repo_root_str.clone()),
                            );
                        }
                    }
                }
            }
        }
    }

    let out_path = dest_dir.join("context.rxt");
    let content = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    fs::write(out_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_bundle_meta(
    dest_dir: &Path,
    rxt_path: &Path,
    relocated: &HashSet<String>,
    zip_mode: bool,
    compression_level: Option<i64>,
) -> Result<(), String> {
    let mut packages: Vec<String> = relocated.iter().cloned().collect();
    packages.sort();

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let meta = serde_json::json!({
        "format": "pkg-rs-bundle",
        "format_version": 1,
        "created_unix": created,
        "pkg_rs_version": pkg_lib::VERSION,
        "zip": zip_mode,
        "compression_level": compression_level,
        "source_context": rxt_path.to_string_lossy().to_string(),
        "package_count": packages.len(),
        "packages": packages,
    });

    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(dest_dir.join("bundle.meta.json"), content).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_bundle_manifest(dest_dir: &Path) -> Result<(), String> {
    let mut entries: Vec<JsonValue> = Vec::new();

    for entry in WalkDir::new(dest_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(dest_dir)
            .map_err(|e| e.to_string())?;
        let path = path.to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        if rel == Path::new("bundle.manifest.json") {
            continue;
        }

        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let digest = format!("{:x}", hasher.finalize());
        let size = entry.metadata().map_err(|e| e.to_string())?.len();

        let rel_str = rel.to_string_lossy().replace('\\', "/");
        entries.push(serde_json::json!({
            "path": rel_str,
            "size": size,
            "sha256": digest,
        }));
    }

    entries.sort_by(|a, b| {
        let ap = a.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let bp = b.get("path").and_then(|v| v.as_str()).unwrap_or("");
        ap.cmp(bp)
    });

    let meta = serde_json::json!({
        "format": "pkg-rs-bundle-manifest",
        "format_version": 1,
        "entries": entries,
    });

    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(dest_dir.join("bundle.manifest.json"), content).map_err(|e| e.to_string())?;
    Ok(())
}
