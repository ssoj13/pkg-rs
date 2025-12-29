//! Environment collection and operations.
//!
//! This module provides the [`Env`] struct - a named collection of [`Evar`]s
//! with operations for merging, compression, and token expansion.
//!
//! # Overview
//!
//! An `Env` represents a complete environment configuration that can be:
//! - Merged with other environments (combining Evars)
//! - Compressed (collapsing same-name Evars using action semantics)
//! - Solved (expanding `{TOKEN}` references recursively)
//! - Committed (applied to the current process)
//!
//! # Workflow
//!
//! 1. Create envs and add evars
//! 2. Merge multiple envs together (e.g., package + dependencies)
//! 3. Compress to collapse same-name evars
//! 4. Solve to expand all tokens
//! 5. Commit to apply to process
//!
//! # Example
//!
//! ```ignore
//! use pkg::{Env, Evar, Action};
//!
//! let mut env = Env::new("default");
//! env.add(Evar::set("ROOT", "/opt/maya"));
//! env.add(Evar::append("PATH", "{ROOT}/bin"));
//!
//! // Solve expands {ROOT} to /opt/maya
//! let solved = env.solve(10, true)?;
//! assert_eq!(solved.get("PATH").unwrap().value(), "/opt/maya/bin");
//!
//! // Apply to current process
//! solved.commit();
//! ```
//!
//! # Python API
//!
//! ```python
//! from pkg import Env, Evar
//!
//! env = Env("default")
//! env.add(Evar("PATH", "/bin", action="append"))
//! env.add(Evar("ROOT", "/opt", action="set"))
//!
//! merged = env + other_env  # __add__ supported
//! solved = env.solve()
//! env.commit()
//!
//! env.to_dict()
//! env.to_json()
//! ```

use crate::error::EnvError;
use crate::evar::Evar;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Default maximum depth for token expansion.
/// Prevents infinite recursion in circular references.
pub const DEFAULT_MAX_DEPTH: usize = 10;

/// Named collection of environment variables.
///
/// An Env groups related [`Evar`]s together under a name. Packages can have
/// multiple named environments (e.g., "default", "dev", "debug") and
/// applications reference them by name.
///
/// # Ordering
///
/// Evars maintain insertion order. When merging envs, the order is:
/// first env's evars, then second env's evars.
///
/// # Serialization
///
/// ```json
/// {
///   "name": "default",
///   "evars": [
///     {"name": "PATH", "value": "/bin", "action": "append"},
///     {"name": "ROOT", "value": "/opt", "action": "set"}
///   ]
/// }
/// ```
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Env {
    /// Environment name (e.g., "default", "dev")
    #[pyo3(get, set)]
    pub name: String,

    /// List of environment variables (maintains insertion order)
    #[pyo3(get)]
    pub evars: Vec<Evar>,
}

#[pymethods]
impl Env {
    /// Create a new empty environment.
    ///
    /// # Arguments
    /// * `name` - Environment name (e.g., "default")
    #[new]
    pub fn new(name: String) -> Self {
        Self {
            name,
            evars: Vec::new(),
        }
    }

    /// Add an Evar to this environment.
    ///
    /// # Arguments
    /// * `evar` - Environment variable to add
    pub fn add(&mut self, evar: Evar) {
        self.evars.push(evar);
    }

    /// Get an Evar by name.
    ///
    /// Returns the first Evar with matching name (case-insensitive).
    /// Returns None if not found.
    pub fn get(&self, name: &str) -> Option<Evar> {
        let name_lower = name.to_lowercase();
        self.evars
            .iter()
            .find(|e| e.name.to_lowercase() == name_lower)
            .cloned()
    }

    /// Get all Evars with a given name.
    ///
    /// A compressed env will have at most one evar per name,
    /// but before compression there may be multiple.
    pub fn get_all(&self, name: &str) -> Vec<Evar> {
        let name_lower = name.to_lowercase();
        self.evars
            .iter()
            .filter(|e| e.name.to_lowercase() == name_lower)
            .cloned()
            .collect()
    }

    /// Remove all Evars with a given name.
    pub fn remove(&mut self, name: &str) {
        let name_lower = name.to_lowercase();
        self.evars
            .retain(|e| e.name.to_lowercase() != name_lower);
    }

