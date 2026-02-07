//! pkg: Software package management system.
//!
//! A Rust library for managing software packages with Python-based definitions,
//! environment variables, and dependency resolution.
//!
//! # Overview
//!
//! pkg provides:
//!
//! - **Package definitions** via Python (`package.py` files)
//! - **Environment management** with variable expansion and merging
//! - **Dependency resolution** using PubGrub algorithm
//! - **Application launching** with proper environment setup
//! - **Both Rust and Python APIs** (via PyO3)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         CLI / GUI                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Storage   │   Loader   │   Solver   │    Launcher          │
//! ├────────────┴────────────┴────────────┴──────────────────────┤
//! │  Package  │    Env     │    Evar    │    App    │  DepSpec  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start (Rust)
//!
//! ```ignore
//! use pkg::{Storage, Solver, Package, Env, Evar, App};
//!
//! // Scan for packages
//! let storage = Storage::scan()?;
//!
//! // Resolve dependencies
//! let solver = Solver::new(storage.packages());
//! let solution = solver.solve("maya-2026.1.0")?;
//!
//! // Get package and set up environment
//! let pkg = storage.get("maya-2026.1.0").unwrap();
//! if let Some(env) = pkg.default_env() {
//!     let solved = env.solve(10, true)?;
//!     solved.commit();
//! }
//! ```
//!
//! # Quick Start (Python)
//!
//! ```python
//! from pkg import Storage, Solver, Package, Env, Evar, App
//!
//! # Scan for packages
//! storage = Storage.scan()
//!
//! # Resolve dependencies
//! solver = Solver(storage.packages)
//! solution = solver.solve("maya-2026.1.0")
//!
//! # Get package and set up environment
//! pkg = storage.get("maya-2026.1.0")
//! env = pkg.default_env()
//! solved = env.solve()
//! solved.commit()
//! ```
//!
//! # Package.py Example
//!
//! ```python
//! from pkg import Package, Env, Evar, App
//! from pathlib import Path
//! import sys
//!
//! def get_package(*args, **kwargs):
//!     pkg = Package("maya", "2026.1.0")
//!
//!     # Requirements
//!     pkg.reqs.append("redshift@>=3.5,<4.0")
//!     pkg.reqs.append("ocio@2")
//!
//!     # Environment
//!     if sys.platform == "win32":
//!         root = Path("C:/Program Files/Autodesk/Maya2026")
//!     else:
//!         root = Path("/opt/autodesk/maya2026")
//!
//!     env = Env("default")
//!     env.add(Evar("MAYA_ROOT", str(root), action="set"))
//!     env.add(Evar("PATH", str(root / "bin"), action="append"))
//!     pkg.envs.append(env)
//!
//!     # Application
//!     exe = root / "bin" / ("maya.exe" if sys.platform == "win32" else "maya")
//!     app = App("maya", path=str(exe), env_name="default")
//!     pkg.apps.append(app)
//!
//!     return pkg
//! ```
//!
//! # Core Types
//!
//! - [`Package`] - Software package with envs, apps, requirements
//! - [`Env`] - Named collection of environment variables
//! - [`Evar`] - Single environment variable with action (set/append/insert)
//! - [`App`] - Executable application definition
//! - [`DepSpec`] - Dependency specification parser
//! - [`Storage`] - Package discovery and indexing
//! - [`Solver`] - Dependency resolver (PubGrub)
//! - [`Loader`] - Package.py executor
//!
//! # Modules
//!
//! - [`app`] - Application definitions
//! - [`dep`] - Dependency specification parsing
//! - [`env`](mod@env) - Environment collections
//! - [`error`] - Error types
//! - [`evar`] - Environment variables
//! - [`loader`] - Package.py loading
//! - [`package`] - Package definitions
//! - [`solver`] - Dependency resolution
//! - [`storage`] - Package discovery
//!
//! # Features
//!
//! - `python` (default) - Enable Python bindings via PyO3

pub mod app;
pub mod build;
pub mod build_command;
pub mod cache;
pub mod dep;
pub mod env;
pub mod error;
pub mod evar;
pub mod loader;
pub mod name;
pub mod package;
pub mod pip;
pub mod solver;
pub mod storage;
pub mod token;
pub mod toolset;

pub mod gui;

