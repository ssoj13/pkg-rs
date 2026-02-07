//! Pip import utilities.
//!
//! This module mirrors rez-pip behavior where possible: it uses `pip install --target`,
//! derives requirements from dist-info metadata, installs payload into a rez-like
//! repository layout, and uses hashed variants when variant requirements are present.

use crate::error::PipError;
use crate::Storage;
use csv::ReaderBuilder;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use regex::Regex;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options for importing a pip package into a repo layout.
#[derive(Debug, Clone)]
pub struct PipOptions {
    /// Optional python version selector (e.g. 3.11).
    pub python_version: Option<String>,
    /// Whether to perform the install (required).
    pub install: bool,
    /// Mark install as a release (prefer shared repo).
    pub release: bool,
    /// Optional repository root override.
    pub prefix: Option<PathBuf>,
    /// Extra args passed to `pip install`.
    pub extra_args: Vec<String>,
    /// Dependency install mode.
    pub install_mode: PipInstallMode,
}

/// Dependency installation mode (rez-pip parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipInstallMode {
    /// Install no dependencies (pip --no-deps).
    NoDeps,
    /// Install minimal dependencies (default).
    MinDeps,
}

/// Summary of a pip import.
#[derive(Debug, Clone)]
pub struct PipReport {
    /// Normalized package name.
    pub name: String,
    /// Version converted to semver-ish format.
    pub version: String,
    /// Install root path in repository layout.
    pub install_path: PathBuf,
    /// Python interpreter used for pip.
    pub python: String,
    /// Entry point names created as wrappers.
    pub entry_points: Vec<String>,
    /// Derived rez-style requirements.
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone)]
struct PythonCmd {
    program: String,
    base_args: Vec<String>,
    version: String,
    env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
struct EntryPoint {
    name: String,
    module: String,
    attr: String,
}

#[derive(Debug, Clone)]
struct PipMetadata {
    name: String,
    version: String,
    summary: Option<String>,
    home_page: Option<String>,
    download_url: Option<String>,
    author: Option<String>,
    author_email: Option<String>,
    requires_python: Option<String>,
    requires_dist: Vec<String>,
    entry_points: Vec<EntryPoint>,
    dist_info_path: PathBuf,
    dist_names: Vec<String>,
    is_pure_python: bool,
}

#[derive(Debug, Clone)]
struct PipRequirements {
    requires: Vec<String>,
    variant_requires: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedRequirement {
    name: String,
    specifier: String,
    marker: Option<String>,
}

/// Install a pip package into a repository layout and generate `package.py`.
///
/// The import uses `pip install --target`, parses metadata from dist-info,
/// creates entry point wrappers, and writes a rez-style `package.py` file.
pub fn import_pip_package(
    storage: &Storage,
    package: &str,
    options: &PipOptions,
) -> Result<PipReport, PipError> {
    if !options.install {
        return Err(PipError::Config("--install is required".to_string()));
    }

    let python = find_python(storage, options.python_version.as_deref())?;
    ensure_pip(&python)?;

    let temp_dir = tempfile::tempdir()?;
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(&target_dir)?;

    run_pip_install(
        &python,
        package,
        &target_dir,
        &options.extra_args,
        options.install_mode,
    )?;

    let requested_name = requested_name_from_spec(package).map(|n| normalize_package_name(&n));

    let metadata = load_metadata(&target_dir, requested_name.as_deref())?;
    let bundle_deps =
        options.install_mode == PipInstallMode::MinDeps && metadata.dist_names.len() > 1;

    let base_name = normalize_package_name(&metadata.name);
    let semver_version = pep440_to_semver(&metadata.version);

    let repo_root = resolve_repo_root(storage, options.prefix.as_ref(), options.release)?;
    let package_root = repo_root.join(&base_name).join(&semver_version);

    if package_root.exists() {
        std::fs::remove_dir_all(&package_root)?;
    }

    let pip_requirements = build_requirements(&metadata, Some(&python.version));
    let requires = if bundle_deps {
        Vec::new()
    } else {
        pip_requirements.requires
    };
    let variant_requires = pip_requirements.variant_requires;
    let variant_subpath = if variant_requires.is_empty() {
        None
    } else {
        Some(hash_variant_subpath(&variant_requires))
    };

    let variant_install_path = match &variant_subpath {
        Some(subpath) => package_root.join(subpath),
        None => package_root.clone(),
    };

    copy_pip_payload(
        &target_dir,
        &metadata.dist_info_path,
        &variant_install_path,
        bundle_deps,
    )?;

    let entry_points = metadata.entry_points.clone();
    let mut entry_names = Vec::new();
    if !entry_points.is_empty() {
        let bin_root = variant_install_path.join("bin");
        std::fs::create_dir_all(&bin_root)?;
        write_entry_points(&bin_root, &entry_points)?;
        entry_names = entry_points.iter().map(|ep| ep.name.clone()).collect();
    }

    let pip_name = format!("{} {}", metadata.name, metadata.version);
    let mut help = Vec::new();
    if let Some(home) = metadata.home_page.as_deref() {
        help.push(vec!["Home Page".to_string(), home.to_string()]);
    }
    if let Some(url) = metadata.download_url.as_deref() {
        help.push(vec!["Source Code".to_string(), url.to_string()]);
    }
    let authors = if let Some(author) = metadata.author.as_deref() {
        let mut entry = author.to_string();
        if let Some(email) = metadata.author_email.as_deref() {
            if !email.trim().is_empty() {
                entry.push(' ');
                entry.push_str(email.trim());
            }
        }
        vec![entry]
    } else {
        Vec::new()
    };

    write_package_py(
        &package_root,
        &base_name,
        &semver_version,
        &requires,
        &variant_requires,
        &entry_points,
        options.release,
        metadata.summary.as_deref(),
        true,
        variant_subpath.as_deref(),
        &pip_name,
        true,
        metadata.is_pure_python,
        &help,
        &authors,
        &entry_names,
    )?;

    Ok(PipReport {
        name: base_name,
        version: semver_version,
        install_path: package_root,
        python: python.version,
        entry_points: entry_names,
        requirements: requires,
    })
}

fn find_python(storage: &Storage, required: Option<&str>) -> Result<PythonCmd, PipError> {
    if let Some(cmd) = find_rez_python(storage, required) {
        return Ok(cmd);
    }

    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(ver) = required {
        let trimmed = ver.trim();
        if cfg!(windows) {
            candidates.push(("py".to_string(), vec![format!("-{}", trimmed)]));
            candidates.push((format!("python{}", trimmed), Vec::new()));
            if let Some((major, minor)) = split_major_minor(trimmed) {
                candidates.push((format!("python{}.{}", major, minor), Vec::new()));
            }
        } else {
            candidates.push((format!("python{}", trimmed), Vec::new()));
            if let Some((major, minor)) = split_major_minor(trimmed) {
                candidates.push((format!("python{}.{}", major, minor), Vec::new()));
            }
        }
    } else {
        candidates.push(("python".to_string(), Vec::new()));
        candidates.push(("python3".to_string(), Vec::new()));
        if cfg!(windows) {
            candidates.push(("py".to_string(), Vec::new()));
        }
    }

    for (program, base_args) in candidates {
        if let Some(found) = probe_python(&program, &base_args, required, None) {
            return Ok(found);
        }
    }

    Err(PipError::Config(
        "python interpreter not found on PATH".to_string(),
    ))
}

fn find_rez_python(storage: &Storage, required: Option<&str>) -> Option<PythonCmd> {
    let mut reqs = Vec::new();
    if let Some(range) = required.and_then(python_major_minor_range) {
        reqs.push(format!("python@{}", range));
    } else {
        reqs.push("python".to_string());
    }

    let mut try_reqs = Vec::new();
    try_reqs.push(reqs.clone());
    if storage.bases().iter().any(|b| b == "pip") {
        let mut with_pip = reqs.clone();
        with_pip.push("pip".to_string());
        try_reqs.insert(0, with_pip);
    }

    for req_list in try_reqs {
        if let Some(env_map) = build_env_from_reqs(storage, &req_list) {
            let candidates = python_candidates(required);
            for (program, base_args) in candidates {
                if let Some(found) = probe_python(&program, &base_args, required, Some(&env_map))
                {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn python_candidates(required: Option<&str>) -> Vec<(String, Vec<String>)> {
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(ver) = required {
        let trimmed = ver.trim();
        if cfg!(windows) {
            candidates.push(("py".to_string(), vec![format!("-{}", trimmed)]));
            candidates.push((format!("python{}", trimmed), Vec::new()));
            if let Some((major, minor)) = split_major_minor(trimmed) {
                candidates.push((format!("python{}.{}", major, minor), Vec::new()));
            }
        } else {
            candidates.push((format!("python{}", trimmed), Vec::new()));
            if let Some((major, minor)) = split_major_minor(trimmed) {
                candidates.push((format!("python{}.{}", major, minor), Vec::new()));
            }
        }
    } else {
        candidates.push(("python".to_string(), Vec::new()));
        candidates.push(("python3".to_string(), Vec::new()));
        if cfg!(windows) {
            candidates.push(("py".to_string(), Vec::new()));
        }
    }

    candidates
}

fn build_env_from_reqs(storage: &Storage, reqs: &[String]) -> Option<HashMap<String, String>> {
    let mut ctx_pkg = crate::Package::new("_pip_context".to_string(), "0.0.0".to_string());
    ctx_pkg.reqs = reqs.to_vec();
    if ctx_pkg.solve(storage.packages()).is_err() {
        return None;
    }
    let env = ctx_pkg
        ._env("default", true)
        .unwrap_or_else(|| crate::Env::new("default".to_string()));
    let solved = env.solve_impl(10, true).unwrap_or_else(|_| env.compress());
    Some(solved.to_map())
}

fn split_major_minor(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn parse_pip_version(text: &str) -> Option<String> {
    let mut parts = text.split_whitespace();
    let first = parts.next()?;
    if first != "pip" {
        return None;
    }
    parts.next().map(|v| v.trim().to_string())
}

fn parse_major(version: &str) -> Option<u32> {
    let major = version.split('.').next()?;
    major.parse().ok()
}

fn probe_python(
    program: &str,
    base_args: &[String],
    required: Option<&str>,
    env: Option<&HashMap<String, String>>,
) -> Option<PythonCmd> {
    let mut args = base_args.to_vec();
    args.push("-c".to_string());
    args.push(
        "import sys; print(f\"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}\")"
            .to_string(),
    );

    let mut cmd = Command::new(program);
    cmd.args(&args);
    if let Some(env) = env {
        cmd.envs(env);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        return None;
    }

    if let Some(req) = required {
        if !version.starts_with(req.trim()) {
            return None;
        }
    }

    Some(PythonCmd {
        program: program.to_string(),
        base_args: base_args.to_vec(),
        version,
        env: env.cloned(),
    })
}

fn ensure_pip(python: &PythonCmd) -> Result<(), PipError> {
    let mut args = python.base_args.clone();
    args.push("-m".to_string());
    args.push("pip".to_string());
    args.push("--version".to_string());

    let mut cmd = Command::new(&python.program);
    cmd.args(&args);
    if let Some(env) = &python.env {
        cmd.envs(env);
    }
    let output = cmd.output().map_err(PipError::Io)?;
    if !output.status.success() {
        return Err(PipError::Config("pip is not available".to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = parse_pip_version(&stdout)
        .ok_or_else(|| PipError::Config("failed to parse pip version".to_string()))?;

    let major = parse_major(&version).unwrap_or(0);
    if major < 19 {
        return Err(PipError::Config(format!(
            "pip>=19 is required (found {})",
            version
        )));
    }

    Ok(())
}

fn run_pip_install(
    python: &PythonCmd,
    package: &str,
    target_dir: &Path,
    extra_args: &[String],
    install_mode: PipInstallMode,
) -> Result<(), PipError> {
    let mut args = python.base_args.clone();
    args.push("-m".to_string());
    args.push("pip".to_string());
    args.push("install".to_string());
    args.push("--no-input".to_string());
    args.push("--disable-pip-version-check".to_string());

    if !option_present(extra_args, "--no-use-pep517", "--no-use-pep517") {
        args.push("--use-pep517".to_string());
    }

    if !option_present(extra_args, "-t", "--target") {
        args.push("--target".to_string());
        args.push(target_dir.display().to_string());
    }

    if install_mode == PipInstallMode::NoDeps
        && !option_present(extra_args, "--no-deps", "--no-deps")
    {
        args.push("--no-deps".to_string());
    }

    args.extend(extra_args.iter().cloned());
    args.push(package.to_string());

    let mut cmd = Command::new(&python.program);
    cmd.args(&args);
    if let Some(env) = &python.env {
        cmd.envs(env);
    }
    let status = cmd.status().map_err(PipError::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(PipError::CommandFailed {
            command: format!("{} {}", python.program, args.join(" ")),
            code: status.code(),
        })
    }
}

fn load_metadata(target_dir: &Path, requested_name: Option<&str>) -> Result<PipMetadata, PipError> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".dist-info") || name.ends_with(".egg-info") {
            candidates.push(entry.path());
        }
    }

    if candidates.is_empty() {
        return Err(PipError::Config(
            "pip install did not produce dist-info metadata".to_string(),
        ));
    }

    let mut parsed = Vec::new();
    for dir in candidates {
        if let Ok(meta) = parse_metadata_dir(&dir) {
            parsed.push((dir, meta));
        }
    }

    if parsed.is_empty() {
        return Err(PipError::Config(
            "failed to parse pip metadata".to_string(),
        ));
    }

    let dist_names = parsed
        .iter()
        .map(|(_, meta)| meta.name.clone())
        .collect::<Vec<_>>();

    if let Some(requested) = requested_name {
        for (_dir, meta) in &parsed {
            if normalize_package_name(&meta.name) == requested {
                let mut selected = meta.clone();
                selected.dist_names = dist_names.clone();
                return Ok(selected);
            }
        }
    }

    if parsed.len() == 1 {
        let mut selected = parsed.remove(0).1;
        selected.dist_names = dist_names;
        Ok(selected)
    } else {
        Err(PipError::Config(
            "multiple packages installed; specify an exact package name".to_string(),
        ))
    }
}

fn parse_metadata_dir(dir: &Path) -> Result<PipMetadata, PipError> {
    let metadata_path = if dir.file_name().and_then(|s| s.to_str()).unwrap_or("").ends_with(".dist-info") {
        dir.join("METADATA")
    } else {
        dir.join("PKG-INFO")
    };

    let metadata_map = parse_metadata_file(&metadata_path)?;

    let name = metadata_map
        .get("Name")
        .and_then(|v| v.first())
        .cloned()
        .ok_or_else(|| PipError::Config("missing Name in metadata".to_string()))?;
    let version = metadata_map
        .get("Version")
        .and_then(|v| v.first())
        .cloned()
        .ok_or_else(|| PipError::Config("missing Version in metadata".to_string()))?;
    let summary = metadata_map
        .get("Summary")
        .and_then(|v| v.first())
        .cloned();
    let home_page = metadata_map
        .get("Home-page")
        .and_then(|v| v.first())
        .cloned();
    let download_url = metadata_map
        .get("Download-URL")
        .and_then(|v| v.first())
        .cloned();
    let author = metadata_map
        .get("Author")
        .and_then(|v| v.first())
        .cloned();
    let author_email = metadata_map
        .get("Author-email")
        .and_then(|v| v.first())
        .cloned();
    let requires_python = metadata_map
        .get("Requires-Python")
        .and_then(|v| v.first())
        .cloned();
    let requires_dist = metadata_map
        .get("Requires-Dist")
        .cloned()
        .unwrap_or_default();

    let entry_points = parse_entry_points(&dir.join("entry_points.txt"));
    let is_pure_python = read_is_pure_python(&dir);

    Ok(PipMetadata {
        name,
        version,
        summary,
        home_page,
        download_url,
        author,
        author_email,
        requires_python,
        requires_dist,
        entry_points,
        dist_info_path: dir.to_path_buf(),
        dist_names: Vec::new(),
        is_pure_python,
    })
}

fn parse_metadata_file(path: &Path) -> Result<HashMap<String, Vec<String>>, PipError> {
    let content = std::fs::read_to_string(path)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in content.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if current_key.is_some() {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }

        if let Some(key) = current_key.take() {
            map.entry(key).or_default().push(current_value.trim().to_string());
            current_value.clear();
        }

        if let Some((key, value)) = line.split_once(':') {
            current_key = Some(key.trim().to_string());
            current_value.push_str(value.trim());
        }
    }

    if let Some(key) = current_key.take() {
        map.entry(key).or_default().push(current_value.trim().to_string());
    }

    Ok(map)
}

fn parse_entry_points(path: &Path) -> Vec<EntryPoint> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut entries = Vec::new();
    let mut section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].to_string();
            continue;
        }
        if section != "console_scripts" && section != "gui_scripts" {
            continue;
        }

        let Some((name, target)) = trimmed.split_once('=') else { continue };
        let name = name.trim().to_string();
        let target = target.trim();
        let target = target.split_whitespace().next().unwrap_or(target);
        let target = target.split('[').next().unwrap_or(target);
        let (module, attr) = match target.split_once(':') {
            Some((m, a)) => (m.trim().to_string(), a.trim().to_string()),
            None => (target.trim().to_string(), "main".to_string()),
        };
        entries.push(EntryPoint { name, module, attr });
    }

    entries
}

fn read_is_pure_python(dist_info_path: &Path) -> bool {
    let wheel_path = dist_info_path.join("WHEEL");
    let content = match std::fs::read_to_string(&wheel_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Root-Is-Purelib:") {
            return rest.trim().eq_ignore_ascii_case("true");
        }
    }
    false
}

fn write_entry_points(bin_root: &Path, entries: &[EntryPoint]) -> Result<(), PipError> {
    for entry in entries {
        write_entry_point(bin_root, entry)?;
    }
    Ok(())
}

fn write_entry_point(bin_root: &Path, entry: &EntryPoint) -> Result<(), PipError> {
    let py_path = bin_root.join(format!("{}.py", entry.name));
    let script_body = format!(
        "#!/usr/bin/env python\nimport sys\nfrom importlib import import_module\n\n\ndef _run():\n    module = import_module(\"{}\")\n    func = getattr(module, \"{}\")\n    result = func()\n    if isinstance(result, int):\n        return result\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(_run())\n",
        entry.module,
        entry.attr
    );

    std::fs::write(&py_path, script_body)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&py_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&py_path, perms)?;
    }

