//! Bind modules: pluggable registry of bindable packages (platform, arch, os, python, rez, …).
//!
//! Each module has a strategy: **Native** (Rust: detect version, write package.py) or **Python**
//! (call `rez.package_bind.bind_package`). Registry = built-in list + config `plugins.pkg_rs.bind_modules_extra`
//! (add names, Python strategy) − config `plugins.pkg_rs.bind_modules_remove` (exclude names).

use crate::config;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;

/// How a bind module is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindStrategy {
    /// Native Rust: detect version, write package.py.
    Native,
    /// Use Python `rez.package_bind.bind_package(name)`.
    Python,
}

/// One bindable package in the registry.
#[derive(Debug, Clone)]
pub struct BindModule {
    pub name: String,
    pub strategy: BindStrategy,
}

const BUILTIN_NATIVE: &[&str] = &["platform", "arch", "os"];
const BUILTIN_PYTHON: &[&str] = &["python", "rez", "rezgui", "setuptools", "pip"];

/// Build registry: built-in (native + python) + config extra (Python) − config remove.
pub fn registry() -> Vec<BindModule> {
    let extra = config_extra();
    let remove = config_remove();
    let mut out = Vec::new();
    for name in BUILTIN_NATIVE {
        if !remove.contains(&(*name).to_string()) {
            out.push(BindModule {
                name: (*name).to_string(),
                strategy: BindStrategy::Native,
            });
        }
    }
    for name in BUILTIN_PYTHON {
        if !remove.contains(&(*name).to_string()) {
            out.push(BindModule {
                name: (*name).to_string(),
                strategy: BindStrategy::Python,
            });
        }
    }
    for name in extra {
        if !remove.contains(&name) && !out.iter().any(|m| m.name == name) {
            out.push(BindModule {
                name,
                strategy: BindStrategy::Python,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// List all registered module names.
pub fn list_names() -> Vec<String> {
    registry().into_iter().map(|m| m.name).collect()
}

/// Search names by substring (case-insensitive).
pub fn search_names(pattern: &str) -> Vec<String> {
    let pat = pattern.to_lowercase();
    registry()
        .into_iter()
        .filter(|m| m.name.to_lowercase().contains(&pat))
        .map(|m| m.name)
        .collect()
}

/// Bind one package by name into install_path. Returns Ok(()) on success.
pub fn bind_one(
    name: &str,
    install_path: &Path,
    no_deps: bool,
) -> Result<(), String> {
    let modules = registry();
    let module = modules.iter().find(|m| m.name == name).ok_or_else(|| {
        let available: Vec<String> = list_names();
        format!(
            "unknown bind module '{}'; available: {}",
            name,
            available.join(", ")
        )
    })?;

    match module.strategy {
        BindStrategy::Native => bind_native(name, install_path),
        BindStrategy::Python => bind_python(name, install_path, no_deps),
    }
}

/// Check if a package version is already installed at repo.
pub fn package_version_installed(repo: &Path, name: &str, version: &str) -> bool {
    repo.join(name).join(version).join("package.py").is_file()
}

/// Check if any version of the package is already in repo (for quickstart skip).
pub fn package_already_installed(repo: &Path, name: &str) -> bool {
    let base = repo.join(name);
    if !base.is_dir() {
        return false;
    }
    if base.join("package.py").is_file() {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("package.py").is_file() {
                return true;
            }
            if path.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&path) {
                    for s in sub.flatten() {
                        if s.path().join("package.py").is_file() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn config_extra() -> Vec<String> {
    let cfg = match config::get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let data = match config::get_json(cfg, "plugins.pkg_rs.bind_modules_extra") {
        Some(d) => d,
        None => return Vec::new(),
    };
    let arr = match data.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

fn config_remove() -> Vec<String> {
    let cfg = match config::get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let data = match config::get_json(cfg, "plugins.pkg_rs.bind_modules_remove") {
        Some(d) => d,
        None => return Vec::new(),
    };
    let arr = match data.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

fn bind_native(name: &str, repo: &Path) -> Result<(), String> {
    let version = detect_version_native(name)?;
    if package_version_installed(repo, name, &version) {
        return Ok(());
    }
    write_basic_package(repo, name, &version)
}

fn detect_version_native(name: &str) -> Result<String, String> {
    match name {
        "platform" => Ok(std::env::consts::OS.to_string()),
        "arch" => Ok(detect_arch()),
        "os" => Ok(detect_os()),
        _ => Err(format!("native bind not implemented for '{}'", name)),
    }
}

fn detect_arch() -> String {
    if cfg!(windows) {
        std::env::var("PROCESSOR_ARCHITECTURE")
            .unwrap_or_else(|_| std::env::consts::ARCH.to_string())
    } else {
        std::env::consts::ARCH.to_string()
    }
}

fn detect_os() -> String {
    if cfg!(windows) {
        detect_windows_version()
            .map(|v| format!("windows-{}", v))
            .unwrap_or_else(|| "windows".to_string())
    } else if cfg!(target_os = "macos") {
        detect_macos_version()
            .map(|v| format!("osx-{}", v))
            .unwrap_or_else(|| "osx".to_string())
    } else if cfg!(target_os = "linux") {
        detect_linux_version().unwrap_or_else(|| "linux".to_string())
    } else {
        std::env::consts::OS.to_string()
    }
}

#[cfg(windows)]
fn detect_windows_version() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()?;
    let major: u32 = key.get_value("CurrentMajorVersionNumber").ok().unwrap_or(10);
    let minor: u32 = key.get_value("CurrentMinorVersionNumber").ok().unwrap_or(0);
    let build: String = key
        .get_value("CurrentBuildNumber")
        .ok()
        .unwrap_or_else(|| "0".to_string());
    Some(format!("{}.{}.{}", major, minor, build))
}

#[cfg(not(windows))]
fn detect_windows_version() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn detect_macos_version() -> Option<String> {
    let out = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_macos_version() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn detect_linux_version() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    let mut id = None;
    let mut version = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("ID=") {
            id = Some(rest.trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
            version = Some(rest.trim_matches('"').to_string());
        }
    }
    if let (Some(id), Some(version)) = (id, version) {
        Some(format!("{}-{}", id, version))
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn detect_linux_version() -> Option<String> {
    None
}

fn write_basic_package(repo: &Path, name: &str, version: &str) -> Result<(), String> {
    let pkg_dir = repo.join(name).join(version);
    std::fs::create_dir_all(&pkg_dir).map_err(|e| e.to_string())?;
    let package_py = pkg_dir.join("package.py");
    let content = format!(
        "from pkg import Package\n\n\ndef get_package():\n    pkg = Package(\"{}\", \"{}\")\n    return pkg\n",
        name, version
    );
    std::fs::write(package_py, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn bind_python(name: &str, install_path: &Path, no_deps: bool) -> Result<(), String> {
    let _ = pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        crate::py::ensure_rez_on_sys_path(py).map_err(|e| e.to_string())?;
        crate::py::ensure_python_executable(py).map_err(|e| e.to_string())?;

        let package_bind = py.import("rez.package_bind").map_err(|e| e.to_string())?;
        let bind_package = package_bind.getattr("bind_package").map_err(|e| e.to_string())?;

        let path_str = install_path.to_string_lossy().to_string();
        let kwargs = PyDict::new(py);
        kwargs.set_item("path", path_str).map_err(|e| e.to_string())?;
        kwargs.set_item("no_deps", no_deps).map_err(|e| e.to_string())?;
        kwargs.set_item("quiet", true).map_err(|e| e.to_string())?;

        bind_package
            .call((name,), Some(&kwargs))
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

/// Run Python bind_package for multiple names; if any were installed, call _print_package_list.
/// Used by quickstart for all Python-strategy modules.
pub fn run_python_bind_batch(
    names: &[String],
    install_path: &Path,
    no_deps: bool,
    quiet: bool,
) -> Result<(), String> {
    if names.is_empty() {
        return Ok(());
    }
    let path_str = install_path.to_string_lossy().to_string();
    let _ = pyo3::Python::initialize();

    pyo3::Python::attach(|py| {
        crate::py::ensure_rez_on_sys_path(py).map_err(|e| e.to_string())?;
        crate::py::ensure_python_executable(py).map_err(|e| e.to_string())?;

        let package_bind = py.import("rez.package_bind").map_err(|e| e.to_string())?;
        let bind_package = package_bind.getattr("bind_package").map_err(|e| e.to_string())?;
        let print_list = package_bind.getattr("_print_package_list").map_err(|e| e.to_string())?;

        let installed_variants = PyList::empty(py);
        for name in names {
            let kwargs = PyDict::new(py);
            kwargs.set_item("path", path_str.clone()).map_err(|e| e.to_string())?;
            kwargs.set_item("no_deps", no_deps).map_err(|e| e.to_string())?;
            kwargs.set_item("quiet", quiet).map_err(|e| e.to_string())?;

            match bind_package.call((name.as_str(),), Some(&kwargs)) {
                Ok(variants) => {
                    let list = variants.cast::<PyList>().map_err(|e| e.to_string())?;
                    for item in list.iter() {
                        installed_variants.append(item).map_err(|e| e.to_string())?;
                    }
                }
                Err(err) => {
                    if err.is_instance_of::<pyo3::exceptions::PyFileExistsError>(py) {
                        if !quiet {
                            eprintln!("Skipping {} (already exists)", name);
                        }
                        continue;
                    }
                    return Err(err.to_string());
                }
            }
        }

        if installed_variants.len() > 0 {
            print_list.call1((installed_variants,)).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}