    /// Get all unique variable names in this environment.
    pub fn names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.evars
            .iter()
            .filter_map(|e| {
                let lower = e.name.to_lowercase();
                if seen.insert(lower) {
                    Some(e.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Number of evars in this environment.
    fn __len__(&self) -> usize {
        self.evars.len()
    }

    /// Check if environment is empty.
    pub fn is_empty(&self) -> bool {
        self.evars.is_empty()
    }

    /// Merge with another environment.
    ///
    /// Creates a new Env containing all evars from both environments.
    /// Self's evars come first, then other's evars.
    /// Does not compress - call compress() after if needed.
    ///
    /// # Python: supports + operator
    /// ```python
    /// merged = env1 + env2
    /// ```
    pub fn merge(&self, other: &Env) -> Env {
        let mut result = self.clone();
        result.evars.extend(other.evars.clone());
        result
    }

    /// Python __add__ operator
    fn __add__(&self, other: &Env) -> Env {
        self.merge(other)
    }

    /// Compress same-name evars into single evars.
    ///
    /// Iterates through evars in order, merging evars with the same name
    /// using their action semantics. The result has at most one evar per name.
    ///
    /// This is typically called after merging multiple environments.
    ///
    /// # Example
    /// ```text
    /// // Before: PATH=/a (append), PATH=/b (append)
    /// // After:  PATH=/a:/b (set)
    /// ```
    pub fn compress(&self) -> Env {
        let mut result = Env::new(self.name.clone());
        let mut seen: HashMap<String, usize> = HashMap::new(); // name -> index in result

        for evar in &self.evars {
            let name_lower = evar.name.to_lowercase();

            if let Some(&idx) = seen.get(&name_lower) {
                // Merge with existing evar
                let existing = &result.evars[idx];
                let merged = existing.merge(evar);
                result.evars[idx] = merged;
            } else {
                // First occurrence of this name
                seen.insert(name_lower, result.evars.len());
                result.evars.push(evar.clone());
            }
        }

        result
    }

    /// Solve all token references in evars.
    ///
    /// Expands `{VAR}` tokens recursively. Each token is replaced with
    /// the value of the corresponding evar. If not found, optionally
    /// falls back to OS environment.
    ///
    /// # Arguments
    /// * `max_depth` - Maximum recursion depth (default: 10)
    /// * `use_os_fallback` - If true, fallback to std::env for unknown vars
    ///
    /// # Returns
    /// New Env with all tokens expanded.
    ///
    /// # Errors
    /// - Circular reference detected
    /// - Maximum depth exceeded
    #[pyo3(signature = (max_depth = None, use_os_fallback = None))]
    pub fn solve(
        &self,
        max_depth: Option<usize>,
        use_os_fallback: Option<bool>,
    ) -> PyResult<Env> {
        self.solve_impl(
            max_depth.unwrap_or(DEFAULT_MAX_DEPTH),
            use_os_fallback.unwrap_or(true),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Apply all evars to current process environment.
    ///
    /// Calls `std::env::set_var` for each evar, respecting action semantics.
    /// Should typically be called on a solved, compressed environment.
    pub fn commit(&self) {
        for evar in &self.evars {
            evar.commit();
        }
    }

    /// Convert to HashMap for current OS.
    ///
    /// Returns a dict mapping variable names to their values.
    /// If there are multiple evars with the same name, the last one wins.
    pub fn to_map(&self) -> HashMap<String, String> {
        self.evars
            .iter()
            .map(|e| (e.name.clone(), e.value.clone()))
            .collect()
    }

    /// Convert to dictionary.
    ///
    /// Returns dict with keys: name, evars
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::types::{PyDict, PyList};
        let dict = PyDict::new(py);
        dict.set_item("name", &self.name)?;

        let evars_list = PyList::empty(py);
        for evar in &self.evars {
            evars_list.append(evar.to_dict(py)?)?;
        }
        dict.set_item("evars", evars_list)?;

        Ok(dict.into())
    }

    /// Create from dictionary.
    ///
    /// # Arguments
    /// * `dict` - Dict with keys: name, evars
    #[staticmethod]
    pub fn from_dict(dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<Self> {
        let name: String = dict
            .get_item("name")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'name'"))?
            .extract()?;

        let mut env = Env::new(name);

        if let Some(evars_obj) = dict.get_item("evars")? {
            let evars_list: Vec<Bound<'_, pyo3::types::PyDict>> = evars_obj.extract()?;
            for evar_dict in evars_list {
                env.add(Evar::from_dict(&evar_dict)?);
            }
        }

        Ok(env)
    }

    /// Export as Windows CMD script.
    ///
    /// Generates `SET VAR=value` lines for cmd.exe.
    /// Use with: `env.to_cmd() > setup.cmd`
    pub fn to_cmd(&self) -> String {
        self.evars
            .iter()
            .map(|e| format!("SET {}={}", e.name, e.value))
            .collect::<Vec<_>>()
            .join("\r\n")
    }

    /// Export as PowerShell script.
    ///
    /// Generates `$env:VAR = "value"` lines.
    /// Use with: `env.to_ps1() > setup.ps1`
    pub fn to_ps1(&self) -> String {
        self.evars
            .iter()
            .map(|e| {
                // Escape double quotes in value
                let escaped = e.value.replace('"', "`\"");
                format!("$env:{} = \"{}\"", e.name, escaped)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export as Bash/sh script.
    ///
    /// Generates `export VAR="value"` lines.
    /// Use with: `env.to_sh() > setup.sh`
    pub fn to_sh(&self) -> String {
        self.evars
            .iter()
            .map(|e| {
                // Escape double quotes and backslashes
                let escaped = e.value.replace('\\', "\\\\").replace('"', "\\\"");
                format!("export {}=\"{}\"", e.name, escaped)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export as Python script.
    ///
    /// Generates `os.environ['VAR'] = 'value'` lines.
    /// Includes `import os` at the top.
    /// Use with: `env.to_py() > setup.py`
    pub fn to_py(&self) -> String {
        let mut lines = vec!["import os".to_string(), "".to_string()];
        for e in &self.evars {
            // Escape single quotes
            let escaped = e.value.replace('\\', "\\\\").replace('\'', "\\'");
            lines.push(format!("os.environ['{}'] = '{}'", e.name, escaped));
        }
        lines.join("\n")
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
        format!("Env({:?}, {} evars)", self.name, self.evars.len())
    }

    /// Iteration support for Python
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<EnvIter>> {
        let iter = EnvIter {
            evars: slf.evars.clone(),
            index: 0,
        };
        Py::new(slf.py(), iter)
    }
}

// Rust-only methods (not exposed to Python)
impl Env {
    /// Returns evars sorted by name (for display).
    pub fn evars_sorted(&self) -> Vec<&Evar> {
        let mut sorted: Vec<_> = self.evars.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        sorted
    }
}

/// Iterator for Env (Python support)
#[pyclass]
struct EnvIter {
    evars: Vec<Evar>,
    index: usize,
}

#[pymethods]
impl EnvIter {
    fn __next__(&mut self) -> Option<Evar> {
        if self.index < self.evars.len() {
            let evar = self.evars[self.index].clone();
            self.index += 1;
            Some(evar)
        } else {
            None
        }
    }
}

// Pure Rust API
impl Env {
    /// Create env from iterator of Evars.
    pub fn from_evars(name: impl Into<String>, evars: impl IntoIterator<Item = Evar>) -> Self {
        Self {
            name: name.into(),
            evars: evars.into_iter().collect(),
        }
    }

    /// Create Env from current OS environment.
    ///
    /// All variables are created with Action::Set.
    pub fn from_os_env(name: impl Into<String>) -> Self {
        let evars: Vec<Evar> = std::env::vars()
            .map(|(k, v)| Evar::set(k, v))
            .collect();
        Self {
            name: name.into(),
            evars,
        }
    }

    /// Internal solve implementation.
    ///
    /// Two-phase solve:
    /// 1. Compress to get single evar per name
    /// 2. Expand tokens using shared token module (with recursion + cycle detection)
    pub fn solve_impl(&self, max_depth: usize, use_os_fallback: bool) -> Result<Env, EnvError> {
        use crate::token;

        // First compress to have single value per variable
        let compressed = self.compress();

        // Build lookup map from compressed evars
        let lookup_map: HashMap<String, String> = compressed
            .evars
            .iter()
            .map(|e| (e.name.to_lowercase(), e.value.clone()))
            .collect();

        // Solve each evar using token module
        let mut solved_evars = Vec::new();
        for evar in &compressed.evars {
            let solved_value = if use_os_fallback {
                token::expand_with_fallback(&evar.value, &lookup_map, max_depth)
            } else {
                token::expand_recursive(&evar.value, &lookup_map, max_depth)
            }
            .map_err(|e| match e {
                token::TokenError::CircularReference { name } => {
                    EnvError::CircularReference { name }
                }
                token::TokenError::DepthExceeded { name, max_depth } => {
                    EnvError::DepthExceeded { name, max_depth }
                }
            })?;

            solved_evars.push(Evar::new(
                evar.name.clone(),
                solved_value,
                evar.get_action(),
            ));
        }

        Ok(Env {
            name: self.name.clone(),
            evars: solved_evars,
        })
    }

    /// Merge multiple environments into one.
    ///
    /// Convenience method to merge a list of environments.
    /// The first env's name is used for the result.
    pub fn merge_all(envs: &[&Env]) -> Option<Env> {
        let mut iter = envs.iter();
        let first = iter.next()?;
        let mut result = (*first).clone();

        for env in iter {
            result = result.merge(env);
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_new() {
        let env = Env::new("test".to_string());
        assert_eq!(env.name, "test");
        assert!(env.is_empty());
    }

    #[test]
    fn env_add_get() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/bin"));
        env.add(Evar::set("ROOT", "/opt"));

        assert_eq!(env.evars.len(), 2);
        assert_eq!(env.get("PATH").unwrap().value(), "/bin");
        assert_eq!(env.get("path").unwrap().value(), "/bin"); // case-insensitive
        assert!(env.get("UNKNOWN").is_none());
    }

    #[test]
    fn env_merge() {
        let mut env1 = Env::new("a".to_string());
        env1.add(Evar::set("A", "1"));

        let mut env2 = Env::new("b".to_string());
        env2.add(Evar::set("B", "2"));

        let merged = env1.merge(&env2);
        assert_eq!(merged.evars.len(), 2);
        assert!(merged.get("A").is_some());
        assert!(merged.get("B").is_some());
    }

    #[test]
    fn env_compress() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/a"));
        env.add(Evar::append("PATH", "/b"));
        env.add(Evar::append("PATH", "/c"));

        let compressed = env.compress();
        assert_eq!(compressed.evars.len(), 1);

        let path = compressed.get("PATH").unwrap();
        assert!(path.value().contains("/a"));
        assert!(path.value().contains("/b"));
        assert!(path.value().contains("/c"));
    }

    #[test]
    fn env_solve_simple() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("ROOT", "/opt/maya"));
        env.add(Evar::set("PATH", "{ROOT}/bin"));

        let solved = env.solve_impl(10, false).unwrap();
        assert_eq!(solved.get("PATH").unwrap().value(), "/opt/maya/bin");
    }

    #[test]
    fn env_solve_chain() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("A", "base"));
        env.add(Evar::set("B", "{A}/level1"));
        env.add(Evar::set("C", "{B}/level2"));

        let solved = env.solve_impl(10, false).unwrap();
        assert_eq!(solved.get("C").unwrap().value(), "base/level1/level2");
    }

    #[test]
    fn env_solve_cycle_detection() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("A", "{B}"));
        env.add(Evar::set("B", "{A}"));

        let result = env.solve_impl(10, false);
        assert!(result.is_err());
        if let Err(EnvError::CircularReference { name }) = result {
            assert!(name == "A" || name == "B");
        } else {
            panic!("Expected CircularReference error");
        }
    }

    #[test]
    fn env_solve_depth_exceeded() {
        // Create a deep chain
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("V0", "base"));
        for i in 1..=15 {
            env.add(Evar::set(format!("V{}", i), format!("{{V{}}}", i - 1)));
        }

        // With max_depth=5, should fail
        let result = env.solve_impl(5, false);
        assert!(matches!(result, Err(EnvError::DepthExceeded { .. })));
    }

    #[test]
    fn env_serialization() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/bin"));

        let json = serde_json::to_string(&env).unwrap();
        let env2: Env = serde_json::from_str(&json).unwrap();

        assert_eq!(env, env2);
    }

    #[test]
    fn env_to_cmd() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "C:\\bin"));
        env.add(Evar::set("ROOT", "C:\\opt"));

        let cmd = env.to_cmd();
        assert!(cmd.contains("SET PATH=C:\\bin"));
        assert!(cmd.contains("SET ROOT=C:\\opt"));
        assert!(cmd.contains("\r\n")); // CRLF for Windows
    }

    #[test]
    fn env_to_ps1() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/bin"));
        env.add(Evar::set("MSG", "hello \"world\""));

        let ps1 = env.to_ps1();
        assert!(ps1.contains("$env:PATH = \"/bin\""));
        assert!(ps1.contains("`\""));  // escaped quote
    }

    #[test]
    fn env_to_sh() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/bin"));
        env.add(Evar::set("MSG", "hello \"world\""));

        let sh = env.to_sh();
        assert!(sh.contains("export PATH=\"/bin\""));
        assert!(sh.contains("\\\"")); // escaped quote
    }

    #[test]
    fn env_to_py() {
        let mut env = Env::new("test".to_string());
        env.add(Evar::set("PATH", "/bin"));
        env.add(Evar::set("MSG", "it's fine"));

        let py = env.to_py();
        assert!(py.starts_with("import os"));
        assert!(py.contains("os.environ['PATH'] = '/bin'"));
        assert!(py.contains("\\'"));  // escaped single quote
    }
}