    if cfg!(windows) {
        let cmd_path = bin_root.join(format!("{}.cmd", entry.name));
        let cmd_body = format!(
            "@echo off\r\npython \"%~dp0\\{}.py\" %*\r\n",
            entry.name
        );
        std::fs::write(cmd_path, cmd_body)?;
    } else {
        let sh_path = bin_root.join(&entry.name);
        let sh_body = format!(
            "#!/usr/bin/env bash\nSCRIPT_DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npython \"$SCRIPT_DIR/{}.py\" \"$@\"\n",
            entry.name
        );
        std::fs::write(&sh_path, sh_body)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&sh_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&sh_path, perms)?;
        }
    }

    Ok(())
}

fn hash_variant_subpath(variant_requires: &[String]) -> String {
    let list_repr = python_list_repr(variant_requires);
    let mut hasher = Sha1::new();
    hasher.update(list_repr.as_bytes());
    format!("{:x}", hasher.finalize())
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

fn copy_pip_payload(
    target_dir: &Path,
    dist_info_path: &Path,
    install_root: &Path,
    bundle_deps: bool,
) -> Result<(), PipError> {
    if bundle_deps {
        let python_root = install_root.join("python");
        copy_dir(target_dir, &python_root)?;
        return Ok(());
    }
    let record_paths = collect_record_paths(dist_info_path).ok();
    let mut copied = 0usize;
    let remaps = load_pip_install_remaps();

    if let Some(paths) = record_paths {
        for rel in paths {
            let mapping = match map_record_path(&rel, &remaps) {
                Ok(Some(m)) => m,
                Ok(None) => continue,
                Err(err) => return Err(err),
            };
            let src = target_dir.join(path_from_rel(&mapping.src_rel));
            if !src.exists() || src.is_dir() {
                continue;
            }
            let dest = install_root.join(path_from_rel(&mapping.dest_rel));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dest)?;
            copied += 1;
        }
    }

    if copied == 0 {
        let python_root = install_root.join("python");
        copy_dir(target_dir, &python_root)?;
    }

    Ok(())
}

