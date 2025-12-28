//! Environment variable representation and operations.
//!
//! This module provides the [`Evar`] struct - the fundamental building block
//! for environment management in pkg. An Evar represents a single
//! environment variable with a name, value, and action.
//!
//! # Design Philosophy
//!
//! Evar is intentionally OS-agnostic. The package.py file is responsible for
//! providing the correct values for the current platform using `sys.platform`.
//! This keeps the Rust side simple and moves platform logic to Python where
//! it's more natural to handle.
//!
//! # Actions
//!
//! - **Set**: Replace the variable value entirely
//! - **Append**: Add to the end of existing value (with path separator)
//! - **Insert**: Add to the beginning of existing value (with path separator)
//!
//! # Token Expansion
//!
//! Values can contain `{VAR_NAME}` tokens that get expanded during solve.
//! For example: `{ROOT}/bin` expands to `/opt/maya/bin` if ROOT=/opt/maya.
//!
//! # Example
//!
//! ```ignore
//! use pkg::{Evar, Action};
//!
//! let path = Evar::new("PATH", "/opt/maya/bin", Action::Append);
//! let root = Evar::new("MAYA_ROOT", "/opt/maya", Action::Set);
//!
//! // Merge two evars with same name
//! let path2 = Evar::new("PATH", "/opt/maya/scripts", Action::Append);
//! let merged = path.merge(&path2);
//! assert_eq!(merged.value(), "/opt/maya/bin:/opt/maya/scripts");
//! ```
//!
//! # Python API
//!
//! ```python
//! from pkg import Evar
//!
//! e = Evar("PATH", "/opt/bin", action="append")
//! e.name    # "PATH"
//! e.value   # "/opt/bin"
//! e.action  # "append"
//!
//! e.to_dict()  # {"name": "PATH", "value": "/opt/bin", "action": "append"}
//! e.to_json()  # '{"name":"PATH","value":"/opt/bin","action":"append"}'
//! ```

use crate::error::EvarError;
use crate::token;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Get path separator for environment variable concatenation.
/// 
/// Checks `PKG_PATH_SEP` env var first, falls back to platform default.
/// Returns ";" on Windows, ":" on Unix (useful for MSYS2/Git Bash).
#[inline]
pub fn path_sep() -> String {
    std::env::var("PKG_PATH_SEP").unwrap_or_else(|_| {
        if cfg!(windows) { ";".into() } else { ":".into() }
    })
}

/// Action to perform when merging environment variables.
///
/// Determines how a new value combines with an existing value
/// when two Evars with the same name are merged.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Replace the existing value entirely.
    /// New value becomes the only value.
    Set,

    /// Append new value to existing value.
    /// Uses OS path separator (`;` on Windows, `:` on Unix).
    /// Example: existing="A", new="B" -> "A:B"
    #[default]
    Append,

    /// Insert new value before existing value.
    /// Uses OS path separator.
    /// Example: existing="A", new="B" -> "B:A"
    Insert,
}

impl Action {
    /// Parse action from string.
    ///
    /// # Arguments
    /// * `s` - One of: "set", "append", "insert" (case-insensitive)
    ///
    /// # Errors
    /// Returns [`EvarError::InvalidAction`] if string is not recognized.
    pub fn from_str(s: &str) -> Result<Self, EvarError> {
        match s.to_lowercase().as_str() {
            "set" => Ok(Action::Set),
            "append" => Ok(Action::Append),
            "insert" => Ok(Action::Insert),
            _ => Err(EvarError::InvalidAction {
                action: s.to_string(),
            }),
        }
    }

    /// Convert action to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Set => "set",
            Action::Append => "append",
            Action::Insert => "insert",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Environment variable with name, value, and merge action.
///
/// This is the fundamental building block for environment management.
/// Evars can be merged together respecting their action semantics,
/// and their values can contain `{TOKEN}` placeholders that get
/// expanded during the solve phase.
///
/// # Fields
///
/// - `name`: Variable name (e.g., "PATH", "PYTHONPATH")
/// - `value`: Variable value, may contain `{TOKENS}` for expansion
/// - `action`: How this value merges with existing values
///
/// # Serialization
///
/// Evar serializes to JSON as:
/// ```json
/// {"name": "PATH", "value": "/bin", "action": "append"}
/// ```
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Evar {
    /// Variable name (e.g., "PATH", "MAYA_ROOT")
    #[pyo3(get, set)]
    pub name: String,

    /// Variable value, may contain {TOKEN} placeholders
    #[pyo3(get, set)]
    pub value: String,

    /// Action for merging with existing values
    #[serde(default)]
    action: Action,
}

#[pymethods]
impl Evar {
    /// Create a new environment variable.
    ///
    /// # Arguments
    /// * `name` - Variable name
    /// * `value` - Variable value (may contain {TOKENS})
    /// * `action` - Optional merge action: "set", "append", "insert" (default: "append")
    ///
    /// # Python Example
    /// ```python
    /// e = Evar("PATH", "/opt/bin")  # default append
    /// e = Evar("ROOT", "/opt", action="set")
    /// ```
    #[new]
    #[pyo3(signature = (name, value, action = None))]
    pub fn py_new(name: String, value: String, action: Option<&str>) -> PyResult<Self> {
        let action = match action {
            Some(s) => Action::from_str(s)?,
            None => Action::Append,
        };
        Ok(Self { name, value, action })
    }

