//! Package.py loading and execution.
//!
//! This module provides [`Loader`] for loading and executing `package.py` files
//! to extract [`Package`] definitions. The loader injects pkg classes
//! into the Python execution context.
//!
//! # Overview
//!
//! Package definitions are written in Python (`package.py` files) that define
//! a `get_package()` function. The loader:
//!
//! 1. Reads the `package.py` file
//! 2. Creates a Python execution context with pkg classes injected
//! 3. Executes the file to define `get_package()`
//! 4. Calls `get_package(*args, **kwargs)` with optional arguments
//! 5. Converts the returned `Package` object to Rust
//!
//! # Package.py Contract
//!
//! A valid `package.py` must define a `get_package` function:
//!
//! ```python
//! from pkg import Package, Env, Evar, App
//! from pathlib import Path
//! import sys
//!
//! def get_package(*args, **kwargs):
//!     """Return package definition.
//!
//!     Args:
//!         *args: Positional arguments (context-specific)
//!         **kwargs: Keyword arguments (context-specific)
//!             - project: Current project name
//!             - user: Current user
//!             - platform: Override platform detection
//!
//!     Returns:
//!         Package object with envs, apps, and requirements.
//!     """
//!     pkg = Package("maya", "2026.1.0")
//!
//!     # Platform-specific configuration
//!     if sys.platform == "win32":
//!         root = Path("C:/Program Files/Autodesk/Maya2026")
//!     else:
//!         root = Path("/opt/autodesk/maya2026")
//!
//!     env = Env("default")
//!     env.add(Evar("MAYA_ROOT", str(root), action="set"))
//!     pkg.envs.append(env)
//!
//!     return pkg
//! ```
//!
//! # Module Registration
//!
//! The loader registers `pkg` module in `sys.modules` with these classes:
//! - `Package` - Package definition class
//! - `Env` - Environment class
//! - `Evar` - Environment variable class
//! - `App` - Application definition class
//! - `Action` - Environment variable action enum
//!
//! Use `from pkg import *` or `from pkg import Package, Env, Evar, App`.
//!
//! Standard library modules (`pathlib`, `sys`, `os`) are also pre-imported.
//!
//! # Usage
//!
//! ```ignore
//! use pkg::Loader;
//! use std::path::Path;
//!
//! let loader = Loader::new();
//!
//! // Load a package
//! let pkg = loader.load(Path::new("/packages/maya/2026.1.0/package.py"))?;
//! println!("Loaded: {}", pkg.name);
//!
//! // Load with arguments
//! let pkg = loader.load_with_args(
//!     Path::new("/packages/maya/2026.1.0/package.py"),
//!     &[],
//!     &[("project", "my_project")],
//! )?;
//! ```
//!
//! # Python API
//!
//! ```python
//! from pkg import Loader
//!
//! loader = Loader()
//! pkg = loader.load("/path/to/package.py")
//!
//! # With arguments
//! pkg = loader.load_with_kwargs(
//!     "/path/to/package.py",
//!     project="my_project",
//!     user="artist"
//! )
//! ```

use crate::app::App;
use crate::env::Env;
use crate::error::LoaderError;
use crate::evar::{Action, Evar};
use crate::package::Package;
use log::{debug, trace};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::ffi::CString;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Extract full Python traceback from PyErr.
fn format_py_error(py: Python<'_>, err: &PyErr) -> String {
    // Try to get formatted traceback using traceback module
    if let Ok(tb_mod) = py.import("traceback") {
        if let Ok(format_exc) = tb_mod.getattr("format_exception") {
            let err_type = err.get_type(py);
            let err_value = err.value(py);
            let err_tb = err.traceback(py);
            
            if let Ok(lines) = format_exc.call1((err_type, err_value, err_tb)) {
                if let Ok(lines_list) = lines.extract::<Vec<String>>() {
                    return lines_list.join("");
                }
            }
        }
    }
    // Fallback to simple error message
    err.to_string()
}

/// Package.py loader.
///
/// Executes `package.py` files and extracts Package definitions.
/// Uses the Python interpreter linked at compile time (via PyO3).
///
/// # Thread Safety
///
/// The loader acquires the Python GIL for each operation.
/// Multiple loaders can be used, but only one can execute at a time.
#[pyclass]
#[derive(Debug, Clone)]
pub struct Loader {
    /// Cache of loaded packages by path.
    cache: HashMap<PathBuf, Package>,

    /// Whether to use caching.
    use_cache: bool,
}

#[pymethods]
impl Loader {
    /// Create a new loader.
    ///
    /// # Arguments
    /// * `use_cache` - Whether to cache loaded packages (default: true)
    #[new]
    #[pyo3(signature = (use_cache = None))]
    pub fn new(use_cache: Option<bool>) -> Self {
        Self {
            cache: HashMap::new(),
            use_cache: use_cache.unwrap_or(true),
        }
    }