fn collect_record_paths(dist_info_path: &Path) -> Result<Vec<String>, PipError> {
    let record_path = dist_info_path.join("RECORD");
    if !record_path.exists() {
        return Err(PipError::Config("dist-info RECORD not found".to_string()));
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(&record_path)
        .map_err(|e| PipError::Config(e.to_string()))?;

    let mut paths = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| PipError::Config(e.to_string()))?;
        if let Some(path) = record.get(0) {
            let path = path.trim();
            if !path.is_empty() {
                paths.push(path.to_string());
            }
        }
    }

    Ok(paths)
}

#[derive(Debug, Clone)]
struct RecordMapping {
    src_rel: String,
    dest_rel: String,
}

#[derive(Debug, Clone)]
struct PipInstallRemap {
    record_path: Regex,
    pip_install: String,
    rez_install: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PipInstallRemapConfig {
    record_path: String,
    pip_install: String,
    rez_install: String,
}

fn load_pip_install_remaps() -> Vec<PipInstallRemap> {
    let configs = std::env::var("PKG_PIP_INSTALL_REMAPS")
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<PipInstallRemapConfig>>(&raw).ok())
        .unwrap_or_else(default_pip_install_remaps);

    let mut remaps = Vec::new();
    for cfg in configs {
        let record_pat = expand_remap_pattern_regex(&cfg.record_path);
        let pip_install = expand_remap_pattern_repl(&cfg.pip_install);
        let rez_install = expand_remap_pattern_repl(&cfg.rez_install);
        if let Ok(regex) = Regex::new(&record_pat) {
            remaps.push(PipInstallRemap {
                record_path: regex,
                pip_install,
                rez_install,
            });
        }
    }
    remaps
}

