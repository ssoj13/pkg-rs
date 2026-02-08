//! Dependency resolution.
//!
//! This module provides the [`Solver`] for resolving package dependencies.
//! Uses PubGrub SAT-solver for correct resolution with backtracking.
//!
//! # Overview
//!
//! The solver takes:
//! - A root package (or list of requirements)
//! - Available packages from [`Storage`](crate::storage::Storage)
//!
//! And produces:
//! - A list of concrete package versions that satisfy all constraints
//! - Or an error explaining why resolution failed
//!
//! # Usage
//!
//! ```ignore
//! use pkg::{Solver, Storage, Package};
//!
//! let storage = Storage::scan()?;
//! let solver = Solver::new(storage.packages())?;
//!
//! // Solve for a single package
//! let solution = solver.solve("maya-2026.1.0")?;
//! println!("Resolved packages: {:?}", solution);
//!
//! // Solve for multiple requirements
//! let reqs = vec!["maya@2026", "redshift@>=3.5"];
//! let solution = solver.solve_reqs(&reqs)?;
//! ```
//!
//! # Python API
//!
//! ```python
//! from pkg import Solver, Storage
//!
//! storage = Storage.scan()
//! solver = Solver(storage.packages)
//!
//! try:
//!     solution = solver.solve("maya-2026.1.0")
//!     for pkg_name in solution:
//!         print(f"  {pkg_name}")
//! except RuntimeError as e:
//!     print(f"Resolution failed: {e}")
//! ```

mod provider;
mod ranges;

use crate::dep::DepSpec;
use crate::error::SolverError;
use crate::package::Package;
use crate::py::ensure_rez_on_sys_path;
use log::{debug, info};
use pyo3::prelude::*;
use pyo3::types::PyList;
use semver::Version;
use std::collections::HashMap;

// Re-export PubGrub provider for advanced usage
pub use provider::PubGrubProvider;
pub use ranges::depspec_to_ranges;

/// Resolver backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverBackend {
    Pkg,
    Rez,
}

pub fn selected_backend() -> Result<ResolverBackend, SolverError> {
    if let Ok(config) = crate::config::get() {
        if let Some(raw) = crate::config::resolver_backend(config) {
            let name = raw.trim().to_ascii_lowercase();
            return match name.as_str() {
                "pkg" | "pubgrub" => Ok(ResolverBackend::Pkg),
                "rez" => Ok(ResolverBackend::Rez),
                _ => Err(SolverError::UnsupportedBackend { backend: raw }),
            };
        }
    }
    Ok(ResolverBackend::Pkg)
}

pub fn solve_reqs_backend(
    packages: &[Package],
    requirements: Vec<String>,
) -> Result<Vec<String>, SolverError> {
    match selected_backend()? {
        ResolverBackend::Pkg => {
            let solver = Solver::from_packages(packages)?;
            solver.solve_requirements_impl(&requirements)
        }
        ResolverBackend::Rez => solve_reqs_rez(&requirements),
    }
}

struct PyConfigSwapGuard {
    ctx: Py<PyAny>,
}

impl PyConfigSwapGuard {
    fn enter(_py: Python<'_>, ctx: &Bound<'_, PyAny>) -> PyResult<Self> {
        ctx.call_method0("__enter__")?;
        let ctx_obj: Py<PyAny> = ctx.clone().unbind();
        Ok(Self { ctx: ctx_obj })
    }
}

impl Drop for PyConfigSwapGuard {
    fn drop(&mut self) {
        Python::attach(|py| {
            let ctx = self.ctx.bind(py);
            let _ = ctx.call_method1("__exit__", (py.None(), py.None(), py.None()));
        });
    }
}

