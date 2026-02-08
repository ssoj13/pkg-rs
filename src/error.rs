//! Error types for the pkg library.
//!
//! This module defines all error types used throughout the crate.
//! Uses `thiserror` for ergonomic error handling and automatic `Display` impl.
//!
//! # Error Hierarchy
//!
//! - [`PkgError`] - Top-level error enum, wraps all other errors
//! - [`EvarError`] - Errors from Evar operations (solve, parse)
//! - [`EnvError`] - Errors from Env operations (solve cycles, depth)
//! - [`PackageError`] - Errors from Package operations
//! - [`SolverError`] - Errors from dependency resolution
//! - [`StorageError`] - Errors from package scanning/loading
//! - [`LoaderError`] - Errors from package.py execution
//!
//! # Usage
//!
//! All public functions return `Result<T, PkgError>` for consistency.
//! Internal modules may use more specific error types.

use std::path::PathBuf;
use thiserror::Error;

/// Top-level error type for the pkg library.
///
/// This enum wraps all specific error types and is the primary error type
/// returned by public API functions.
#[derive(Error, Debug)]
pub enum PkgError {
    /// Error from environment variable operations
    #[error("evar error: {0}")]
    Evar(#[from] EvarError),

    /// Error from environment operations
    #[error("env error: {0}")]
    Env(#[from] EnvError),

    /// Error from package operations
    #[error("package error: {0}")]
    Package(#[from] PackageError),

    /// Error from dependency resolution
    #[error("solver error: {0}")]
    Solver(#[from] SolverError),

    /// Error from storage/scanning operations
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// Error from package.py loading
    #[error("loader error: {0}")]
    Loader(#[from] LoaderError),

    /// Error from build pipeline
    #[error("build error: {0}")]
    Build(#[from] BuildError),

    /// Error from pip import
    #[error("pip error: {0}")]
    Pip(#[from] PipError),

    /// IO error (file operations)
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Errors from [`Evar`](crate::Evar) operations.
///
/// These occur during token expansion or parsing.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EvarError {
    /// Token expansion exceeded maximum recursion depth.
    /// Default max depth is 10 to prevent infinite loops.
    #[error("token expansion depth exceeded for '{name}': max {max_depth}")]
    DepthExceeded {
        /// Variable name that caused the error
        name: String,
        /// Maximum allowed depth
        max_depth: usize,
    },

    /// Circular reference detected during token expansion.
    /// E.g., A={B}, B={A} would cause this.
    #[error("circular reference detected: {name}")]
    CircularReference {
        /// Variable name where cycle was detected
        name: String,
    },

    /// Invalid action string (must be "set", "append", or "insert")
    #[error("invalid action '{action}', expected: set, append, insert")]
    InvalidAction {
        /// The invalid action string
        action: String,
    },
}

/// Errors from [`Env`](crate::Env) operations.
///
/// These occur during environment solving or merging.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EnvError {
    /// Token expansion exceeded maximum depth (propagated from EvarError)
    #[error("solve depth exceeded for '{name}': max {max_depth}")]
    DepthExceeded {
        /// Variable name
        name: String,
        /// Maximum depth
        max_depth: usize,
    },

    /// Circular reference in token expansion (propagated from EvarError)
    #[error("circular reference in env solve: {name}")]
    CircularReference {
        /// Variable name
        name: String,
    },

    /// Variable not found during token expansion (when fallback is disabled)
    #[error("variable not found: {name}")]
    VariableNotFound {
        /// Missing variable name
        name: String,
    },
}

/// Errors from [`Package`](crate::Package) operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    /// Referenced environment not found in package.envs
    #[error("env not found: {name}")]
    EnvNotFound {
        /// Missing environment name
        name: String,
    },

    /// Referenced application not found in package.apps
    #[error("app not found: {name}")]
    AppNotFound {
        /// Missing application name
        name: String,
    },

    /// Invalid package name format (should be "name-version")
    #[error("invalid package name '{name}': {reason}")]
    InvalidName {
        /// The invalid name
        name: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Invalid version string (must be valid SemVer)
    #[error("invalid version '{version}': {reason}")]
    InvalidVersion {
        /// The invalid version string
        version: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Dependencies not yet solved (call solve_deps first)
    #[error("dependencies not solved for package: {name}")]
    DepsNotSolved {
        /// Package name
        name: String,
    },
}

/// Errors from the dependency [`Solver`](crate::Solver).
///
/// Wraps PubGrub errors and adds pkg-specific errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SolverError {
    /// Dependency specification parse error (e.g., "maya@invalid")
    #[error("invalid dependency spec '{spec}': {reason}")]
    InvalidDepSpec {
        /// The invalid spec string
        spec: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Invalid version string
    #[error("invalid version for {package}: '{version}' - {reason}")]
    InvalidVersion {
        /// Package name
        package: String,
        /// Invalid version string
        version: String,
        /// Reason
        reason: String,
    },

    /// Invalid dependency in package
    #[error("invalid dependency in {package}: '{dependency}' - {reason}")]
    InvalidDependency {
        /// Package with bad dep
        package: String,
        /// Dependency string
        dependency: String,
        /// Reason
        reason: String,
    },

    /// No version satisfies the requirements
    #[error("no solution found: {reason}")]
    NoSolution {
        /// Explanation of why no solution exists
        reason: String,
    },

    /// No matching version for constraint
    #[error("no matching version for {package}: {constraint}")]
    NoMatchingVersion {
        /// Package base name
        package: String,
        /// Constraint that couldn't be satisfied
        constraint: String,
    },

    /// Version conflict between packages
    #[error("conflict: {message}")]
    Conflict {
        /// Conflict description
        message: String,
    },

    /// Dependency chain too deep
    #[error("dependency depth exceeded: max {max}, got {actual}")]
    DepthExceeded {
        /// Maximum allowed depth
        max: usize,
        /// Actual depth encountered
        actual: usize,
    },

    /// Circular dependency detected
    #[error("circular dependency: {package}")]
    CircularDependency {
        /// Package where cycle was detected
        package: String,
    },

    /// Package not found in registry
    #[error("package not found: {package}")]
    PackageNotFound {
        /// Missing package name
        package: String,
    },

    /// Version not found for package
    #[error("version not found: {name}@{version}")]
    VersionNotFound {
        /// Package name
        name: String,
        /// Missing version
        version: String,
    },

    /// Unsupported solver backend
    #[error("unsupported solver backend: {backend}")]
    UnsupportedBackend {
        /// Backend name
        backend: String,
    },

    /// Backend execution error
    #[error("{backend} solver error: {message}")]
    BackendError {
        /// Backend name
        backend: String,
        /// Error message
        message: String,
    },
}

/// Errors from the build pipeline.
#[derive(Error, Debug)]
pub enum BuildError {
    /// Build configuration error
    #[error("build config error: {0}")]
    Config(String),

    /// Build dependency resolution error
    #[error("build resolve error: {0}")]
    Resolve(String),

    /// Build command failed
    #[error("build command failed: {command} (exit {code:?})")]
    CommandFailed {
        /// Command string
        command: String,
        /// Exit code (if available)
        code: Option<i32>,
    },

    /// IO error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors from pip import.
#[derive(Error, Debug)]
pub enum PipError {
    /// Pip configuration error
    #[error("pip config error: {0}")]
    Config(String),

    /// Pip command failed
    #[error("pip command failed: {command} (exit {code:?})")]
    CommandFailed {
        /// Command string
        command: String,
        /// Exit code (if available)
        code: Option<i32>,
    },

    /// IO error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors from [`Storage`](crate::Storage) operations.
///
/// These occur during package discovery and loading.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Configuration error
    #[error("config error for {}: {reason}", path.display())]
    Config {
        /// Config path
        path: PathBuf,
        /// Error reason
        reason: String,
    },
    /// Invalid path (doesn't exist or not accessible)
    #[error("invalid path: {}", path.display())]
    InvalidPath {
        /// The invalid path
        path: PathBuf,
    },

    /// Failed to scan directory
    #[error("scan failed for {}: {reason}", path.display())]
    ScanFailed {
        /// Directory path
        path: PathBuf,
        /// Failure reason
        reason: String,
    },

    /// Scan error (alias for ScanFailed)
    #[error("scan error for {}: {reason}", path.display())]
    ScanError {
        /// Directory path
        path: PathBuf,
        /// Error reason
        reason: String,
    },

    /// Invalid package definition
    #[error("invalid package at {}: {reason}", path.display())]
    InvalidPackage {
        /// Path to package.py or directory
        path: PathBuf,
        /// Reason it's invalid
        reason: String,
    },

    /// Failed to load package.py
    #[error("load failed for {}: {reason}", path.display())]
    LoadFailed {
        /// Path to package.py
        path: PathBuf,
        /// Failure reason
        reason: String,
    },

    /// IO error during scanning
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors from [`Loader`](crate::Loader) (package.py execution).
///
/// These occur during Python code execution and result parsing.
#[derive(Error, Debug)]
pub enum LoaderError {
    /// File not found
    #[error("file not found: {}", path.display())]
    FileNotFound {
        /// Path to missing file
        path: PathBuf,
    },

    /// Failed to read file
    #[error("read error for {}: {reason}", path.display())]
    ReadError {
        /// Path to file
        path: PathBuf,
        /// Error reason
        reason: String,
    },

    /// Python execution error
    #[error("execution error in {}: {reason}", path.display())]
    ExecutionError {
        /// Path to package.py
        path: PathBuf,
        /// Error reason
        reason: String,
    },

    /// Python execution error (legacy)
    #[error("python error in {}: {message}", path.display())]
    PythonError {
        /// Path to package.py
        path: PathBuf,
        /// Error message from Python
        message: String,
    },

    /// Required function not found
    #[error("function '{}' not found in {}", function, path.display())]
    MissingFunction {
        /// Path to package.py
        path: PathBuf,
        /// Missing function name
        function: String,
    },

    /// get_package function not found (legacy)
    #[error("get_package() not found in {}", path.display())]
    MissingGetPackage {
        /// Path to package.py
        path: PathBuf,
    },

    /// Invalid return value
    #[error("invalid return from {}: {reason}", path.display())]
    InvalidReturn {
        /// Path to package.py
        path: PathBuf,
        /// Reason
        reason: String,
    },

    /// get_package returned invalid type (legacy)
    #[error("get_package() returned invalid type in {}: expected Package or list[Package]", path.display())]
    InvalidReturnType {
        /// Path to package.py
        path: PathBuf,
    },

    /// Missing required field in returned dict
    #[error("missing field '{field}' in {}", path.display())]
    MissingField {
        /// Path to package.py
        path: PathBuf,
        /// Missing field name
        field: String,
    },

    /// Invalid field type in returned dict
    #[error("invalid type for field '{field}' in {}: {reason}", path.display())]
    InvalidFieldType {
        /// Path to package.py
        path: PathBuf,
        /// Field name
        field: String,
        /// Reason
        reason: String,
    },

    /// IO error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias using PkgError
pub type Result<T> = std::result::Result<T, PkgError>;

// ============================================================================
// PyO3 error conversions
// ============================================================================
// These impls allow using ? operator directly in #[pymethods] functions:
// fn foo() -> PyResult<T> { do_something()?; Ok(result) }

use pyo3::exceptions::PyValueError;
use pyo3::PyErr;

impl From<PkgError> for PyErr {
    fn from(err: PkgError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<EvarError> for PyErr {
    fn from(err: EvarError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<EnvError> for PyErr {
    fn from(err: EnvError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<PackageError> for PyErr {
    fn from(err: PackageError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<SolverError> for PyErr {
    fn from(err: SolverError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<StorageError> for PyErr {
    fn from(err: StorageError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<LoaderError> for PyErr {
    fn from(err: LoaderError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

// ============================================================================
// Helper trait for external error types (orphan rule workaround)
// ============================================================================

/// Extension trait to convert external errors to PyErr.
/// Usage: `serde_json::to_string(x).py_err()?`
pub trait IntoPyErr<T> {
    fn py_err(self) -> std::result::Result<T, PyErr>;
}

impl<T> IntoPyErr<T> for std::result::Result<T, serde_json::Error> {
    fn py_err(self) -> std::result::Result<T, PyErr> {
        self.map_err(|e| PyValueError::new_err(format!("JSON error: {}", e)))
    }
}

impl<T> IntoPyErr<T> for std::result::Result<T, semver::Error> {
    fn py_err(self) -> std::result::Result<T, PyErr> {
        self.map_err(|e| PyValueError::new_err(format!("Invalid semver: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = EvarError::InvalidAction {
            action: "invalid".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid action 'invalid', expected: set, append, insert"
        );
    }

    #[test]
    fn error_conversion() {
        let evar_err = EvarError::CircularReference {
            name: "PATH".to_string(),
        };
        let pkg_err: PkgError = evar_err.into();
        assert!(matches!(pkg_err, PkgError::Evar(_)));
    }
}