fn default_pip_install_remaps() -> Vec<PipInstallRemapConfig> {
    vec![
        PipInstallRemapConfig {
            record_path: r"^{p}{s}{p}{s}(bin{s}.*)".to_string(),
            pip_install: r"\1".to_string(),
            rez_install: r"\1".to_string(),
        },
        PipInstallRemapConfig {
            record_path: r"^{p}{s}{p}{s}lib{s}python{s}(.*)".to_string(),
            pip_install: r"\1".to_string(),
            rez_install: r"python{s}\1".to_string(),
        },
    ]
}

fn expand_remap_pattern_regex(pattern: &str) -> String {
    let mut out = pattern.replace("{pardir}", "..").replace("{p}", "..");
    out = out
        .replace("{sep}", r"[/\\]")
        .replace("{s}", r"[/\\]");
    out
}

fn expand_remap_pattern_repl(pattern: &str) -> String {
    let mut out = pattern.replace("{pardir}", "..").replace("{p}", "..");
    out = out.replace("{sep}", "/").replace("{s}", "/");
    out
}

fn map_record_path(
    rel_src: &str,
    remaps: &[PipInstallRemap],
) -> Result<Option<RecordMapping>, PipError> {
    let rel = rel_src.replace('\\', "/");
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Ok(None);
    }

    let top = parts[0];
    if top.ends_with(".dist-info") {
        return Ok(Some(RecordMapping {
            src_rel: rel.clone(),
            dest_rel: format!("python/{}", rel),
        }));
    }

    if rel.starts_with("..") {
        for remap in remaps {
            if remap.record_path.is_match(&rel) {
                let pip_subpath = remap.record_path.replace(&rel, remap.pip_install.as_str());
                let rez_subpath = remap.record_path.replace(&rel, remap.rez_install.as_str());
                return Ok(Some(RecordMapping {
                    src_rel: pip_subpath.to_string(),
                    dest_rel: rez_subpath.to_string(),
                }));
            }
        }

        return Err(PipError::Config(format!(
            "unknown RECORD path: {} (set PKG_PIP_INSTALL_REMAPS)",
            rel
        )));
    }

    Ok(Some(RecordMapping {
        src_rel: rel.clone(),
        dest_rel: format!("python/{}", rel),
    }))
}