fn solve_reqs_rez(requirements: &[String]) -> Result<Vec<String>, SolverError> {
    if requirements.is_empty() {
        return Ok(Vec::new());
    }

    let filepaths = crate::config::get().map_err(|e| SolverError::BackendError {
        backend: "rez".to_string(),
        message: format!("failed to load rez config: {e}"),
    })?;
    let filepaths = filepaths
        .filepaths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let _ = Python::initialize();
    Python::attach(|py| {
        ensure_rez_on_sys_path(py).map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("failed to set rez python path: {e}"),
        })?;

        let config_mod = py.import("rez.config").map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("failed to import rez.config: {e}"),
        })?;
        let config_cls = config_mod.getattr("Config").map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("failed to access rez.config.Config: {e}"),
        })?;
        let filepaths_py = PyList::new(py, &filepaths).map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("failed to build rez config file list: {e}"),
        })?;
        let config_obj = config_cls
            .call1((filepaths_py,))
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to create rez Config: {e}"),
            })?;

        let replace_ctx = config_mod
            .getattr("_replace_config")
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to access rez.config._replace_config: {e}"),
            })?
            .call1((config_obj,))
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to start rez config swap: {e}"),
            })?;
        let _guard = PyConfigSwapGuard::enter(py, &replace_ctx).map_err(|e| {
            SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to enter rez config context: {e}"),
            }
        })?;

        let resolved_context = py
            .import("rez.resolved_context")
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to import rez.resolved_context: {e}"),
            })?;
        let ctx_cls = resolved_context
            .getattr("ResolvedContext")
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to access ResolvedContext: {e}"),
            })?;
        let reqs_py = PyList::new(py, requirements).map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("failed to build requirement list: {e}"),
        })?;
        let ctx = ctx_cls
            .call1((reqs_py,))
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("rez resolve failed to start: {e}"),
            })?;

        let success = ctx
            .getattr("success")
            .and_then(|v| v.extract::<bool>())
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to read rez resolve status: {e}"),
            })?;

        if !success {
            let reason = ctx
                .getattr("failure_description")
                .ok()
                .and_then(|val| if val.is_none() { None } else { val.extract::<String>().ok() })
                .or_else(|| {
                    ctx.getattr("status")
                        .ok()
                        .and_then(|s| s.getattr("name").ok())
                        .and_then(|n| n.extract::<String>().ok())
                })
                .unwrap_or_else(|| "rez solver failed".to_string());
            return Err(SolverError::NoSolution { reason });
        }

        let resolved = ctx
            .getattr("resolved_packages")
            .map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to read resolved packages: {e}"),
            })?;

        if resolved.is_none() {
            return Err(SolverError::NoSolution {
                reason: "rez resolve returned no packages".to_string(),
            });
        }

        let resolved_list = resolved.cast::<PyList>().map_err(|e| SolverError::BackendError {
            backend: "rez".to_string(),
            message: format!("resolved packages is not a list: {e}"),
        })?;

        let mut out = Vec::with_capacity(resolved_list.len());
        for item in resolved_list.iter() {
            let name_attr = item
                .getattr("qualified_package_name")
                .or_else(|_| item.getattr("qualified_name"))
                .map_err(|e| SolverError::BackendError {
                    backend: "rez".to_string(),
                    message: format!("failed to read resolved package name: {e}"),
                })?;
            let name = name_attr.extract::<String>().map_err(|e| SolverError::BackendError {
                backend: "rez".to_string(),
                message: format!("failed to decode resolved package name: {e}"),
            })?;
            out.push(name);
        }

        Ok(out)
    })
}

/// Package index for solver.
///
/// Maps package base names to available versions and their dependencies.
/// Built from Storage's package list.
#[derive(Debug, Clone, Default)]
pub struct PackageIndex {
    /// Map: base name -> sorted list of (version, dependencies)
    packages: HashMap<String, Vec<(Version, Vec<DepSpec>)>>,
}

impl PackageIndex {
    /// Create new empty index.
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Add a package to the index.
    pub fn add(&mut self, pkg: &Package) -> Result<(), SolverError> {
        let version = Version::parse(&pkg.version).map_err(|e| SolverError::InvalidVersion {
            package: pkg.name.clone(),
            version: pkg.version.clone(),
            reason: e.to_string(),
        })?;

        // Parse requirements
        let deps: Vec<DepSpec> = pkg
            .reqs
            .iter()
            .map(|r| DepSpec::parse_impl(r))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SolverError::InvalidDependency {
                package: pkg.name.clone(),
                dependency: format!("{:?}", pkg.reqs),
                reason: e.to_string(),
            })?;

        self.packages
            .entry(pkg.base.clone())
            .or_default()
            .push((version, deps));

        // Sort versions descending (newest first)
        if let Some(versions) = self.packages.get_mut(&pkg.base) {
            versions.sort_by(|a, b| b.0.cmp(&a.0));
        }

        Ok(())
    }

    /// Get all versions of a package (newest first).
    pub fn versions(&self, base: &str) -> Vec<&Version> {
        self.packages
            .get(base)
            .map(|v| v.iter().map(|(ver, _)| ver).collect())
            .unwrap_or_default()
    }

    /// Get dependencies for a specific version.
    pub fn deps(&self, base: &str, version: &Version) -> Option<&Vec<DepSpec>> {
        self.packages.get(base).and_then(|versions| {
            versions
                .iter()
                .find(|(v, _)| v == version)
                .map(|(_, deps)| deps)
        })
    }

    /// Check if package exists.
    pub fn has(&self, base: &str) -> bool {
        self.packages.contains_key(base)
    }

    /// Get all base names.
    pub fn bases(&self) -> Vec<&String> {
        self.packages.keys().collect()
    }

    /// Find best matching version for a spec (newest first).
    pub fn find_match(&self, spec: &DepSpec) -> Option<Version> {
        let versions = self.packages.get(&spec.base)?;

        for (version, _) in versions {
            if spec.matches_impl(&version.to_string()).unwrap_or(false) {
                return Some(version.clone());
            }
        }

        None
    }

    /// Number of packages in index.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Dependency solver.
