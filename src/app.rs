//! Application definition and execution.
//!
//! This module provides the [`App`] struct representing an executable application
//! defined in a package. Apps reference environments by name and contain execution
//! settings like path, arguments, working directory, and custom properties.
//!
//! # Overview
//!
//! An `App` is a named executable (binary, script, or any runnable) that belongs
//! to a package. Each app specifies:
//!
//! - **path**: Absolute path to the executable (OS-agnostic, Python handles platform differences)
//! - **env_name**: Name of the [`Env`](crate::env::Env) to use when launching
//! - **args**: Default command-line arguments
//! - **cwd**: Working directory (defaults to executable's parent directory)
//! - **properties**: Custom metadata (icon, hidden flags, engine type, etc.)
//!
//! # Package.py Example
//!
//! ```python
//! from pkg import App, Env, Package
//! from pathlib import Path
//! import sys
//!
//! def get_package(*args, **kwargs):
//!     pkg = Package("maya", "2026.1.0")
//!
//!     # Create environment
//!     env = Env("default")
//!     env.add(Evar("MAYA_ROOT", str(Path("/opt/autodesk/maya2026"))))
//!     pkg.envs.append(env)
//!
//!     # Create application
//!     if sys.platform == "win32":
//!         exe_path = Path("C:/Program Files/Autodesk/Maya2026/bin/maya.exe")
//!     else:
//!         exe_path = Path("/opt/autodesk/maya2026/bin/maya")
//!
//!     app = App(
//!         name="maya",
//!         path=str(exe_path),
//!         env_name="default",
//!         args=["-noAutoloadPlugins"],
//!         properties={"icon": "maya.png", "engine": "tk-maya"}
//!     )
//!     pkg.apps.append(app)
//!
//!     return pkg
//! ```
//!
//! # Rust Usage
//!
//! ```ignore
//! use pkg::App;
//! use std::collections::HashMap;
//!
//! let mut props = HashMap::new();
//! props.insert("icon".to_string(), "maya.png".to_string());
//!
//! let app = App::new("maya")
//!     .with_path("/opt/maya/bin/maya")
//!     .with_env("default")
//!     .with_args(vec!["-batch"])
//!     .with_properties(props);
//!
//! assert_eq!(app.name, "maya");
//! assert_eq!(app.env_name, Some("default".to_string()));
//! ```
//!
//! # Properties
//!
//! Common property keys (by convention):
//! - `icon`: Path to icon image
//! - `hidden`: Hide from UI
//! - `hidden_sg`: Hide from Shotgrid UI
//! - `engine`: Toolkit engine name (e.g., "tk-maya")
//! - `console`: Open in terminal window
//! - `path_check`: Verify path exists before launch
//!
//! # Serialization
//!
//! ```json
//! {
//!   "name": "maya",
//!   "path": "/opt/maya/bin/maya",
//!   "env_name": "default",
//!   "args": ["-batch"],
//!   "cwd": null,
//!   "properties": {"icon": "maya.png"}
//! }
//! ```

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Application definition within a package.
///
/// Represents an executable that can be launched with a specific environment.
/// The actual launching logic is handled by CLI/GUI, not by App itself.
///
/// # Fields
///
/// - `name`: Application identifier (e.g., "maya", "mayapy", "render")
/// - `path`: Path to executable (can be platform-specific, set in package.py)
/// - `env_name`: Which Env from the package to use (references by name)
/// - `args`: Default arguments passed to the executable
/// - `cwd`: Working directory for launch (None = use executable's parent)
/// - `properties`: Arbitrary key-value metadata for UI and extensions
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    /// Application name/identifier.
    /// Used to reference this app from CLI (e.g., `pkg env maya -- maya`)
    #[pyo3(get, set)]
    pub name: String,

    /// Path to the executable.
    /// Platform-specific - package.py should use sys.platform to set correctly.
    #[pyo3(get, set)]
    pub path: Option<String>,

    /// Name of the Env to use from the package.
    /// Must match an env name in Package.envs.
    #[pyo3(get, set)]
    pub env_name: Option<String>,

    /// Default command-line arguments.
    /// Additional args can be passed at launch time.
    #[pyo3(get, set)]
    pub args: Vec<String>,

    /// Working directory for launch.
    /// If None, defaults to the executable's parent directory.
    #[pyo3(get, set)]
    pub cwd: Option<String>,

    /// Custom properties (icon, hidden, engine, etc.).
    /// Convention-based keys - see module docs for common ones.
    #[pyo3(get, set)]
    pub properties: HashMap<String, String>,
}