fn path_from_rel(rel: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for part in rel.split(['/', '\\']) {
        if part.is_empty() {
            continue;
        }
        path.push(part);
    }
    path
}

fn option_present(opts: &[String], short: &str, long: &str) -> bool {
    for opt in opts {
        if opt == short || opt == long {
            return true;
        }
        if opt.starts_with(&format!("{}=", short)) || opt.starts_with(&format!("{}=", long)) {
            return true;
        }
    }
    false
}

fn build_requirements(metadata: &PipMetadata, python_version: Option<&str>) -> PipRequirements {
    let mut requires = Vec::new();
    let mut variant_requires = Vec::new();
    let mut seen_requires = HashSet::new();
    let mut seen_variant = HashSet::new();

    let python_version = python_version.unwrap_or("0.0.0");
    let python_mm = split_major_minor(python_version)
        .map(|(maj, min)| format!("{}.{}", maj, min))
        .unwrap_or_else(|| "0.0".to_string());

    let mut sys_requires: HashSet<&'static str> = HashSet::new();
    sys_requires.insert("python");

    let has_entry_points = !metadata.entry_points.is_empty();
    if !metadata.is_pure_python || has_entry_points {
        sys_requires.insert("platform");
        sys_requires.insert("arch");
    }

    if let Some(req) = metadata.requires_python.as_deref() {
        if let Some(constraint) = pep440_specifier_to_constraint(req) {
            let entry = format!("python@{}", constraint);
            if seen_variant.insert(entry.clone()) {
                variant_requires.push(entry);
            }
        }
    }

    let name_mapping = build_name_mapping(&metadata.dist_names);
    if let Ok(parsed) = parse_requires_dist_lines(&metadata.requires_dist, python_version) {
        for item in parsed {
            let mut to_variant = false;
            if let Some(marker) = item.marker.as_deref() {
                let marker_reqs = marker_sys_requirements(marker);
                if !marker_reqs.is_empty() {
                    to_variant = true;
                    for req in marker_reqs {
                        sys_requires.insert(req);
                    }
                }
            }

            let mut name = item.name.clone();
            if let Some(mapped) = name_mapping.get(&name.to_ascii_lowercase()) {
                name = mapped.clone();
            }
            let normalized = normalize_package_name(&name);

            let req_str = if item.specifier.is_empty() {
                normalized.clone()
            } else if let Some(constraint) = pep440_specifier_to_constraint(&item.specifier) {
                format!("{}@{}", normalized, constraint)
            } else {
                normalized.clone()
            };

            if to_variant {
                if seen_variant.insert(req_str.clone()) {
                    variant_requires.push(req_str);
                }
            } else if seen_requires.insert(req_str.clone()) {
                requires.push(req_str);
            }
        }
    }

    if sys_requires.contains("platform") {
        variant_requires.push(format!("platform-{}", detect_platform()));
    }
    if sys_requires.contains("arch") {
        variant_requires.push(format!("arch-{}", detect_arch()));
    }
    if sys_requires.contains("os") {
        variant_requires.push(format!("os-{}", detect_os()));
    }
    if sys_requires.contains("python") {
        variant_requires.push(format!("python-{}", python_mm));
    }

    PipRequirements {
        requires,
        variant_requires,
    }
}