// Re-exports for convenience
pub use app::App;
pub use dep::DepSpec;
pub use env::Env;
pub use error::{BuildError, EnvError, EvarError, LoaderError, PackageError, PipError, PkgError, SolverError, StorageError};
pub use evar::{Action, Evar};
pub use loader::Loader;
pub use package::{Package, SolveStatus};
pub use build_command::BuildCommand;
pub use solver::{PackageIndex, Solver};
pub use storage::Storage;

use pyo3::prelude::*;

/// Library version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get library version.
#[pyfunction]
fn version() -> &'static str {
    VERSION
}

/// Python module initialization.
///
/// Creates the `pkg` Python module with all classes and functions.
/// Built via `maturin build` to produce a `.pyd` (Windows) or `.so` (Unix) file.
///
/// # Usage
///
/// ```python
/// import pkg
/// print(pkg.version())
///
/// from pkg import Package, Env, Evar, App
/// ```
#[pymodule]
fn pkg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version function
    m.add_function(wrap_pyfunction!(version, m)?)?;

    // Core classes
    m.add_class::<Package>()?;
    m.add_class::<Env>()?;
    m.add_class::<Evar>()?;
    m.add_class::<App>()?;
    m.add_class::<Action>()?;
    m.add_class::<package::SolveStatus>()?;

    // Dependency handling
    m.add_class::<DepSpec>()?;

    // Storage and resolution
    m.add_class::<Storage>()?;
    m.add_class::<Solver>()?;
    m.add_class::<Loader>()?;

    // Module docstring
    m.add("__doc__", "pkg: Software package management system.")?;
    m.add("__version__", VERSION)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_reexports() {
        // Verify re-exports work
        let _pkg = Package::new("test".to_string(), "1.0.0".to_string());
        let _env = Env::new("default".to_string());
        let _evar = Evar::set("TEST", "value");
        let _app = App::named("test");
    }

    #[test]
    fn integration_basic() {
        // Create a package
        let mut pkg = Package::new("maya".to_string(), "2026.1.0".to_string());

        // Add requirement
        pkg.add_req("redshift@>=3.5".to_string());

        // Create environment
        let mut env = Env::new("default".to_string());
        env.add(Evar::set("MAYA_ROOT", "/opt/maya"));
        env.add(Evar::append("PATH", "{MAYA_ROOT}/bin"));
        pkg.add_env(env);

        // Create application
        let app = App::named("maya")
            .with_path("/opt/maya/bin/maya")
            .with_env("default");
        pkg.add_app(app);

        // Verify
        assert_eq!(pkg.name, "maya-2026.1.0");
        assert_eq!(pkg.envs.len(), 1);
        assert_eq!(pkg.apps.len(), 1);
        assert_eq!(pkg.reqs.len(), 1);

        // Get default env and solve
        let env = pkg.default_env().unwrap();
        let solved = env.solve_impl(10, false).unwrap();
        let path = solved.get("PATH").unwrap();
        assert_eq!(path.value(), "/opt/maya/bin");
    }

    #[test]
    fn integration_storage_solver() {
        // Create test packages
        let pkg1 = Package::new("maya".to_string(), "2026.0.0".to_string());
        let mut pkg2 = Package::new("maya".to_string(), "2026.1.0".to_string());
        pkg2.add_req("redshift@>=3.0".to_string());
        let pkg3 = Package::new("redshift".to_string(), "3.5.0".to_string());

        // Create storage
        let storage = Storage::from_packages(vec![pkg1, pkg2, pkg3]);

        assert_eq!(storage.count(), 3);
        assert!(storage.has("maya-2026.1.0"));

        let versions = storage.versions("maya");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0], "maya-2026.1.0"); // Newest first

        // Create solver
        let solver = Solver::new(storage.packages()).unwrap();
        assert!(solver.has_package("maya"));
        assert!(solver.has_package("redshift"));
    }

    #[test]
    fn integration_depspec() {
        // Parse various formats
        let spec1 = DepSpec::parse_impl("redshift@>=3.5,<4.0").unwrap();
        assert_eq!(spec1.base, "redshift");
        assert!(spec1.matches_impl("3.5.2").unwrap());
        assert!(!spec1.matches_impl("4.0.0").unwrap());

        let spec2 = DepSpec::parse_impl("maya-2026.1.0").unwrap();
        assert_eq!(spec2.base, "maya");
        assert!(spec2.is_exact());

        let spec3 = DepSpec::parse_impl("python").unwrap();
        assert!(spec3.is_any());
    }
}