    /// Load a package from file.
    ///
    /// # Arguments
    /// * `path` - Path to package.py file
    /// * `kwargs` - Optional keyword arguments for get_package()
    ///
    /// # Returns
    /// Loaded Package or error.
    ///
    /// # Examples
    /// ```python
    /// loader = Loader()
    /// pkg = loader.load("repo/maya/2026.1.0/package.py")
    /// pkg = loader.load("repo/maya/2026.1.0/package.py", platform="linux")
    /// ```
    #[pyo3(signature = (path, **kwargs))]
    pub fn load(
        &mut self,
        path: &str,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Package> {
        let kwargs_map: HashMap<String, String> = kwargs
            .map(|d| {
                d.iter()
                    .filter_map(|(k, v)| {
                        let key: String = k.extract().ok()?;
                        let val: String = v.extract().ok()?;
                        Some((key, val))
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.load_impl(Path::new(path), &[], &kwargs_map)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Clear the package cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Check if a path is cached.
    pub fn is_cached(&self, path: &str) -> bool {
        self.cache.contains_key(Path::new(path))
    }

    fn __repr__(&self) -> String {
        format!(
            "Loader(cache={}, cached={})",
            self.use_cache,
            self.cache.len()
        )
    }
}

// Pure Rust API
impl Loader {
    /// Load a package (Rust API).
    pub fn load_path(&mut self, path: &Path) -> Result<Package, LoaderError> {
        self.load_impl(path, &[], &HashMap::new())
    }

    /// Load with full arguments.
    pub fn load_with_args(
        &mut self,
        path: &Path,
        args: &[String],
        kwargs: &HashMap<String, String>,
    ) -> Result<Package, LoaderError> {
        self.load_impl(path, args, kwargs)
    }

    /// Internal load implementation.
    fn load_impl(
        &mut self,
        path: &Path,
        args: &[String],
        kwargs: &HashMap<String, String>,
    ) -> Result<Package, LoaderError> {
        // Check cache
        if self.use_cache {
            if let Some(cached) = self.cache.get(path) {
                return Ok(cached.clone());
            }
        }

        // Validate path
        if !path.exists() {
            return Err(LoaderError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        // Read file
        let code = std::fs::read_to_string(path).map_err(|e| LoaderError::ReadError {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        // Execute and get package
        let pkg = self.execute_package_py(&code, path, args, kwargs)?;

        // Cache result
        if self.use_cache {
            self.cache.insert(path.to_path_buf(), pkg.clone());
        }

        Ok(pkg)
    }

    /// Execute package.py code and return Package.
    fn execute_package_py(
        &self,
        code: &str,
        path: &Path,
        args: &[String],
        kwargs: &HashMap<String, String>,
    ) -> Result<Package, LoaderError> {
        debug!("Loader: executing {}", path.display());
        trace!("Loader: code length={} args={:?} kwargs={:?}", code.len(), args, kwargs);

        Python::attach(|py| {
            // Create execution globals with injected classes
            trace!("Loader: creating Python globals");
            let globals = self.create_globals(py, path)?;

            // Execute the code using CString
            let code_cstr = CString::new(code.as_bytes()).map_err(|e| {
                LoaderError::ExecutionError {
                    path: path.to_path_buf(),
                    reason: format!("Invalid code (null byte): {}", e),
                }
            })?;
            if let Err(e) = py.run(code_cstr.as_c_str(), Some(&globals), None) {
                let traceback = format_py_error(py, &e);
                return Err(LoaderError::ExecutionError {
                    path: path.to_path_buf(),
                    reason: format!("Python error:\n{}", traceback),
                });
            }

            // Get get_package function
            let get_package = globals.get_item("get_package").map_err(|_e| {
                LoaderError::MissingFunction {
                    path: path.to_path_buf(),
                    function: "get_package".to_string(),
                }
            })?;

            let get_package = get_package.ok_or_else(|| LoaderError::MissingFunction {
                path: path.to_path_buf(),
                function: "get_package".to_string(),
            })?;

            // Build arguments
            let py_args = PyTuple::new(py, args.iter().map(|s| s.as_str()))
                .map_err(|e| LoaderError::ExecutionError {
                    path: path.to_path_buf(),
                    reason: format!("Failed to create args tuple: {}", e),
                })?;
            let py_kwargs = PyDict::new(py);
            for (k, v) in kwargs {
                py_kwargs.set_item(k, v).ok();
            }

            // Call get_package
            let result = match get_package.call(py_args, Some(&py_kwargs)) {
                Ok(r) => r,
                Err(e) => {
                    let traceback = format_py_error(py, &e);
                    return Err(LoaderError::ExecutionError {
                        path: path.to_path_buf(),
                        reason: format!("get_package() error:\n{}", traceback),
                    });
                }
            };

            // Convert result to Package
            self.extract_package(py, &result, path)
        })
    }

    /// Create Python globals with injected classes.
    fn create_globals<'py>(
        &self,
        py: Python<'py>,
        path: &Path,
    ) -> Result<Bound<'py, PyDict>, LoaderError> {
        let globals = PyDict::new(py);

        // Add builtins
        let builtins = py.import("builtins").map_err(|e| LoaderError::ExecutionError {
            path: path.to_path_buf(),
            reason: format!("Cannot import builtins: {}", e),
        })?;
        globals.set_item("__builtins__", builtins).map_err(|e| {
            LoaderError::ExecutionError {
                path: path.to_path_buf(),
                reason: format!("Cannot set builtins: {}", e),
            }
        })?;

        // Set __file__ for the script
        globals.set_item("__file__", path.to_string_lossy().to_string()).ok();

        // Create and register 'pkg' module in sys.modules
        // This allows package.py to use: from pkg import Package, Env, ...
        let pkg_module = PyModule::new(py, "pkg").map_err(|e| LoaderError::ExecutionError {
            path: path.to_path_buf(),
            reason: format!("Cannot create pkg module: {}", e),
        })?;
        pkg_module.add_class::<Package>().ok();
        pkg_module.add_class::<Env>().ok();
        pkg_module.add_class::<Evar>().ok();
        pkg_module.add_class::<App>().ok();
        pkg_module.add_class::<Action>().ok();

        // Add __all__ for 'from pkg import *' support
        let all_exports = vec!["Package", "Env", "Evar", "App", "Action"];
        pkg_module.add("__all__", all_exports).ok();

        // Register in sys.modules so 'from pkg import ...' works
        let sys_modules = py.import("sys")
            .and_then(|sys| sys.getattr("modules"))
            .map_err(|e| LoaderError::ExecutionError {
                path: path.to_path_buf(),
                reason: format!("Cannot get sys.modules: {}", e),
            })?;
        sys_modules.set_item("pkg", &pkg_module).map_err(|e| LoaderError::ExecutionError {
            path: path.to_path_buf(),
            reason: format!("Cannot register pkg module: {}", e),
        })?;

        // Add pkg module to globals for pkg.Package(...) style
        globals.set_item("pkg", &pkg_module).ok();

        // Also inject classes directly for convenience (Package(...) without import)
        // Both styles work:
        //   - pkg.Package("name", "1.0.0")    - namespace style
        //   - Package("name", "1.0.0")        - direct style
        //   - from pkg import Package          - explicit import
        globals.set_item("Package", py.get_type::<Package>()).ok();
        globals.set_item("Env", py.get_type::<Env>()).ok();
        globals.set_item("Evar", py.get_type::<Evar>()).ok();
        globals.set_item("App", py.get_type::<App>()).ok();
        globals.set_item("Action", py.get_type::<Action>()).ok();

        // Add common imports (pathlib, sys, os)
        let pathlib = py.import("pathlib").ok();
        let sys = py.import("sys").ok();
        let os = py.import("os").ok();

        if let Some(m) = pathlib {
            if let Ok(path_class) = m.getattr("Path") {
                globals.set_item("Path", path_class).ok();
            }
            globals.set_item("pathlib", m).ok();
        }
        if let Some(m) = sys {
            globals.set_item("sys", m).ok();
        }
        if let Some(m) = os {
            globals.set_item("os", m).ok();
        }

        Ok(globals)
    }

    /// Extract Package from Python object.
    fn extract_package<'py>(
        &self,
        _py: Python<'py>,
        obj: &Bound<'py, PyAny>,
        path: &Path,
    ) -> Result<Package, LoaderError> {
        // Try direct extraction (if it's already our Package type)
        if let Ok(pkg) = obj.extract::<Package>() {
            return Ok(pkg);
        }

        // Try dict extraction
        if let Ok(dict) = obj.cast::<PyDict>() {
            return Package::from_dict(dict).map_err(|e| LoaderError::InvalidReturn {
                path: path.to_path_buf(),
                reason: format!("Invalid dict format: {}", e),
            });
        }

        // Try to call to_dict() method
        if let Ok(to_dict) = obj.getattr("to_dict") {
            if let Ok(dict_obj) = to_dict.call0() {
                if let Ok(dict) = dict_obj.cast::<PyDict>() {
                    return Package::from_dict(dict).map_err(|e| LoaderError::InvalidReturn {
                        path: path.to_path_buf(),
                        reason: format!("Invalid to_dict() result: {}", e),
                    });
                }
            }
        }

        Err(LoaderError::InvalidReturn {
            path: path.to_path_buf(),
            reason: format!(
                "get_package() must return Package or dict, got: {}",
                obj.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "unknown".to_string())
            ),
        })
    }

    /// Load package from string (for testing).
    pub fn load_from_string(
        &mut self,
        code: &str,
        virtual_path: &str,
    ) -> Result<Package, LoaderError> {
        let path = Path::new(virtual_path);
        self.execute_package_py(code, path, &[], &HashMap::new())
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new(Some(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_new() {
        let loader = Loader::new(None);
        assert!(loader.use_cache);
        assert_eq!(loader.cache_size(), 0);
    }

    #[test]
    fn loader_no_cache() {
        let loader = Loader::new(Some(false));
        assert!(!loader.use_cache);
    }

    // Note: Tests that require actual Python execution need
    // Python to be available at runtime. These are better suited
    // for integration tests.
}