fn parse_requires_dist_lines(
    lines: &[String],
    python_version: &str,
) -> Result<Vec<ParsedRequirement>, PipError> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let _ = Python::initialize();
    let parsed: Result<Vec<ParsedRequirement>, PipError> = Python::attach(|py| {
        let locals = PyDict::new(py);
        let requires = PyList::new(py, lines).map_err(|e| {
            PipError::Config(format!("pip requirements parse failed: {e}"))
        })?;
        locals.set_item("requires", requires).ok();
        locals
            .set_item("python_version", python_version)
            .ok();

        let code = r#"
import packaging.requirements as _req
from packaging.markers import Marker

def _normalize(req):
    marker_str = str(req.marker) if req.marker else ""
    conditional_extras = set()

    if marker_str and "extra" in marker_str.split():
        marker_str = marker_str.replace(" and ", " \nand ")
        marker_str = marker_str.replace(" or ", " \nor ")
        lines = [x.strip() for x in marker_str.split("\n") if x.strip()]
        new_lines = []
        for line in lines:
            if "extra" in line.split():
                extra = line.split()[-1].strip("'\"")
                conditional_extras.add(extra)
            else:
                new_lines.append(line)
        if new_lines:
            parts = " ".join(new_lines).split()
            if parts and parts[0] in ("and", "or"):
                parts = parts[1:]
            marker_str = " ".join(parts)
        else:
            marker_str = ""

    if conditional_extras:
        return None

    marker_env = {
        "python_full_version": python_version,
        "python_version": ".".join(python_version.split(".")[:2]),
        "implementation_version": python_version,
    }

    if marker_str:
        if not Marker(marker_str).evaluate(environment=marker_env):
            return None

    return (req.name, str(req.specifier), marker_str)

result = []
for raw in requires:
    req = _req.Requirement(raw)
    if req.extras:
        # extras not supported; ignore extras portion
        req = _req.Requirement(req.name + str(req.specifier))
    item = _normalize(req)
    if item:
        result.append(item)
"#;

        let code = std::ffi::CString::new(code).map_err(|e| {
            PipError::Config(format!("pip requirements parse failed: {e}"))
        })?;
        py.run(code.as_c_str(), None, Some(&locals))
            .map_err(|e| PipError::Config(format!("pip requirements parse failed: {e}")))?;
        let result = locals
            .get_item("result")
            .map_err(|e| PipError::Config(format!("pip requirements parse failed: {e}")))?;
        let result = result
            .ok_or_else(|| PipError::Config("pip requirements parse failed".to_string()))?;
        let parsed: Vec<(String, String, String)> = result
            .extract()
            .map_err(|e| PipError::Config(format!("pip requirements parse failed: {e}")))?;

        Ok(parsed
            .into_iter()
            .map(|(name, spec, marker)| ParsedRequirement {
                name,
                specifier: spec,
                marker: if marker.is_empty() { None } else { Some(marker) },
            })
            .collect::<Vec<ParsedRequirement>>())
    });

    match parsed {
        Ok(list) if !list.is_empty() || lines.is_empty() => Ok(list),
        Ok(_) | Err(_) => Ok(parse_requires_dist_fallback(lines)),
    }
}

fn build_name_mapping(names: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for name in names {
        map.insert(name.to_ascii_lowercase(), name.clone());
    }
    map
}

fn marker_sys_requirements(marker: &str) -> Vec<&'static str> {
    let mut sys_requires = Vec::new();
    let parts: Vec<&str> = marker.split_whitespace().collect();

    let mut add = |key: &str, reqs: &[&'static str]| {
        if parts.iter().any(|p| *p == key) {
            for req in reqs {
                if !sys_requires.contains(req) {
                    sys_requires.push(req);
                }
            }
        }
    };

    add("implementation_name", &["python"]);
    add("implementation_version", &["python"]);
    add("platform_python_implementation", &["python"]);
    add("platform.python_implementation", &["python"]);
    add("python_implementation", &["python"]);
    add("sys.platform", &["platform"]);
    add("sys_platform", &["platform"]);
    add("os.name", &["platform"]);
    add("os_name", &["platform"]);
    add("platform.machine", &["arch"]);
    add("platform_machine", &["arch"]);
    add("platform.version", &["platform"]);
    add("platform_version", &["platform"]);
    add("platform_system", &["platform"]);
    add("platform_release", &["platform"]);
    add("python_version", &["python"]);
    add("python_full_version", &["python"]);

    sys_requires
}

fn parse_requires_dist_fallback(lines: &[String]) -> Vec<ParsedRequirement> {
    let mut out = Vec::new();

    for raw in lines {
        let mut parts = raw.splitn(2, ';');
        let req_part = parts.next().unwrap_or("").trim();
        let marker = parts
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        if marker
            .as_deref()
            .map(|m| m.to_ascii_lowercase().contains("extra"))
            .unwrap_or(false)
        {
            continue;
        }

        if req_part.is_empty() {
            continue;
        }

        let mut name_end = req_part.len();
        for (idx, ch) in req_part.char_indices() {
            if ch == ' ' || ch == '(' || ch == '<' || ch == '>' || ch == '=' || ch == '!' || ch == '~' {
                name_end = idx;
                break;
            }
        }

        let mut name = req_part[..name_end].trim();
        if let Some(bracket) = name.find('[') {
            name = &name[..bracket];
        }
        if name.is_empty() {
            continue;
        }

        let mut spec = req_part[name_end..].trim();
        if spec.starts_with('(') && spec.ends_with(')') && spec.len() > 2 {
            spec = &spec[1..spec.len() - 1];
        }
        let spec = spec.trim().to_string();

        out.push(ParsedRequirement {
            name: name.to_string(),
            specifier: spec,
            marker,
        });
    }

    out
}

fn detect_platform() -> String {
    match std::env::consts::OS {
        "macos" => "osx".to_string(),
        other => other.to_string(),
    }
}

fn detect_arch() -> String {
    std::env::consts::ARCH.to_string()
}

fn detect_os() -> String {
    detect_platform()
}

