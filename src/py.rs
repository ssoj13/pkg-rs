//! Python runtime helpers for Rez compatibility.

use pyo3::prelude::*;
use pyo3::types::PyList;
use std::env;
use std::path::{Path, PathBuf};

pub fn find_python_root() -> Result<PathBuf, String> {
    if let Ok(raw) = env::var("PKG_PYTHON_PATH") {
        let path = PathBuf::from(raw);
        if has_rezconfig(&path) {
            return Ok(path);
        }
        return Err(format!("PKG_PYTHON_PATH does not contain rez/rezconfig.py: {}", path.display()));
    }

    if let Ok(raw) = env::var("REZ_PYTHON_PATH") {
        let path = PathBuf::from(raw);
        if has_rezconfig(&path) {
            return Ok(path);
        }
        return Err(format!("REZ_PYTHON_PATH does not contain rez/rezconfig.py: {}", path.display()));
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(found) = search_upwards(&exe) {
            return Ok(found);
        }
    }

    if let Ok(cwd) = env::current_dir() {
        if let Some(found) = search_upwards(&cwd) {
            return Ok(found);
        }
    }

    Err("unable to locate python/rez package root".to_string())
}

pub fn ensure_rez_on_sys_path(py: Python<'_>) -> PyResult<()> {
    let root = find_python_root().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("rez python root not found: {e}"))
    })?;
    let root_str = root.to_string_lossy().to_string();

    let sys = py.import("sys")?;
    let path_obj = sys.getattr("path")?;
    let path_list = path_obj.cast::<PyList>()?;

    let mut found = false;
    for item in path_list.iter() {
        if let Ok(value) = item.extract::<String>() {
            if value == root_str {
                found = true;
                break;
            }
        }
    }

    if !found {
        path_list.insert(0, root_str.clone())?;
    }

    let rezplugins_path = root.join("rezplugins");
    if !rezplugins_path.is_dir() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "rezplugins not found at {} (expected embedded Rez plugins)",
            rezplugins_path.display()
        )));
    }

    Ok(())
}

pub fn ensure_python_executable(py: Python<'_>) -> PyResult<()> {
    let code = r#"
import os, sys, shutil

candidate = os.environ.get("PKG_PYTHON_EXE") or os.environ.get("REZ_PYTHON_EXE")
if not candidate:
    candidate = shutil.which("python") or shutil.which("python3")
if candidate:
    sys.executable = candidate
"#;
    let code = std::ffi::CString::new(code).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("invalid python code: {e}"))
    })?;
    py.run(&code, None, None)
}

fn has_rezconfig(root: &Path) -> bool {
    root.join("rez").join("rezconfig.py").exists()
}

fn search_upwards(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let python_root = current.join("python");
        if has_rezconfig(&python_root) {
            return Some(python_root);
        }
        if has_rezconfig(&current) {
            return Some(current.clone());
        }
        if !current.pop() {
            break;
        }
    }

    None
}