///
/// Resolves package dependencies using PubGrub SAT-solver.
/// Falls back to greedy algorithm if PubGrub fails.
#[pyclass]
#[derive(Clone)]
pub struct Solver {
    index: PackageIndex,
}

#[pymethods]
impl Solver {
    /// Create solver from package list.
    ///
    /// # Arguments
    /// * `packages` - List of Package objects
    #[new]
    pub fn new(packages: Vec<Package>) -> PyResult<Self> {
        let mut index = PackageIndex::new();

        for pkg in packages {
            index.add(&pkg)?;
        }

        Ok(Self { index })
    }

    /// Solve dependencies for a package.
    ///
    /// # Arguments
    /// * `package_name` - Full package name (e.g., "maya-2026.1.0")
    ///
    /// # Returns
    /// List of resolved package names.
    pub fn solve(&self, package_name: &str) -> PyResult<Vec<String>> {
        self.solve_impl(package_name)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Solve for multiple requirements.
    ///
    /// # Arguments
    /// * `requirements` - List of requirement strings
    ///
    /// # Returns
    /// List of resolved package names.
    pub fn solve_reqs(&self, requirements: Vec<String>) -> PyResult<Vec<String>> {
        self.solve_requirements_impl(&requirements)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Check if package exists in index.
    pub fn has_package(&self, base: &str) -> bool {
        self.index.has(base)
    }

    /// Get all versions of a package.
    pub fn versions(&self, base: &str) -> Vec<String> {
        self.index
            .versions(base)
            .into_iter()
            .map(|v| v.to_string())
            .collect()
    }

    /// Get all known package base names.
    pub fn packages(&self) -> Vec<String> {
        self.index.bases().into_iter().cloned().collect()
    }

    fn __repr__(&self) -> String {
        format!("Solver({} packages)", self.index.len())
    }
}

// Pure Rust API
impl Solver {
    /// Create solver from package slice (borrows, doesn't consume).
    pub fn from_packages(packages: &[Package]) -> Result<Self, SolverError> {
        let mut index = PackageIndex::new();
        for pkg in packages {
            index.add(pkg)?;
        }
        Ok(Self { index })
    }

    /// Create solver from package index.
    pub fn from_index(index: PackageIndex) -> Self {
        Self { index }
    }

    /// Solve using PubGrub algorithm.
    pub fn solve_impl(&self, package_name: &str) -> Result<Vec<String>, SolverError> {
        info!("Solver: resolving {}", package_name);

        // Parse package name
        let (base, version_str) =
            Package::parse_name(package_name).map_err(|e| SolverError::InvalidDependency {
                package: package_name.to_string(),
                dependency: "".to_string(),
                reason: e.to_string(),
            })?;

        let version = Version::parse(&version_str).map_err(|e| SolverError::InvalidVersion {
            package: package_name.to_string(),
            version: version_str.clone(),
            reason: e.to_string(),
        })?;

        // Verify package exists
        if !self.index.has(&base) {
            return Err(SolverError::PackageNotFound {
                package: package_name.to_string(),
            });
        }

        // Check version exists
        let versions = self.index.versions(&base);
        if !versions.iter().any(|v| **v == version) {
            return Err(SolverError::NoMatchingVersion {
                package: base.clone(),
                constraint: format!("={}", version_str),
            });
        }

        // Use PubGrub solver
        self.solve_pubgrub(&base, &version)
    }

    /// PubGrub-based resolution.
    fn solve_pubgrub(&self, base: &str, version: &Version) -> Result<Vec<String>, SolverError> {
        let provider = PubGrubProvider::new(&self.index);

        debug!("Solver: using PubGrub for {}-{}", base, version);

        // resolve() takes package name and starting version
        match pubgrub::resolve(&provider, base.to_string(), version.clone()) {
            Ok(solution) => {
                // Convert solution Map<String, Version> to Vec<String>
                let mut result: Vec<String> = solution
                    .into_iter()
                    .map(|(pkg, ver)| format!("{}-{}", pkg, ver))
                    .collect();

                result.sort();
                info!("Solver: resolved {} packages", result.len());
                Ok(result)
            }
            Err(pubgrub_error) => {
                // Convert PubGrub error to SolverError
                Err(provider::pubgrub_error_to_solver_error(pubgrub_error))
            }
        }
    }

    /// Solve for multiple requirements.
    pub fn solve_requirements_impl(
        &self,
        requirements: &[String],
    ) -> Result<Vec<String>, SolverError> {
        // Parse all requirements
        let specs: Vec<DepSpec> = requirements
            .iter()
            .map(|r| DepSpec::parse_impl(r))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SolverError::InvalidDependency {
                package: "root".to_string(),
                dependency: format!("{:?}", requirements),
                reason: e.to_string(),
            })?;

        if specs.is_empty() {
            return Ok(Vec::new());
        }

        // Create a virtual root package with all requirements
        let provider = PubGrubProvider::with_root_deps(&self.index, &specs);

        // Resolve from virtual root (version 0.0.0)
        match pubgrub::resolve(&provider, "__root__".to_string(), Version::new(0, 0, 0)) {
            Ok(solution) => {
                // Filter out virtual root, convert to package names
                let mut result: Vec<String> = solution
                    .into_iter()
                    .filter(|(pkg, _)| pkg != "__root__")
                    .map(|(pkg, ver)| format!("{}-{}", pkg, ver))
                    .collect();

                result.sort();
                info!("Solver: resolved {} packages from {} requirements", result.len(), specs.len());
                Ok(result)
            }
            Err(pubgrub_error) => {
                Err(provider::pubgrub_error_to_solver_error(pubgrub_error))
            }
        }
    }

    /// Get the package index.
    pub fn index(&self) -> &PackageIndex {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkg(name: &str, version: &str, reqs: Vec<&str>) -> Package {
        let mut pkg = Package::new(name.to_string(), version.to_string());
        for req in reqs {
            pkg.add_req(req.to_string());
        }
        pkg
    }

    #[test]
    fn solver_simple() {
        let packages = vec![
            make_pkg("maya", "2026.0.0", vec![]),
            make_pkg("maya", "2026.1.0", vec![]),
        ];

        let solver = Solver::new(packages).unwrap();
        let solution = solver.solve_impl("maya-2026.1.0").unwrap();

        assert_eq!(solution.len(), 1);
        assert!(solution.contains(&"maya-2026.1.0".to_string()));
    }

    #[test]
    fn solver_with_deps() {
        let packages = vec![
            make_pkg("maya", "2026.0.0", vec!["redshift@>=3.0"]),
            make_pkg("redshift", "3.0.0", vec![]),
            make_pkg("redshift", "3.5.0", vec![]),
        ];

        let solver = Solver::new(packages).unwrap();
        let solution = solver.solve_impl("maya-2026.0.0").unwrap();

        assert!(solution.contains(&"maya-2026.0.0".to_string()));
        // Should pick newest redshift that matches
        assert!(solution.contains(&"redshift-3.5.0".to_string()));
    }

    #[test]
    fn solver_package_not_found() {
        let packages = vec![make_pkg("maya", "2026.0.0", vec![])];

        let solver = Solver::new(packages).unwrap();
        let result = solver.solve_impl("houdini-20.0.0");

        assert!(result.is_err());
        if let Err(SolverError::PackageNotFound { package }) = result {
            assert_eq!(package, "houdini-20.0.0");
        }
    }

    #[test]
    fn solver_requirements() {
        let packages = vec![
            make_pkg("maya", "2026.0.0", vec![]),
            make_pkg("maya", "2026.1.0", vec![]),
            make_pkg("houdini", "20.0.0", vec![]),
        ];

        let solver = Solver::new(packages).unwrap();
        let reqs = vec!["maya@>=2026".to_string(), "houdini".to_string()];
        let solution = solver.solve_requirements_impl(&reqs).unwrap();

        assert!(solution.iter().any(|s| s.starts_with("maya-")));
        assert!(solution.iter().any(|s| s.starts_with("houdini-")));
    }

    #[test]
    fn package_index() {
        let mut index = PackageIndex::new();

        let pkg1 = make_pkg("maya", "2026.0.0", vec![]);
        let pkg2 = make_pkg("maya", "2026.1.0", vec![]);

        index.add(&pkg1).unwrap();
        index.add(&pkg2).unwrap();

        assert!(index.has("maya"));
        assert!(!index.has("houdini"));

        let versions = index.versions("maya");
        assert_eq!(versions.len(), 2);
        // Newest first
        assert_eq!(versions[0].to_string(), "2026.1.0");
    }
}
