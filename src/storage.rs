//! Package storage and discovery.
//!
//! This module provides [`Storage`] - a registry of available packages
//! discovered from filesystem locations. It scans directories for
//! `package.py` files and loads them using the [`Loader`](crate::loader::Loader).
//!
//! # Overview
//!
//! Storage scans multiple locations for packages:
//! 1. Default location (platform-specific)
//! 2. Paths from `PKG_LOCATIONS` environment variable
//! 3. Explicitly added paths
//!
//! Each location is scanned recursively for `package.py` files.
//! Found packages are validated and indexed by name and version.
//!
//! # Directory Structure
//!
//! Packages are organized in directories:
//!
//! ```text
//! /packages/
//! ├── maya/
//! │   ├── 2025.0.0/
//! │   │   └── package.py
//! │   ├── 2026.0.0/
//! │   │   └── package.py
//! │   └── 2026.1.0/
//! │       └── package.py
//! ├── redshift/
//! │   └── 3.5.0/
//! │       └── package.py
//! └── houdini/
//!     └── 20.0.0/
//!         └── package.py
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use pkg::Storage;
//!
//! // Scan default locations
//! let storage = Storage::scan()?;
//!
//! // Get all packages
//! for pkg in storage.packages() {
//!     println!("{} ({})", pkg.name, pkg.version);
//! }
//!
//! // Find specific package
//! if let Some(maya) = storage.get("maya-2026.1.0") {
//!     println!("Found: {:?}", maya);
//! }
//!
//! // Get all versions of a package
//! let maya_versions = storage.versions("maya");
//! ```
//!
//! # Environment Variables
//!
//! - `PKG_LOCATIONS`: Colon/semicolon-separated list of additional
//!   directories to scan for packages.
//!
//! # Python API
//!
//! ```python
//! from pkg import Storage
//!
//! # Scan default locations
//! storage = Storage.scan()
//!
//! # Or scan specific paths
//! storage = Storage.scan_paths(["/my/packages", "/other/packages"])
//!
//! # Query packages
//! pkg = storage.get("maya-2026.1.0")
//! all_maya = storage.versions("maya")
//! all_pkgs = storage.packages
//! ```

use crate::cache::Cache;
use crate::dep::DepSpec;
use crate::error::StorageError;
use crate::package::Package;
use jwalk::WalkDir;
use log::{debug, info, trace, warn};
use pyo3::prelude::*;

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Environment variable for additional package locations.
const PKG_LOCATIONS_VAR: &str = "PKG_LOCATIONS";

/// Default package file name.
const PACKAGE_FILE: &str = "package.py";

/// Package storage and discovery.
///
/// Holds all discovered packages and provides lookup functionality.
/// Packages are indexed by their full name (`base-version`) for fast access.
///
/// # Thread Safety
///
/// Storage is immutable after construction. Create a new Storage
/// to refresh the package list.
#[pyclass]
#[derive(Debug, Clone)]
pub struct Storage {
    /// All discovered packages, indexed by full name (base-version).
    packages: HashMap<String, Package>,

    /// Packages grouped by base name for version queries.
    by_base: HashMap<String, Vec<String>>,

    /// Scanned locations.
    locations: Vec<PathBuf>,

