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
use log::{debug, info};
use pyo3::prelude::*;
use semver::Version;
use std::collections::HashMap;

// Re-export PubGrub provider for advanced usage
pub use provider::PubGrubProvider;
pub use ranges::depspec_to_ranges;

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