#[pymethods]
impl App {
    /// Create a new App with just a name.
    ///
    /// # Arguments
    /// * `name` - Application identifier
    /// * `path` - Optional path to executable
    /// * `env_name` - Optional environment name
    /// * `args` - Optional default arguments
    /// * `cwd` - Optional working directory
    /// * `properties` - Optional custom properties
    #[new]
    #[pyo3(signature = (name, path = None, env_name = None, args = None, cwd = None, properties = None))]
    pub fn new(
        name: String,
        path: Option<String>,
        env_name: Option<String>,
        args: Option<Vec<String>>,
        cwd: Option<String>,
        properties: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            name,
            path,
            env_name,
            args: args.unwrap_or_default(),
            cwd,
            properties: properties.unwrap_or_default(),
        }
    }

    /// Get a property value by key.
    ///
    /// Returns None if key not found.
    pub fn get_prop(&self, key: &str) -> Option<String> {
        self.properties.get(key).cloned()
    }

    /// Set a property value.
    pub fn set_prop(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Remove a property.
    ///
    /// Returns the removed value or None if not found.
    pub fn remove_prop(&mut self, key: &str) -> Option<String> {
        self.properties.remove(key)
    }

    /// Check if a property exists.
    pub fn has_prop(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Get the effective working directory.
    ///
    /// Returns cwd if set, otherwise the parent directory of the executable.
    /// Returns None if neither is available.
    pub fn effective_cwd(&self) -> Option<String> {
        if let Some(ref cwd) = self.cwd {
            return Some(cwd.clone());
        }

        // Try to get parent directory of executable
        self.path
            .as_ref()
            .and_then(|p| Path::new(p).parent())
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Check if the executable path exists.
    ///
    /// Returns false if path is not set.
    pub fn path_exists(&self) -> bool {
        self.path
            .as_ref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false)
    }

    /// Build complete argument list.
    ///
    /// Combines default args with additional args.
    /// Additional args come after default args.
    #[pyo3(signature = (extra_args = None))]
    pub fn build_args(&self, extra_args: Option<Vec<String>>) -> Vec<String> {
        let mut result = self.args.clone();
        if let Some(extra) = extra_args {
            result.extend(extra);
        }
        result
    }

    /// Check if app is marked as hidden.
    ///
    /// Convenience method checking the "hidden" property.
    pub fn is_hidden(&self) -> bool {
        self.properties
            .get("hidden")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    /// Get icon path if set.
    ///
    /// Convenience method for the "icon" property.
    pub fn icon(&self) -> Option<String> {
        self.properties.get("icon").cloned()
    }

    /// Get engine name if set (for integrations like Shotgrid).
    ///
    /// Convenience method for the "engine" property.
    pub fn engine(&self) -> Option<String> {
        self.properties.get("engine").cloned()
    }

    /// Convert to dictionary.
    ///
    /// Returns dict with all fields.
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);

        dict.set_item("name", &self.name)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("env_name", &self.env_name)?;

        let args_list = PyList::new(py, &self.args)?;
        dict.set_item("args", args_list)?;

        dict.set_item("cwd", &self.cwd)?;

        let props = PyDict::new(py);
        for (k, v) in &self.properties {
            props.set_item(k, v)?;
        }
        dict.set_item("properties", props)?;

        Ok(dict.into())
    }

    /// Create from dictionary.
    #[staticmethod]
    pub fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = dict
            .get_item("name")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'name'"))?
            .extract()?;

        let path: Option<String> = dict
            .get_item("path")?
            .and_then(|v| v.extract().ok());

        let env_name: Option<String> = dict
            .get_item("env_name")?
            .and_then(|v| v.extract().ok());

        let args: Vec<String> = dict
            .get_item("args")?
            .map(|v| v.extract().unwrap_or_default())
            .unwrap_or_default();

        let cwd: Option<String> = dict
            .get_item("cwd")?
            .and_then(|v| v.extract().ok());

        let properties: HashMap<String, String> = dict
            .get_item("properties")?
            .map(|v| v.extract().unwrap_or_default())
            .unwrap_or_default();

        Ok(Self {
            name,
            path,
            env_name,
            args,
            cwd,
            properties,
        })
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> PyResult<String> {
        use crate::error::IntoPyErr;
        serde_json::to_string(self).py_err()
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    pub fn from_json(json: &str) -> PyResult<Self> {
        use crate::error::IntoPyErr;
        serde_json::from_str(json).py_err()
    }

    /// String representation for Python
    fn __repr__(&self) -> String {
        format!(
            "App({:?}, path={:?}, env={:?})",
            self.name, self.path, self.env_name
        )
    }

    // === Python Builder Methods (return Self for chaining) ===

    /// Builder: set executable path.
    /// Returns self for method chaining.
    #[pyo3(name = "with_path")]
    fn py_with_path(mut slf: PyRefMut<'_, Self>, path: String) -> PyRefMut<'_, Self> {
        slf.path = Some(path);
        slf
    }

    /// Builder: set environment name.
    /// Returns self for method chaining.
    #[pyo3(name = "with_env")]
    fn py_with_env(mut slf: PyRefMut<'_, Self>, env_name: String) -> PyRefMut<'_, Self> {
        slf.env_name = Some(env_name);
        slf
    }

    /// Builder: set working directory.
    /// Returns self for method chaining.
    #[pyo3(name = "with_cwd")]
    fn py_with_cwd(mut slf: PyRefMut<'_, Self>, cwd: String) -> PyRefMut<'_, Self> {
        slf.cwd = Some(cwd);
        slf
    }

    /// Builder: add argument.
    /// Returns self for method chaining.
    #[pyo3(name = "with_arg")]
    fn py_with_arg(mut slf: PyRefMut<'_, Self>, arg: String) -> PyRefMut<'_, Self> {
        slf.args.push(arg);
        slf
    }

    /// Builder: set property.
    /// Returns self for method chaining.
    #[pyo3(name = "with_property")]
    fn py_with_property(mut slf: PyRefMut<'_, Self>, key: String, value: String) -> PyRefMut<'_, Self> {
        slf.properties.insert(key, value);
        slf
    }

    /// Hash based on name (apps in a package should have unique names)
    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        hasher.finish()
    }

    /// Equality based on name
    fn __eq__(&self, other: &Self) -> bool {
        self.name == other.name
    }

    /// Launch the application with the given environment.
    ///
    /// # Arguments
    /// * `env` - Solved environment to use (optional, uses empty env if None)
    /// * `extra_args` - Additional arguments to pass
    /// * `wait` - Wait for process to complete (default: false)
    ///
    /// # Returns
    /// Process exit code if wait=true, else 0.
    ///
    /// # Examples
    /// ```python
    /// # With Env object
    /// env = pkg.effective_env("maya")
    /// app.launch(env, extra_args=["--batch"])
    ///
    /// # With dict (like subprocess.Popen)
    /// app.launch({"PATH": "/usr/bin", "HOME": "/home/user"})
    ///
    /// # No environment
    /// app.launch()
    /// ```
    #[pyo3(signature = (env = None, extra_args = None, wait = false))]
    pub fn launch(
        &self,
        _py: Python<'_>,
        env: Option<Bound<'_, PyAny>>,
        extra_args: Option<Vec<String>>,
        wait: bool,
    ) -> PyResult<i32> {
        use std::process::Command;

        let Some(exe_path) = &self.path else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                format!("No executable path defined for app: {}", self.name)
            ));
        };

        // Build command
        let mut cmd = Command::new(exe_path);
        
        // Add arguments
        let args = self.build_args(extra_args);
        cmd.args(&args);

        // Set working directory
        if let Some(cwd) = self.effective_cwd() {
            cmd.current_dir(cwd);
        }

        // Apply environment if provided (Env object or dict)
        if let Some(env_obj) = env {
            if let Ok(env) = env_obj.extract::<crate::env::Env>() {
                // It's an Env object
                for evar in &env.evars {
                    cmd.env(&evar.name, &evar.value);
                }
            } else if let Ok(dict) = env_obj.extract::<HashMap<String, String>>() {
                // It's a dict
                for (key, value) in dict {
                    cmd.env(&key, &value);
                }
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "env must be Env object or dict[str, str]"
                ));
            }
        }

        // Launch
        if wait {
            match cmd.status() {
                Ok(status) => Ok(status.code().unwrap_or(-1)),
                Err(e) => Err(pyo3::exceptions::PyOSError::new_err(
                    format!("Failed to launch {}: {}", self.name, e)
                )),
            }
        } else {
            match cmd.spawn() {
                Ok(_) => Ok(0),
                Err(e) => Err(pyo3::exceptions::PyOSError::new_err(
                    format!("Failed to spawn {}: {}", self.name, e)
                )),
            }
        }
    }
}