fn pep440_specifier_to_constraint(specifier: &str) -> Option<String> {
    let specifier = specifier.trim();
    if specifier.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for raw in specifier.split(',') {
        let spec = raw.trim();
        if spec.is_empty() {
            continue;
        }
        if let Some(converted) = convert_specifier(spec) {
            parts.push(converted);
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

fn convert_specifier(spec: &str) -> Option<String> {
    let (op, ver) = parse_specifier(spec)?;
    let is_wildcard = ver.ends_with(".*");
    let version = ver.trim_end_matches(".*");
    let normalized = pep440_to_semver(version);

    match op {
        "==" if is_wildcard => {
            let (lower, upper) = wildcard_bounds(&normalized)?;
            Some(format!(">={},<{}", lower, upper))
        }
        "==" => {
            let upper = adjacent_version(&normalized)?;
            Some(format!(">={},<{}", normalized, upper))
        }
        ">=" => Some(format!(">={}", normalized)),
        ">" => {
            let upper = adjacent_version(&normalized)?;
            Some(format!(">={}", upper))
        }
        "<=" => {
            let upper = adjacent_version(&normalized)?;
            Some(format!("<{}", upper))
        }
        "<" => Some(format!("<{}", normalized)),
        "~=" => {
            let upper = compatible_upper_bound(&normalized)?;
            Some(format!(">={},<{}", normalized, upper))
        }
        "!=" if is_wildcard => {
            let (lower, upper) = wildcard_bounds(&normalized)?;
            Some(format!("<{}|>={}", lower, upper))
        }
        "!=" => {
            let upper = adjacent_version(&normalized)?;
            Some(format!("<{}|>={}", normalized, upper))
        }
        _ => None,
    }
}

fn parse_specifier(spec: &str) -> Option<(&str, &str)> {
    for op in ["~=", "===", "==", "!=", ">=", "<=", ">", "<", "="] {
        if let Some(rest) = spec.strip_prefix(op) {
            let op = if op == "===" { "==" } else if op == "=" { "==" } else { op };
            return Some((op, rest.trim()));
        }
    }
    None
}

fn wildcard_bounds(version: &str) -> Option<(String, String)> {
    let parts = split_version_parts(version);
    if parts.is_empty() {
        return None;
    }
    let lower = normalize_parts(&parts, 3);
    if parts.len() == 1 {
        let upper = normalize_parts(&[parts[0] + 1], 3);
        return Some((lower, upper));
    }
    let upper = normalize_parts(&[parts[0], parts[1] + 1], 3);
    Some((lower, upper))
}

fn adjacent_version(version: &str) -> Option<String> {
    let mut parts = split_version_parts(version);
    if parts.is_empty() {
        return None;
    }
    if parts.len() >= 3 {
        parts[2] = parts[2].saturating_add(1);
    } else {
        parts.push(1);
    }
    Some(normalize_parts(&parts, 3))
}

fn compatible_upper_bound(version: &str) -> Option<String> {
    let parts = split_version_parts(version);
    if parts.is_empty() {
        return None;
    }
    if parts.len() <= 2 {
        return Some(normalize_parts(&[parts[0] + 1], 3));
    }
    Some(normalize_parts(&[parts[0], parts[1] + 1], 3))
}

fn split_version_parts(version: &str) -> Vec<u64> {
    let base = version
        .split(|c| c == '-' || c == '+')
        .next()
        .unwrap_or("");
    base.split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
}

fn normalize_parts(parts: &[u64], min_len: usize) -> String {
    let mut out = parts.to_vec();
    while out.len() < min_len {
        out.push(0);
    }
    out.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn python_major_minor_range(version: &str) -> Option<String> {
    let (major, minor) = split_major_minor(version)?;
    Some(format!(">={}.{}.0,<{}.{}.0", major, minor, major, minor + 1))
}

fn pep440_to_semver(version: &str) -> String {
    let v = version.trim();
    if v.is_empty() {
        return "0.0.0".to_string();
    }

    let mut local = None;
    let core_part = if let Some((core, loc)) = v.split_once('+') {
        local = Some(loc.to_string());
        core
    } else {
        v
    };

    let mut core = String::new();
    let mut chars = core_part.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == '.' {
            core.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    let rest: String = chars.collect();

    if core.is_empty() {
        return "0.0.0".to_string();
    }

    let mut nums: Vec<&str> = core.split('.').filter(|s| !s.is_empty()).collect();
    while nums.len() < 3 {
        nums.push("0");
    }
    let mut semver = format!("{}.{}.{}", nums[0], nums[1], nums[2]);

    let mut pre = None;
    let mut build = Vec::new();
    let mut remaining = rest.trim_start_matches(|c: char| c == '.' || c == '-' || c == '_');

    if let Some((tag, value, tail)) = take_tag(remaining) {
        pre = Some(format!("{}.{}", tag, value));
        remaining = tail;
    }

    loop {
        let trimmed = remaining.trim_start_matches(|c: char| c == '.' || c == '-' || c == '_');
        if trimmed.is_empty() {
            break;
        }
        if let Some((tag, value, tail)) = take_post_or_dev(trimmed) {
            build.push(format!("{}.{}", tag, value));
            remaining = tail;
        } else {
            break;
        }
    }

    if let Some(pre) = pre {
        semver.push('-');
        semver.push_str(&pre);
    }

    if let Some(local) = local {
        let sanitized = sanitize_build(&local);
        if !sanitized.is_empty() {
            build.push(sanitized);
        }
    }

    if !build.is_empty() {
        semver.push('+');
        semver.push_str(&build.join("."));
    }

    semver
}

fn take_tag(input: &str) -> Option<(&'static str, String, &str)> {
    let lower = input.to_ascii_lowercase();
    for (tag, label) in [("rc", "rc"), ("a", "alpha"), ("b", "beta"), ("dev", "dev")] {
        if lower.starts_with(tag) {
            let rest = &input[tag.len()..];
            let (num, tail) = take_number(rest);
            let value = if num.is_empty() { "0".to_string() } else { num };
            return Some((label, value, tail));
        }
    }
    None
}

fn take_post_or_dev(input: &str) -> Option<(&'static str, String, &str)> {
    let lower = input.to_ascii_lowercase();
    for (tag, label) in [("post", "post"), ("dev", "dev")] {
        if lower.starts_with(tag) {
            let rest = &input[tag.len()..];
            let (num, tail) = take_number(rest);
            let value = if num.is_empty() { "0".to_string() } else { num };
            return Some((label, value, tail));
        }
    }
    None
}

fn take_number(input: &str) -> (String, &str) {
    let mut digits = String::new();
    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            return (digits, &input[idx..]);
        }
    }
    (digits, "")
}

fn sanitize_build(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn resolve_repo_root(
    storage: &Storage,
    prefix: Option<&PathBuf>,
    release: bool,
) -> Result<PathBuf, PipError> {
    if let Some(prefix) = prefix {
        return Ok(prefix.clone());
    }

    if release {
        if let Some(first) = storage.location_paths().first() {
            return Ok(first.clone());
        }
    }

    if let Some(user_dir) = Storage::user_packages_dir() {
        return Ok(user_dir);
    }

    if let Some(first) = storage.location_paths().first() {
        return Ok(first.clone());
    }

    Err(PipError::Config(
        "no repository path available; use --prefix".to_string(),
    ))
}

fn write_package_py(
    install_root: &Path,
    base_name: &str,
    version: &str,
    requirements: &[String],
    variant_requires: &[String],
    entry_points: &[EntryPoint],
    release: bool,
    description: Option<&str>,
    hashed_variants: bool,
    variant_subpath: Option<&str>,
    pip_name: &str,
    from_pip: bool,
    is_pure_python: bool,
    help: &[Vec<String>],
    authors: &[String],
    tools: &[String],
) -> Result<(), PipError> {
    let mut out = String::new();
    out.push_str("from pkg import Package, Env, Evar, App\n");
    out.push_str("from pathlib import Path\n");
    out.push_str("import sys\n\n");

    out.push_str("def get_package(*args, **kwargs):\n");
    out.push_str(&format!("    pkg = Package(\"{}\", \"{}\")\n", base_name, version));
    out.push_str("    pkg.add_tag(\"pip\")\n");
    if release {
        out.push_str("    pkg.add_tag(\"release\")\n");
    }
    if let Some(desc) = description {
        let escaped = escape_py_string(desc);
        if !escaped.is_empty() {
            out.push_str(&format!("    pkg.description = \"{}\"\n", escaped));
        }
    }
    if !pip_name.is_empty() {
        out.push_str(&format!(
            "    pkg.pip_name = \"{}\"\n",
            escape_py_string(pip_name)
        ));
    }
    if from_pip {
        out.push_str("    pkg.from_pip = True\n");
    }
    out.push_str(&format!(
        "    pkg.is_pure_python = {}\n",
        if is_pure_python { "True" } else { "False" }
    ));
    if !help.is_empty() {
        out.push_str("    pkg.help = [\n");
        for pair in help {
            if pair.len() == 2 {
                out.push_str(&format!(
                    "        [\"{}\", \"{}\"],\n",
                    escape_py_string(&pair[0]),
                    escape_py_string(&pair[1])
                ));
            }
        }
        out.push_str("    ]\n");
    }
    if !authors.is_empty() {
        let items = authors
            .iter()
            .map(|a| format!("\"{}\"", escape_py_string(a)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    pkg.authors = [{}]\n", items));
    }
    if !tools.is_empty() {
        let items = tools
            .iter()
            .map(|t| format!("\"{}\"", escape_py_string(t)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    pkg.tools = [{}]\n", items));
    }

    for req in requirements {
        out.push_str(&format!("    pkg.add_req(\"{}\")\n", req));
    }

    if !variant_requires.is_empty() {
        let items = variant_requires
            .iter()
            .map(|r| format!("\"{}\"", escape_py_string(r)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    pkg.variants = [[{}]]\n", items));
    }
    if hashed_variants {
        out.push_str("    pkg.hashed_variants = True\n");
    }

    if let Some(subpath) = variant_subpath {
        out.push_str(&format!(
            "    ROOT = Path(__file__).resolve().parent / \"{}\"\n",
            escape_py_string(subpath)
        ));
    } else {
        out.push_str("    ROOT = Path(__file__).resolve().parent\n");
    }
    out.push_str("    env = Env(\"default\")\n");
    out.push_str("    env.add(Evar(\"PYTHONPATH\", str(ROOT / \"python\"), \"append\"))\n");
    if !entry_points.is_empty() {
        out.push_str("    env.add(Evar(\"PATH\", str(ROOT / \"bin\"), \"append\"))\n");
    }
    out.push_str("    pkg.add_env(env)\n");

    for entry in entry_points {
        out.push_str("    if sys.platform == \"win32\":\n");
        out.push_str(&format!(
            "        app_path = str(ROOT / \"bin\" / \"{}.cmd\")\n",
            entry.name
        ));
        out.push_str("    else:\n");
        out.push_str(&format!(
            "        app_path = str(ROOT / \"bin\" / \"{}\")\n",
            entry.name
        ));
        out.push_str(&format!(
            "    app = App(\"{}\", path=app_path, env_name=\"default\")\n",
            entry.name
        ));
        out.push_str("    pkg.add_app(app)\n");
    }

    out.push_str("    return pkg\n");

    let package_py = install_root.join("package.py");
    std::fs::create_dir_all(install_root)?;
    std::fs::write(package_py, out)?;

    Ok(())
}

fn escape_py_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn requested_name_from_spec(spec: &str) -> Option<String> {
    if spec.contains("://") {
        return None;
    }
    if spec.contains('\\') || spec.contains('/') {
        return None;
    }

    let mut name = spec;
    for op in ["==", ">=", "<=", "!=", "~=", ">", "<", "="] {
        if let Some(idx) = name.find(op) {
            name = &name[..idx];
            break;
        }
    }

    let name = name.split('[').next().unwrap_or(name).trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn normalize_package_name(name: &str) -> String {
    name.replace('-', "_")
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), PipError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&from, &to)?;
        } else if file_type.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