    /// Get action as string ("set", "append", "insert")
    #[getter]
    pub fn action(&self) -> &str {
        self.action.as_str()
    }

    /// Set action from string
    #[setter]
    pub fn set_action(&mut self, action: &str) -> PyResult<()> {
        self.action = Action::from_str(action)?;
        Ok(())
    }

    /// Convert to dictionary.
    ///
    /// # Returns
    /// Dict with keys: name, value, action
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::types::PyDict;
        let dict = PyDict::new(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("action", self.action.as_str())?;
        Ok(dict.into())
    }

    /// Create from dictionary.
    ///
    /// # Arguments
    /// * `dict` - Dict with keys: name, value, action (optional)
    #[staticmethod]
    pub fn from_dict(dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<Self> {
        let name: String = dict
            .get_item("name")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'name'"))?
            .extract()?;
        let value: String = dict
            .get_item("value")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'value'"))?
            .extract()?;
        let action = match dict.get_item("action")? {
            Some(a) => Action::from_str(a.extract::<String>()?.as_str())?,
            None => Action::Append,
        };
        Ok(Self { name, value, action })
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
            "Evar({:?}, {:?}, action={:?})",
            self.name, self.value, self.action.as_str()
        )
    }

    /// Equality check for Python
    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    /// Hash for Python (allows use in sets/dicts)
    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

// Pure Rust API (not exposed to Python directly)
impl Evar {
    /// Create a new Evar (Rust API).
    ///
    /// # Arguments
    /// * `name` - Variable name
    /// * `value` - Variable value
    /// * `action` - Merge action
    pub fn new(name: impl Into<String>, value: impl Into<String>, action: Action) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            action,
        }
    }

    /// Create an Evar with Set action.
    pub fn set(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value, Action::Set)
    }

    /// Create an Evar with Append action.
    pub fn append(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value, Action::Append)
    }

    /// Create an Evar with Insert action.
    pub fn insert(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value, Action::Insert)
    }

    /// Get the action.
    pub fn get_action(&self) -> Action {
        self.action
    }

    /// Get value reference.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Merge another Evar into this one.
    ///
    /// Both Evars must have the same name. The `other` Evar's action
    /// determines how values are combined:
    ///
    /// - Set: other.value replaces self.value
    /// - Append: self.value + separator + other.value
    /// - Insert: other.value + separator + self.value
    ///
    /// # Panics
    /// Panics if names don't match. Use `try_merge` for fallible version.
    pub fn merge(&self, other: &Evar) -> Evar {
        assert_eq!(
            self.name.to_lowercase(),
            other.name.to_lowercase(),
            "Cannot merge Evars with different names: {} vs {}",
            self.name,
            other.name
        );

        let new_value = match other.action {
            Action::Set => other.value.clone(),
            Action::Append => {
                if self.value.is_empty() {
                    other.value.clone()
                } else if other.value.is_empty() {
                    self.value.clone()
                } else {
                    format!("{}{}{}", self.value, path_sep(), other.value)
                }
            }
            Action::Insert => {
                if self.value.is_empty() {
                    other.value.clone()
                } else if other.value.is_empty() {
                    self.value.clone()
                } else {
                    format!("{}{}{}", other.value, path_sep(), self.value)
                }
            }
        };

        Evar {
            name: self.name.clone(),
            value: new_value,
            // After merge, action becomes Set (value is now concrete)
            action: Action::Set,
        }
    }

    /// Find all `{TOKEN}` patterns in the value.
    ///
    /// Returns a set of token names (without braces).
    /// Used during solve to determine which variables need expansion.
    ///
    /// # Example
    /// ```ignore
    /// let e = Evar::append("PATH", "{ROOT}/bin:{SCRIPTS}");
    /// assert_eq!(e.tokens(), vec!["ROOT", "SCRIPTS"].into_iter().collect());
    /// ```
    pub fn tokens(&self) -> HashSet<String> {
        token::extract(&self.value)
    }

    /// Check if value contains any tokens.
    pub fn has_tokens(&self) -> bool {
        self.value.contains('{') && self.value.contains('}')
    }

    /// Expand tokens in value using provided lookup function.
    ///
    /// This is the core solve operation. Tokens like `{VAR}` are replaced
    /// with their values from the lookup function.
    ///
    /// # Arguments
    /// * `lookup` - Function that resolves variable name to value
    /// * `visiting` - Set of currently visiting vars (for cycle detection)
    /// * `depth` - Current recursion depth
    /// * `max_depth` - Maximum allowed depth
    ///
    /// # Errors
    /// - [`EvarError::DepthExceeded`] if recursion too deep
    /// - [`EvarError::CircularReference`] if cycle detected
    pub fn solve_with<F>(
        &self,
        lookup: F,
        visiting: &mut HashSet<String>,
        depth: usize,
        max_depth: usize,
    ) -> Result<Evar, EvarError>
    where
        F: Fn(&str) -> Option<String>,
    {
        if depth > max_depth {
            return Err(EvarError::DepthExceeded {
                name: self.name.clone(),
                max_depth,
            });
        }

        // Check for circular reference
        let name_lower = self.name.to_lowercase();
        if visiting.contains(&name_lower) {
            return Err(EvarError::CircularReference {
                name: self.name.clone(),
            });
        }
        visiting.insert(name_lower.clone());

        // Expand all tokens in value
        let solved_value = token::expand_tokens(&self.value, &lookup);

        visiting.remove(&name_lower);

        Ok(Evar {
            name: self.name.clone(),
            value: solved_value,
            action: self.action,
        })
    }

    /// Apply this variable to the current process environment.
    ///
    /// Uses `std::env::set_var` with action semantics:
    /// - Set: overwrites
    /// - Append: adds to end
    /// - Insert: adds to beginning
    pub fn commit(&self) {
        match self.action {
            Action::Set => {
                std::env::set_var(&self.name, &self.value);
            }
            Action::Append => {
                let current = std::env::var(&self.name).unwrap_or_default();
                let new_value = if current.is_empty() {
                    self.value.clone()
                } else {
                    format!("{}{}{}", current, path_sep(), self.value)
                };
                std::env::set_var(&self.name, new_value);
            }
            Action::Insert => {
                let current = std::env::var(&self.name).unwrap_or_default();
                let new_value = if current.is_empty() {
                    self.value.clone()
                } else {
                    format!("{}{}{}", self.value, path_sep(), current)
                };
                std::env::set_var(&self.name, new_value);
            }
        }
    }
}

