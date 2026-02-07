//! Build command representation for custom build systems.
//!
//! Rez-style build commands can be:
//! - `false` to disable the build step
//! - a string command
//! - a list of command arguments

use pyo3::conversion::IntoPyObject;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3::{BoundObject, PyErr};
use serde::{Deserialize, Serialize};

/// Custom build command definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildCommand {
    /// Disable build step when explicitly set to `false`.
    Disabled(bool),
    /// Shell command string.
    String(String),
    /// List of command arguments (argv-style).
    List(Vec<String>),
}

impl BuildCommand {
    /// Returns true if the build command explicitly disables the build.
    pub fn is_disabled(&self) -> bool {
        matches!(self, BuildCommand::Disabled(false))
    }

    /// Returns true if the build command is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            BuildCommand::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the list form if available.
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            BuildCommand::List(list) => Some(list.as_slice()),
            _ => None,
        }
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for BuildCommand {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let ob = ob.as_any();
        if let Ok(value) = ob.extract::<bool>() {
            if !value {
                return Ok(BuildCommand::Disabled(false));
            }
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "build_command only supports False as a boolean",
            ));
        }
        if let Ok(value) = ob.extract::<String>() {
            return Ok(BuildCommand::String(value));
        }
        if let Ok(value) = ob.extract::<Vec<String>>() {
            return Ok(BuildCommand::List(value));
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "build_command must be bool, string, or list of strings",
        ))
    }
}

impl<'py> IntoPyObject<'py> for BuildCommand {
    type Target = PyAny;
    type Output = pyo3::Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            BuildCommand::Disabled(value) => {
                let any = value.into_pyobject(py)?.into_any();
                Ok(any.into_bound())
            }
            BuildCommand::String(value) => {
                let any = value.into_pyobject(py)?.into_any();
                Ok(any.into_bound())
            }
            BuildCommand::List(value) => {
                let any = value.into_pyobject(py)?.into_any();
                Ok(any.into_bound())
            }
        }
    }
}