    /// Errors encountered during scanning (non-fatal).
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

#[pymethods]
impl Storage {
    /// Create empty storage.
    #[new]
    pub fn empty() -> Self {
        Self {
            packages: HashMap::new(),
            by_base: HashMap::new(),
            locations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Scan default locations for packages.
    ///
    /// Scans:
    /// 1. Platform default location
    /// 2. Paths from PKG_LOCATIONS env var
    ///
    /// # Returns
    /// Storage with discovered packages.
    #[staticmethod]
    pub fn scan() -> PyResult<Self> {
        Self::scan_impl(None)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Scan specific paths for packages.
    ///
    /// # Arguments
    /// * `paths` - List of directory paths to scan
    #[staticmethod]
    pub fn scan_paths(paths: Vec<String>) -> PyResult<Self> {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        Self::scan_impl(Some(&paths))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Get package by full name.
    ///
    /// # Arguments
    /// * `name` - Full package name (e.g., "maya-2026.1.0")
    ///
    /// # Returns
    /// Package if found, None otherwise.
    pub fn get(&self, name: &str) -> Option<Package> {
        self.packages.get(name).cloned()
    }

    /// Get all versions of a package.
    ///
    /// # Arguments
    /// * `base` - Package base name (e.g., "maya")
    ///
    /// # Returns
    /// List of full package names, sorted by version (newest first).
    pub fn versions(&self, base: &str) -> Vec<String> {
        self.by_base.get(base).cloned().unwrap_or_default()
    }

    /// Get all package base names.
    pub fn bases(&self) -> Vec<String> {
        self.by_base.keys().cloned().collect()
    }

    /// Get number of packages.
    pub fn count(&self) -> usize {
        self.packages.len()
    }

    /// Check if a package exists.
    pub fn has(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Check if any version of a base package exists.
    pub fn has_base(&self, base: &str) -> bool {
        self.by_base.contains_key(base)
    }

    /// Get all packages as a list.
    #[getter]
    pub fn packages(&self) -> Vec<Package> {
        self.packages.values().cloned().collect()
    }

    /// List packages with optional tag filter.
    ///
    /// # Arguments
    /// * `tags` - Filter by tags (package must have ALL specified tags)
    ///
    /// # Example
    /// ```python
    /// all_pkgs = storage.list()
    /// dcc_pkgs = storage.list(tags=["dcc"])
    /// adobe_render = storage.list(tags=["adobe", "render"])
    /// ```
    #[pyo3(signature = (tags = None))]
    pub fn list(&self, tags: Option<Vec<String>>) -> Vec<Package> {
        let tags = tags.unwrap_or_default();
        
        if tags.is_empty() {
            return self.packages.values().cloned().collect();
        }

        self.packages
            .values()
            .filter(|pkg| tags.iter().all(|t| pkg.tags.contains(t)))
            .cloned()
            .collect()
    }

    /// Get scanned locations (as strings for Python).
    #[getter]
    pub fn locations(&self) -> Vec<String> {
        self.locations
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// Get raw location paths (Rust only).
    pub fn location_paths(&self) -> &[PathBuf] {
        &self.locations
    }

    /// Find packages matching a pattern.
    ///
    /// # Arguments
    /// * `pattern` - Glob-like pattern (supports * wildcard)
    ///
    /// # Example
    /// ```python
    /// storage.find("maya-*")  # All maya versions
    /// storage.find("*-2026.*")  # All 2026 versions
    /// ```
    pub fn find(&self, pattern: &str) -> Vec<String> {
        let pattern = pattern.replace('*', ".*");
        let re = regex::Regex::new(&format!("^{}$", pattern)).ok();

        match re {
            Some(re) => self
                .packages
                .keys()
                .filter(|name| re.is_match(name))
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get latest version of a package.
    ///
    /// # Arguments
    /// * `base` - Package base name
    ///
    /// # Returns
    /// Latest package or None if not found.
    pub fn latest(&self, base: &str) -> Option<Package> {
        self.versions(base).first().and_then(|name| self.get(name))
    }

    /// Resolve package name with flexible syntax.
    ///
    /// Supports multiple formats:
    /// - `"maya"` - latest version of maya
    /// - `"maya-2026.1.0"` - exact version match
    /// - `"maya@2025"` - latest 2025.x.x version
    /// - `"maya@>=2024,<2026"` - latest matching constraint
    ///
    /// # Arguments
    /// * `name` - Package name with optional version constraint
    ///
    /// # Returns
    /// Best matching package or None.
    pub fn resolve(&self, name: &str) -> Option<Package> {
        // Version requirement syntax: name@constraint
        if let Some(idx) = name.find('@') {
            let base = &name[..idx];
            
            // Parse constraint once, reuse for matching
            let spec = DepSpec::parse_impl(name).ok()?;
            
            // Iterate packages directly (versions are sorted newest-first)
            self.by_base
                .get(base)?
                .iter()
                .filter_map(|n| self.packages.get(n))
                .find(|pkg| spec.matches_impl(&pkg.version).unwrap_or(false))
                .cloned()
        } else {
            // Standard: exact match or latest
            self.get(name).or_else(|| self.latest(name))
        }
    }

    /// Manually add a package.
    ///
    /// Used for testing or dynamically loaded packages.
    pub fn add(&mut self, pkg: Package) {
        let name = pkg.name.clone();
        let base = pkg.base.clone();

        self.packages.insert(name.clone(), pkg);

        self.by_base
            .entry(base.clone())
            .or_default()
            .push(name.clone());

        // Re-sort versions
        if let Some(versions) = self.by_base.get_mut(&base) {
            sort_versions_vec(versions);
        }
    }

    /// Refresh storage by rescanning locations.
    ///
    /// # Returns
    /// New Storage with refreshed packages.
    pub fn refresh(&self) -> PyResult<Self> {
        Self::scan_impl(Some(&self.locations))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Storage({} packages from {} locations)",
            self.packages.len(),
            self.locations.len()
        )
    }

    fn __len__(&self) -> usize {
        self.packages.len()
    }

    fn __contains__(&self, name: &str) -> bool {
        self.has(name)
    }
}

// Pure Rust API
impl Storage {
    /// Internal scan implementation with caching and parallel scanning.
    pub fn scan_impl(paths: Option<&[PathBuf]>) -> Result<Self, StorageError> {
        info!("Storage: scanning for packages");
        
        // Initialize Python interpreter for Loader
        // Safe to call multiple times - no-op if already initialized
        let _ = pyo3::Python::initialize();
        trace!("Storage: Python interpreter initialized");

        // Load cache
        let mut cache = Cache::load();
        let cache_hits = Arc::new(Mutex::new(0usize));
        let cache_misses = Arc::new(Mutex::new(0usize));

        let mut storage = Self::empty();

        // Determine locations to scan
        let locations = match paths {
            Some(p) => {
                debug!("Storage: using {} custom paths", p.len());
                p.to_vec()
            }
            None => {
                let locs = Self::default_locations();
                debug!("Storage: using {} default locations", locs.len());
                locs
            }
        };

        storage.locations = locations.clone();

        // Collect all package.py files in parallel using jwalk
        let package_files: Vec<PathBuf> = locations
            .iter()
            .filter(|loc| loc.exists())
            .flat_map(|location| {
                debug!("Storage: walking {}", location.display());
                WalkDir::new(location)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter(|e| e.file_name().to_string_lossy() == PACKAGE_FILE)
                    .map(|e| e.path())
                    .collect::<Vec<_>>()
            })
            .collect();

        debug!("Storage: found {} package.py files", package_files.len());

        // Load packages (with cache)
        for path in &package_files {
            // Try cache first
            if let Some(pkg) = cache.get(path) {
                *cache_hits.lock().unwrap() += 1;
                
                // Check for duplicates
                if storage.packages.contains_key(&pkg.name) {
                    storage.warnings.push(format!(
                        "Duplicate package '{}': ignoring {} (first location wins)",
                        pkg.name, path.display()
                    ));
                    continue;
                }
                
                let name = pkg.name.clone();
                let base = pkg.base.clone();
                storage.packages.insert(name.clone(), pkg.clone());
                storage.by_base.entry(base).or_default().push(name);
                continue;
            }

            // Cache miss - load from disk
            *cache_misses.lock().unwrap() += 1;
            
            match storage.load_package_cached(path, &mut cache) {
                Ok(()) => {},
                Err(e) => {
                    storage.warnings.push(format!(
                        "Failed to load {}: {}",
                        path.display(), e
                    ));
                }
            }
        }

        // Scan toolsets for each location
        for location in &locations {
            if location.exists() {
                storage.scan_toolsets(location);
            }
        }

        // Sort versions for each base (newest first)
        for versions in storage.by_base.values_mut() {
            sort_versions_vec(versions);
        }

        // Prune and save cache
        cache.prune();
        cache.save();

        let hits = *cache_hits.lock().unwrap();
        let misses = *cache_misses.lock().unwrap();
        info!("Storage: found {} packages (cache: {} hits, {} misses)", 
              storage.packages.len(), hits, misses);
        
        Ok(storage)
    }

    /// Get default locations to scan.
    ///
    /// Priority (fallback system):
    /// 1. scan_paths() args - handled by caller
    /// 2. PKG_LOCATIONS env var
    /// 3. "repo" folder in cwd (if exists)
    /// 4. nothing
    fn default_locations() -> Vec<PathBuf> {
        let mut locations = Vec::new();

        // 1. Environment variable (highest priority for default scan)
        if let Ok(env_paths) = env::var(PKG_LOCATIONS_VAR) {
            let separator = if cfg!(windows) { ';' } else { ':' };
            for path in env_paths.split(separator) {
                let path = path.trim();
                if !path.is_empty() {
                    let p = PathBuf::from(path);
                    if !locations.contains(&p) {
                        locations.push(p);
                    }
                }
            }
        }

        // 2. Fallback: "repo" folder in cwd (only if env var not set)
        if locations.is_empty() {
            if let Ok(cwd) = env::current_dir() {
                let repo_path = cwd.join("repo");
                if repo_path.exists() {
                    locations.push(repo_path);
                }
            }
        }

        locations
    }

    /// Scan .toolsets directory for toolset definitions.
    fn scan_toolsets(&mut self, location: &Path) {
        use crate::toolset::scan_toolsets_dir;
        
        let toolset_packages = scan_toolsets_dir(location);
        
        for pkg in toolset_packages {
            // Check for duplicates (first wins with warning)
            if self.packages.contains_key(&pkg.name) {
                self.warnings.push(format!(
                    "Duplicate package '{}': ignoring toolset (first location wins)",
                    pkg.name
                ));
                warn!(
                    "Duplicate package '{}': ignoring toolset (first location wins)",
                    pkg.name
                );
                continue;
            }
            
            // Add to storage
            let name = pkg.name.clone();
            let base = pkg.base.clone();
            
            self.packages.insert(name.clone(), pkg);
            self.by_base.entry(base).or_default().push(name);
        }
    }

    /// Load a single package.py file and update cache.
    fn load_package_cached(&mut self, path: &Path, cache: &mut Cache) -> Result<(), StorageError> {
        use crate::loader::Loader;

        trace!("Storage: loading package from {}", path.display());

        // Use Loader to execute package.py and get Package
        let mut loader = Loader::new(Some(false));
        let mut pkg = loader.load_path(path).map_err(|e| {
            debug!("Storage: failed to load {}: {}", path.display(), e);
            StorageError::InvalidPackage {
                path: path.to_path_buf(),
                reason: e.to_string(),
            }
        })?;

        // Set source path
        pkg.package_source = Some(path.to_string_lossy().to_string());

        // Update cache
        cache.insert(path.to_path_buf(), pkg.clone());

        // Check for duplicates (first wins with warning)
        let name = pkg.name.clone();
        if self.packages.contains_key(&name) {
            self.warnings.push(format!(
                "Duplicate package '{}': ignoring {} (first location wins)",
                name, path.display()
            ));
            return Ok(());
        }
        
        // Index it
        let base = pkg.base.clone();
        info!("Storage: loaded package {} ({})", name, base);
        self.packages.insert(name.clone(), pkg);
        self.by_base.entry(base).or_default().push(name);

        Ok(())
    }

    /// Get all packages as a vector (for Solver).
    /// Note: Clones all packages. Use `packages_iter()` for zero-copy iteration.
    pub fn all_packages(&self) -> Vec<Package> {
        self.packages.values().cloned().collect()
    }

    /// Iterate over packages without cloning (Rust-only).
    /// More efficient than `all_packages()` when you only need to read.
    pub fn packages_iter(&self) -> impl Iterator<Item = &Package> {
        self.packages.values()
    }


    /// Create storage from a list of packages (for testing).
    pub fn from_packages(packages: Vec<Package>) -> Self {
        let mut storage = Self::empty();
        for pkg in packages {
            storage.add(pkg);
        }
        storage
    }
    
    /// Exclude packages matching patterns (glob-style: * matches anything).
    pub fn exclude_packages(&mut self, patterns: &[String]) {
        use log::debug;
        
        let to_remove: Vec<String> = self.packages.keys()
            .filter(|name| {
                patterns.iter().any(|pat| {
                    if pat.contains('*') {
                        // Simple glob: * matches any chars
                        let parts: Vec<&str> = pat.split('*').collect();
                        if parts.len() == 2 {
                            name.starts_with(parts[0]) && name.ends_with(parts[1])
                        } else if pat.starts_with('*') {
                            name.ends_with(&pat[1..])
                        } else if pat.ends_with('*') {
                            name.starts_with(&pat[..pat.len()-1])
                        } else {
                            name.contains(&pat.replace('*', ""))
                        }
                    } else {
                        name.as_str() == pat.as_str() || name.starts_with(&format!("{}@", pat)) || name.starts_with(&format!("{}-", pat))
                    }
                })
            })
            .cloned()
            .collect();
        
        for name in &to_remove {
            if let Some(pkg) = self.packages.remove(name) {
                debug!("Excluded package: {}", name);
                // Remove from by_base
                if let Some(versions) = self.by_base.get_mut(&pkg.base) {
                    versions.retain(|v| v != name);
                }
            }
        }
    }
    
    /// Get user packages directory (~/.pkg-rs/packages).
    ///
    /// This directory is used for user-specific packages and toolsets.
    /// Add with `-u` / `--user-packages` flag.
    ///
    /// Structure:
    /// ```text
    /// ~/.pkg-rs/
    ///   packages/           # user package overrides
    ///     .toolsets/        # user toolsets
    ///       my-env.toml
    /// ```
    pub fn user_packages_dir() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|h| h.join(".pkg-rs").join("packages"))
    }
}

/// Sort versions newest-first using semver comparison.
/// Standalone function to avoid borrow conflicts.
fn sort_versions_vec(versions: &mut Vec<String>) {
    versions.sort_by(|a, b| {
        let va = Package::parse_name(a)
            .ok()
            .and_then(|(_, v)| semver::Version::parse(&v).ok());
        let vb = Package::parse_name(b)
            .ok()
            .and_then(|(_, v)| semver::Version::parse(&v).ok());

        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va), // Reverse for newest first
            _ => b.cmp(a),                       // Fallback to string comparison
        }
    });
}

impl Default for Storage {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_empty() {
        let storage = Storage::empty();
        assert_eq!(storage.count(), 0);
        assert!(storage.packages().is_empty());
    }

    #[test]
    fn storage_add() {
        let mut storage = Storage::empty();

        let pkg1 = Package::new("maya".to_string(), "2026.0.0".to_string());
        let pkg2 = Package::new("maya".to_string(), "2026.1.0".to_string());
        let pkg3 = Package::new("houdini".to_string(), "20.0.0".to_string());

        storage.add(pkg1);
        storage.add(pkg2);
        storage.add(pkg3);

        assert_eq!(storage.count(), 3);
        assert!(storage.has("maya-2026.0.0"));
        assert!(storage.has("maya-2026.1.0"));
        assert!(storage.has("houdini-20.0.0"));
        assert!(!storage.has("nuke-14.0.0"));
    }

    #[test]
    fn storage_versions() {
        let mut storage = Storage::empty();

        storage.add(Package::new("maya".to_string(), "2025.0.0".to_string()));
        storage.add(Package::new("maya".to_string(), "2026.1.0".to_string()));
        storage.add(Package::new("maya".to_string(), "2026.0.0".to_string()));

        let versions = storage.versions("maya");
        assert_eq!(versions.len(), 3);

        // Should be sorted newest first
        assert_eq!(versions[0], "maya-2026.1.0");
        assert_eq!(versions[1], "maya-2026.0.0");
        assert_eq!(versions[2], "maya-2025.0.0");
    }

    #[test]
    fn storage_latest() {
        let mut storage = Storage::empty();

        storage.add(Package::new("maya".to_string(), "2025.0.0".to_string()));
        storage.add(Package::new("maya".to_string(), "2026.1.0".to_string()));

        let latest = storage.latest("maya").unwrap();
        assert_eq!(latest.version, "2026.1.0");
    }

    #[test]
    fn storage_find() {
        let mut storage = Storage::empty();

        storage.add(Package::new("maya".to_string(), "2026.0.0".to_string()));
        storage.add(Package::new("maya".to_string(), "2026.1.0".to_string()));
        storage.add(Package::new("houdini".to_string(), "20.0.0".to_string()));

        let maya_all = storage.find("maya-*");
        assert_eq!(maya_all.len(), 2);

        let v2026 = storage.find("*-2026.*");
        assert_eq!(v2026.len(), 2);
    }

    #[test]
    fn storage_bases() {
        let mut storage = Storage::empty();

        storage.add(Package::new("maya".to_string(), "2026.0.0".to_string()));
        storage.add(Package::new("houdini".to_string(), "20.0.0".to_string()));

        let bases = storage.bases();
        assert_eq!(bases.len(), 2);
        assert!(bases.contains(&"maya".to_string()));
        assert!(bases.contains(&"houdini".to_string()));
    }
}