// Pure Rust API - builder pattern
impl App {
    /// Create App with just name (Rust-only convenience).
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            env_name: None,
            args: Vec::new(),
            cwd: None,
            properties: HashMap::new(),
        }
    }

    /// Builder: set executable path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Builder: set environment name.
    pub fn with_env(mut self, env_name: impl Into<String>) -> Self {
        self.env_name = Some(env_name.into());
        self
    }

    /// Builder: set arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Builder: add single argument.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Builder: set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Builder: set properties map.
    pub fn with_properties(mut self, props: HashMap<String, String>) -> Self {
        self.properties = props;
        self
    }

    /// Builder: add single property.
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Get path as PathBuf if set.
    pub fn path_buf(&self) -> Option<PathBuf> {
        self.path.as_ref().map(PathBuf::from)
    }

    /// Get cwd as PathBuf if set.
    pub fn cwd_path(&self) -> Option<PathBuf> {
        self.cwd.as_ref().map(PathBuf::from)
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: None,
            env_name: None,
            args: Vec::new(),
            cwd: None,
            properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_new() {
        let app = App::named("maya");
        assert_eq!(app.name, "maya");
        assert!(app.path.is_none());
        assert!(app.args.is_empty());
    }

    #[test]
    fn app_builder() {
        let app = App::named("maya")
            .with_path("/opt/maya/bin/maya")
            .with_env("default")
            .with_arg("-batch")
            .with_property("icon", "maya.png");

        assert_eq!(app.name, "maya");
        assert_eq!(app.path, Some("/opt/maya/bin/maya".to_string()));
        assert_eq!(app.env_name, Some("default".to_string()));
        assert_eq!(app.args, vec!["-batch"]);
        assert_eq!(app.properties.get("icon"), Some(&"maya.png".to_string()));
    }

    #[test]
    fn app_effective_cwd() {
        // With explicit cwd
        let app1 = App::named("test").with_cwd("/custom/dir");
        assert_eq!(app1.effective_cwd(), Some("/custom/dir".to_string()));

        // Without cwd, uses parent of path
        let app2 = App::named("test").with_path("/opt/maya/bin/maya");
        let cwd = app2.effective_cwd().unwrap();
        assert!(cwd.contains("bin") || cwd.ends_with("bin"));

        // Without either
        let app3 = App::named("test");
        assert!(app3.effective_cwd().is_none());
    }

    #[test]
    fn app_build_args() {
        let app = App::named("maya").with_args(vec!["-batch".to_string()]);

        // No extra args
        let args1 = app.build_args(None);
        assert_eq!(args1, vec!["-batch"]);

        // With extra args
        let args2 = app.build_args(Some(vec!["-file".to_string(), "scene.ma".to_string()]));
        assert_eq!(args2, vec!["-batch", "-file", "scene.ma"]);
    }

    #[test]
    fn app_properties() {
        let mut app = App::named("maya");
        assert!(!app.is_hidden());

        app.set_prop("hidden".to_string(), "true".to_string());
        assert!(app.is_hidden());

        app.set_prop("icon".to_string(), "maya.png".to_string());
        assert_eq!(app.icon(), Some("maya.png".to_string()));

        app.remove_prop("hidden");
        assert!(!app.is_hidden());
    }

    #[test]
    fn app_serialization() {
        let app = App::named("maya")
            .with_path("/opt/maya/bin/maya")
            .with_env("default")
            .with_property("icon", "maya.png");

        let json = serde_json::to_string(&app).unwrap();
        let app2: App = serde_json::from_str(&json).unwrap();

        assert_eq!(app, app2);
    }

    #[test]
    fn app_equality() {
        let app1 = App::named("maya").with_path("/path");
        let app2 = App::named("maya").with_path("/path");
        let app3 = App::named("houdini");

        // Full equality (all fields)
        assert_eq!(app1, app2);
        assert_ne!(app1, app3);

        // Different path = not equal
        let app4 = App::named("maya").with_path("/other");
        assert_ne!(app1, app4);
    }
}
