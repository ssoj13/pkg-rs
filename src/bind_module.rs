//! Bind modules: pluggable registry of bindable packages (platform, arch, os, python, rez, …).
//!
//! **Extensible design:** each module is a [`BindHandler`](crate::bind::BindHandler). Built-in
//! handlers live in [`crate::bind`]; config `plugins.pkg_rs.bind_modules_extra` adds names
//! (using Python fallback), `plugins.pkg_rs.bind_modules_remove` excludes names.
//!
//! To add a new module: implement `BindHandler`, register in [`crate::bind::builtin_handlers`].

use crate::bind::{self, BindHandler};
use crate::config;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;

/// One bindable package in the registry (name + handler). Built from builtins + config − remove.
pub struct BindModule {
    pub name: String,
    handler: Box<dyn BindHandler>,
}

impl BindModule {
    pub fn strategy_label(&self) -> &'static str {
        self.handler.strategy_label()
    }

    pub fn bind(&self, path: &Path, no_deps: bool) -> Result<(), String> {
        self.handler.bind(&self.name, path, no_deps)
    }
}

/// Build registry: builtin handlers + config extra (Python) − config remove.
pub fn registry() -> Vec<BindModule> {
    let extra = config_extra();
    let remove = config_remove();
    let mut out: Vec<BindModule> = bind::builtin_handlers()
        .into_iter()
        .filter(|(name, _)| !remove.contains(&(*name).to_string()))
        .map(|(name, handler)| BindModule {
            name: name.to_string(),
            handler,
        })
        .collect();
    for name in extra {
        if remove.contains(&name) || out.iter().any(|m| m.name == name) {
            continue;
        }
        out.push(BindModule {
            name,
            handler: Box::new(PythonFallback),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Python fallback: call rez.package_bind.bind_package (for config extra names).
struct PythonFallback;

impl BindHandler for PythonFallback {
    fn bind(&self, name: &str, install_path: &Path, no_deps: bool) -> Result<(), String> {
        bind_python_impl(name, install_path, no_deps)
    }

    fn strategy_label(&self) -> &'static str {
        "python"
    }
}

pub fn list_names() -> Vec<String> {
    registry().into_iter().map(|m| m.name).collect()
}

pub fn search_names(pattern: &str) -> Vec<String> {
    let pat = pattern.to_lowercase();
    registry()
        .into_iter()
        .filter(|m| m.name.to_lowercase().contains(&pat))
        .map(|m| m.name)
        .collect()
}

pub fn bind_one(name: &str, install_path: &Path, no_deps: bool) -> Result<(), String> {
    let modules = registry();
    let module = modules.iter().find(|m| m.name == name).ok_or_else(|| {
        let available = list_names();
        format!(
            "неизвестный модуль bind '{}'; доступны: {}",
            name,
            available.join(", ")
        )
    })?;
    module.bind(install_path, no_deps)
}

pub fn package_version_installed(repo: &Path, name: &str, version: &str) -> bool {
    repo.join(name).join(version).join("package.py").is_file()
}

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

fn bind_python_impl(name: &str, install_path: &Path, no_deps: bool) -> Result<(), String> {
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
/// Used by quickstart for names that use Python strategy (config extra).
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
