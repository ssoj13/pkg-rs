//! In-memory package repository.

use super::PackageRepository;
use crate::package::Package;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static MEMORY_REPO_COUNTER: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone)]
pub struct MemoryRepository {
    location: PathBuf,
    packages: Vec<Package>,
    warnings: Vec<String>,
}

impl MemoryRepository {
    pub fn new(packages: Vec<Package>) -> Self {
        let id = MEMORY_REPO_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            location: PathBuf::from(format!("memory{{{}}}", id)),
            packages,
            warnings: Vec::new(),
        }
    }

    /// Create memory repository from Rez-style dict structure.
    pub fn from_data(data: JsonValue) -> Result<Self, String> {
        let mut warnings = Vec::new();
        let mut packages = Vec::new();
        let id = MEMORY_REPO_COUNTER.fetch_add(1, Ordering::Relaxed);

        let obj = data
            .as_object()
            .ok_or_else(|| "memory repository data must be a mapping".to_string())?;

        for (name, versions) in obj {
            let versions_obj = match versions.as_object() {
                Some(v) => v,
                None => {
                    warnings.push(format!("invalid package entry for '{}'", name));
                    continue;
                }
            };

            for (version, pkg_data) in versions_obj {
                let mut pkg_obj = match pkg_data.as_object() {
                    Some(v) => v.clone(),
                    None => {
                        warnings.push(format!("invalid package data for '{}'", name));
                        continue;
                    }
                };

                if !pkg_obj.contains_key("name") {
                    pkg_obj.insert("name".to_string(), JsonValue::String(name.clone()));
                }
                if !pkg_obj.contains_key("version") {
                    if version != "_NO_VERSION" {
                        pkg_obj.insert("version".to_string(), JsonValue::String(version.clone()));
                    }
                }

                if version == "_NO_VERSION" {
                    warnings.push(format!("unversioned package '{}' is not fully supported", name));
                    continue;
                }

                match package_from_json(&pkg_obj) {
                    Ok(pkg) => packages.push(pkg),
                    Err(err) => warnings.push(format!("{}: {}", name, err)),
                }
            }
        }

        Ok(Self {
            location: PathBuf::from(format!("memory{{{}}}", id)),
            packages,
            warnings,
        })
    }

    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

impl PackageRepository for MemoryRepository {
    fn repo_type(&self) -> &'static str {
        "memory"
    }

    fn location(&self) -> &PathBuf {
        &self.location
    }

    fn packages(&self) -> &[Package] {
        &self.packages
    }

    fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

fn package_from_json(data: &serde_json::Map<String, JsonValue>) -> Result<Package, String> {
    let _ = pyo3::Python::initialize();
    Python::attach(|py| {
        let dict = PyDict::new(py);
        for (k, v) in data {
            let value = json_to_py(py, v).map_err(|e| e.to_string())?;
            dict.set_item(k, value).map_err(|e| e.to_string())?;
        }
        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
            dict.set_item("base", name).map_err(|e| e.to_string())?;
        }
        let pkg = Package::from_dict(&dict).map_err(|e| e.to_string())?;
        Ok(pkg)
    })
}

fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<Py<PyAny>> {
    Ok(match value {
        JsonValue::Null => py.None(),
        JsonValue::Bool(b) => b.into_py_any(py)?,
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py_any(py)?
            } else if let Some(u) = n.as_u64() {
                u.into_py_any(py)?
            } else if let Some(f) = n.as_f64() {
                f.into_py_any(py)?
            } else {
                py.None()
            }
        }
        JsonValue::String(s) => s.as_str().into_py_any(py)?,
        JsonValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            list.into()
        }
        JsonValue::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            dict.into()
        }
    })
}