impl fmt::Display for Evar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={} ({})", self.name, self.value, self.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_parse() {
        assert_eq!(Action::from_str("set").unwrap(), Action::Set);
        assert_eq!(Action::from_str("APPEND").unwrap(), Action::Append);
        assert_eq!(Action::from_str("Insert").unwrap(), Action::Insert);
        assert!(Action::from_str("invalid").is_err());
    }

    #[test]
    fn evar_new() {
        let e = Evar::new("PATH", "/bin", Action::Append);
        assert_eq!(e.name, "PATH");
        assert_eq!(e.value, "/bin");
        assert_eq!(e.action, Action::Append);
    }

    #[test]
    fn evar_merge_set() {
        let a = Evar::new("PATH", "/old", Action::Set);
        let b = Evar::new("PATH", "/new", Action::Set);
        let c = a.merge(&b);
        assert_eq!(c.value, "/new");
    }

    #[test]
    fn evar_merge_append() {
        let a = Evar::new("PATH", "/a", Action::Set);
        let b = Evar::new("PATH", "/b", Action::Append);
        let c = a.merge(&b);
        // On Unix: "/a:/b", on Windows: "/a;/b"
        assert!(c.value.contains("/a"));
        assert!(c.value.contains("/b"));
    }

    #[test]
    fn evar_merge_insert() {
        let a = Evar::new("PATH", "/a", Action::Set);
        let b = Evar::new("PATH", "/b", Action::Insert);
        let c = a.merge(&b);
        // Should be "/b:/a" or "/b;/a"
        assert!(c.value.starts_with("/b"));
    }

    #[test]
    fn extract_tokens_basic() {
        let tokens = token::extract("{ROOT}/bin/{LIB}");
        assert!(tokens.contains("ROOT"));
        assert!(tokens.contains("LIB"));
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn extract_tokens_empty_braces() {
        let tokens = token::extract("{}/bin");
        assert!(tokens.is_empty());
    }

    #[test]
    fn expand_tokens_basic() {
        let value = "{ROOT}/bin";
        let result = token::expand_tokens(value, |name| {
            if name == "ROOT" {
                Some("/opt/maya".to_string())
            } else {
                None
            }
        });
        assert_eq!(result, "/opt/maya/bin");
    }

    #[test]
    fn expand_tokens_missing() {
        let value = "{UNKNOWN}/bin";
        let result = token::expand_tokens(value, |_| None);
        assert_eq!(result, "{UNKNOWN}/bin");
    }

    #[test]
    fn evar_serialization() {
        let e = Evar::new("PATH", "/bin", Action::Append);
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"name\":\"PATH\""));
        assert!(json.contains("\"action\":\"append\""));

        let e2: Evar = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }
}
