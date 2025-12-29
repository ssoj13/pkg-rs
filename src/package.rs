//! Package definition - the core data structure.
//!
//! A [`Package`] represents a software package with environments, applications,
//! and dependency requirements. Packages are defined in `package.py` files and
//! loaded by the [`Storage`](crate::storage::Storage) module.
//!
//! # Package Naming Convention
//!
//! Package names follow a strict format: `base-version` where:
//! - **base**: Package identifier (e.g., "maya", "redshift", "houdini")
//! - **version**: SemVer-compatible version (e.g., "2026.1.0", "3.5.0")
//!
//! The full name is `maya-2026.1.0` and is used as the unique identifier.
//!
//! # Requirements vs Dependencies
//!
//! - **reqs** (requirements): Version constraints defined in package.py
//!   - Examples: `"redshift@>=3.5,<4.0"`, `"ocio@2"`, `"python@>=3.10"`
//!   - Parsed by the solver to find compatible versions
//!
//! - **deps** (dependencies): Solved/resolved concrete versions
//!   - Examples: `"redshift-3.5.2"`, `"ocio-2.3.1"`, `"python-3.11.0"`
//!   - Populated by the solver after resolution
//!
//! # Environment Stamping
//!
//! When resolving environments, each package can "stamp" PKG_* variables:
//!
//! ```text
//! PKG_MAYA_ROOT=/packages/maya/2026.1.0
//! PKG_MAYA_VERSION=2026.1.0
//! PKG_MAYA_MAJOR=2026
//! PKG_MAYA_MINOR=1
//! PKG_MAYA_PATCH=0
//! PKG_MAYA_VARIANT=       # prerelease/build if any
//! ```
//!
//! Use `pkg.stamp()` to get these evars, or `pkg env --stamp` (enabled by default).
//!
//! # Package.py Example
//!
//! ```python
//! from pkg import Package, Env, Evar, App
//! from pathlib import Path
//! import sys
//!
//! def get_package(*args, **kwargs):
//!     # Create package with base name and version
//!     pkg = Package("maya", "2026.1.0")
//!
//!     # Add requirements (version constraints)
//!     pkg.reqs.append("redshift@>=3.5,<4.0")
//!     pkg.reqs.append("ocio@2")
//!     pkg.reqs.append("python@>=3.10")
//!
//!     # Create environment
//!     root = Path("/opt/autodesk/maya2026") if sys.platform != "win32" \
//!            else Path("C:/Program Files/Autodesk/Maya2026")
//!
//!     env = Env("default")
//!     env.add(Evar("MAYA_ROOT", str(root), action="set"))
//!     env.add(Evar("PATH", str(root / "bin"), action="append"))
//!     env.add(Evar("PYTHONPATH", str(root / "scripts"), action="append"))
//!     pkg.envs.append(env)
//!
//!     # Create application
//!     exe = root / "bin" / ("maya.exe" if sys.platform == "win32" else "maya")
//!     app = App(name="maya", path=str(exe), env_name="default")
//!     app.properties["icon"] = "maya.png"
//!     pkg.apps.append(app)
//!
//!     return pkg
//! ```
//!
//! # Rust Usage
//!
//! ```ignore
//! use pkg::{Package, Env, Evar, App};
//!
//! // Create package
//! let mut pkg = Package::new("maya", "2026.1.0");
//!
//! // Add requirement
//! pkg.add_req("redshift@>=3.5,<4.0");
//!
//! // Add environment
//! let mut env = Env::new("default");
//! env.add(Evar::set("MAYA_ROOT", "/opt/maya"));
//! pkg.add_env(env);
//!
//! // Add application
//! let app = App::named("maya").with_path("/opt/maya/bin/maya");
//! pkg.add_app(app);
//!
//! // Package name is auto-generated
//! assert_eq!(pkg.name, "maya-2026.1.0");
//! ```
//!
//! # Serialization
//!
//! ```json
//! {
//!   "name": "maya-2026.1.0",
//!   "base": "maya",
//!   "version": "2026.1.0",
//!   "envs": [...],
//!   "apps": [...],
//!   "reqs": ["redshift@>=3.5,<4.0", "ocio@2"],
//!   "deps": []
//! }
//! ```

use crate::app::App;
use crate::env::Env;
use crate::error::PackageError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use semver::Version;
use serde::{Deserialize, Serialize};

/// Status of package dependency resolution.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SolveStatus {
    /// Dependencies not yet resolved.
    #[default]
    NotSolved,
    /// Dependencies resolved successfully.
    Solved,
    /// Resolution failed with error.
    Failed,
}

#[pymethods]
impl SolveStatus {
    /// Check if status is Solved.
    pub fn is_ok(&self) -> bool {
        matches!(self, SolveStatus::Solved)
    }

    /// Check if status is Failed.
    pub fn is_error(&self) -> bool {
        matches!(self, SolveStatus::Failed)
    }

    /// Check if resolution was attempted.
    pub fn was_attempted(&self) -> bool {
        !matches!(self, SolveStatus::NotSolved)
    }

    fn __repr__(&self) -> String {
        match self {
            SolveStatus::NotSolved => "SolveStatus.NotSolved".to_string(),
            SolveStatus::Solved => "SolveStatus.Solved".to_string(),
            SolveStatus::Failed => "SolveStatus.Failed".to_string(),
        }
    }
}

/// Software package definition.
///
/// The central data structure containing all package information:
/// environments, applications, and dependencies.
///
/// # Naming
///
/// - `name`: Full package identifier (`maya-2026.1.0`)
/// - `base`: Package base name (`maya`)
/// - `version`: SemVer version string (`2026.1.0`)
///
/// The `name` field is automatically computed as `{base}-{version}`.
///
/// # Collections
///
/// - `envs`: Named environments (default, dev, debug, etc.)
/// - `apps`: Executable applications
/// - `reqs`: Dependency requirements (version constraints)
/// - `deps`: Resolved dependencies (concrete versions, populated by solver)
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// Full package name: `{base}-{version}`.
    /// Auto-computed from base and version.
    #[pyo3(get)]
    pub name: String,

    /// Base package name (e.g., "maya", "houdini", "redshift").
    #[pyo3(get, set)]
    pub base: String,

    /// Package version in SemVer format (e.g., "2026.1.0").
    #[pyo3(get)]
    pub version: String,

    /// Named environments (e.g., "default", "dev", "debug").
    /// Apps reference these by name.
    #[pyo3(get, set)]
    pub envs: Vec<Env>,

    /// Executable applications defined in this package.
    #[pyo3(get, set)]
    pub apps: Vec<App>,

    /// Dependency requirements (version constraints).
    /// Format: `name@constraint` or just `name` (e.g., `redshift@>=3.5,<4.0`).
    /// Processed by the solver to find compatible versions.
    #[pyo3(get, set)]
    pub reqs: Vec<String>,

    /// Resolved dependencies (full Package objects).
    /// Populated by the solver after successful resolution.
    /// 
    /// NOTE: These are intentionally cloned (owned) copies from Storage.
    /// This makes Package self-contained after solving - it doesn't need
    /// Storage reference to access dependency envs/apps via _env()/_app().
    #[pyo3(get)]
    pub deps: Vec<Package>,

    /// Package tags for categorization and filtering.
    /// Common tags: "dcc", "render", "adobe", "autodesk", "vfx", etc.
    #[pyo3(get, set)]
    pub tags: Vec<String>,

    /// Path to package icon (relative to package root or absolute).
    #[pyo3(get, set)]
    pub icon: Option<String>,

    /// Status of dependency resolution.
    #[pyo3(get)]
    #[serde(default)]
    pub solve_status: SolveStatus,

    /// Error message if solve failed.
    #[pyo3(get)]
    #[serde(default)]
    pub solve_error: Option<String>,

    /// Path to the source file (package.py or .toml for toolsets).
    /// Set by loader/storage during package discovery.
    #[pyo3(get, set)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_source: Option<String>,
}

#[pymethods]
impl Package {
    /// Create a new Package.
    ///
    /// # Arguments
    /// * `base` - Package base name (e.g., "maya")
    /// * `version` - Version string (e.g., "2026.1.0")
    ///
    /// # Example
    /// ```python
    /// pkg = Package("maya", "2026.1.0")
    /// assert pkg.name == "maya-2026.1.0"
    /// ```
    #[new]
    pub fn new(base: String, version: String) -> Self {
        let name = format!("{}-{}", base, version);
        Self {
            name,
            base,
            version,
            envs: Vec::new(),
            apps: Vec::new(),
            reqs: Vec::new(),
            deps: Vec::new(),
            tags: Vec::new(),
            icon: None,
            solve_status: SolveStatus::NotSolved,
            solve_error: None,
            package_source: None,
        }
    }

    /// Set the version and update the name.
    ///
    /// This setter maintains consistency between name and version.
    #[setter]
    pub fn set_version(&mut self, version: String) {
        self.version = version;
        self.name = format!("{}-{}", self.base, self.version);
    }

    /// Add an environment to the package.
    pub fn add_env(&mut self, env: Env) {
        self.envs.push(env);
    }

    /// Add an application to the package.
    pub fn add_app(&mut self, app: App) {
        self.apps.push(app);
    }

    /// Add a requirement (dependency constraint).
    ///
    /// # Arguments
    /// * `req` - Requirement string (e.g., "redshift@>=3.5,<4.0")
    pub fn add_req(&mut self, req: String) {
        self.reqs.push(req);
    }

    /// Add a tag to the package.
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Check if package has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get environment(s).
    ///
    /// - `name=None`: returns all envs (`Vec<Env>`)
    /// - `name=Some(x)`: returns single env or None
    ///
    /// By default includes deps envs (merged). Set deps=false for own only.
    #[pyo3(signature = (name = None, deps = true))]
    pub fn env(&self, py: Python<'_>, name: Option<&str>, deps: bool) -> PyResult<Py<PyAny>> {
        match name {
            None => {
                // Return all envs
                let result = self.all_envs(deps);
                Ok(result.into_pyobject(py)?.into_any().unbind())
            }
            Some(n) => {
                // Return single env or None
                let env = self._env(n, deps);
                Ok(env.into_pyobject(py)?.into_any().unbind())
            }
        }
    }

    /// Get all envs. By default includes deps.
    #[pyo3(signature = (deps = true))]
    pub fn all_envs(&self, deps: bool) -> Vec<Env> {
        let mut result = self.envs.clone();
        if deps {
            for dep in &self.deps {
                for env in &dep.envs {
                    if !result.iter().any(|e| e.name == env.name) {
                        result.push(env.clone());
                    }
                }
            }
        }
        result
    }

    /// Get all tags. By default includes deps.
    #[pyo3(signature = (deps = true))]
    pub fn all_tags(&self, deps: bool) -> Vec<String> {
        let mut result: Vec<String> = self.tags.clone();
        if deps {
            for dep in &self.deps {
                for tag in &dep.tags {
                    if !result.contains(tag) {
                        result.push(tag.clone());
                    }
                }
            }
        }
        result
    }

    /// Get application(s).
    ///
    /// - `name=None`: returns all apps (`Vec<App>`)
    /// - `name=Some(x)`: returns single app or None
    ///
    /// By default searches in deps too. Set deps=false for own apps only.
    #[pyo3(signature = (name = None, deps = true))]
    pub fn app(&self, py: Python<'_>, name: Option<&str>, deps: bool) -> PyResult<Py<PyAny>> {
        match name {
            None => {
                // Return all apps
                let result = self.all_apps(deps);
                Ok(result.into_pyobject(py)?.into_any().unbind())
            }
            Some(n) => {
                // Return single app or None
                let app = self._app(n, deps);
                Ok(app.into_pyobject(py)?.into_any().unbind())
            }
        }
    }

    /// Get all apps. By default includes deps.
    #[pyo3(signature = (deps = true))]
    pub fn all_apps(&self, deps: bool) -> Vec<App> {
        let mut result = self.apps.clone();
        if deps {
            for dep in &self.deps {
                result.extend(dep.apps.clone());
            }
        }
        result
    }

    /// Check if package has a specific requirement.
    ///
    /// Checks if any requirement starts with the given base name.
    pub fn has_req(&self, base_name: &str) -> bool {
        self.reqs.iter().any(|r| {
            r.starts_with(base_name)
                && (r.len() == base_name.len()
                    || r.chars().nth(base_name.len()) == Some('@'))
        })
    }

    /// Get the default environment.
    ///
    /// Returns the env named "default", or the first env if no default exists,
    /// or None if there are no environments.
    pub fn default_env(&self) -> Option<Env> {
        self._env("default", true)
            .or_else(|| self.envs.first().cloned())
    }

    /// Get the default application.
    ///
    /// Returns the app with the same name as package base, or the first app,
    /// or None if there are no applications.
    pub fn default_app(&self) -> Option<App> {
        self._app(&self.base, true)
            .or_else(|| self.apps.first().cloned())
    }

    /// Get all app names.
    pub fn app_names(&self) -> Vec<String> {
        self.apps.iter().map(|a| a.name.clone()).collect()
    }

    /// Get all env names.
    pub fn env_names(&self) -> Vec<String> {
        self.envs.iter().map(|e| e.name.clone()).collect()
    }

    /// Get effective environment for an app.
    ///
    /// Looks up the app by name, finds its env_name, and returns
    /// the corresponding solved environment.
    ///
    /// # Arguments
    /// * `app_name` - Name of the app (uses default app if None)
    #[pyo3(signature = (app_name = None))]
    pub fn effective_env(&self, app_name: Option<&str>) -> PyResult<Option<Env>> {
        // Get app
        let app = match app_name {
            Some(name) => self._app(name, true),
            None => self.default_app(),
        };

        let Some(app) = app else {
            return Ok(None);
        };

        // Get env name from app
        let env_name = app.env_name.as_deref().unwrap_or("default");

        // _env with deps=true already returns solved env
        Ok(self._env(env_name, true).or_else(|| self.default_env()))
    }

    /// Parse version as SemVer.
    ///
    /// Returns error if version is not valid SemVer.
    pub fn semver(&self) -> PyResult<String> {
        // Just validate, return as string for Python
        use crate::error::IntoPyErr;
        Ok(Version::parse(&self.version).py_err()?.to_string())
    }

    /// Check if this package satisfies a version constraint.
    ///
    /// # Arguments
    /// * `constraint` - Version requirement (e.g., ">=2026.0.0,<2027.0.0")
    pub fn satisfies(&self, constraint: &str) -> PyResult<bool> {
        use semver::VersionReq;

        use crate::error::IntoPyErr;
        let version = Version::parse(&self.version).py_err()?;
        let req = VersionReq::parse(constraint).py_err()?;

        Ok(req.matches(&version))
    }

    /// Convert to dictionary.
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);

        dict.set_item("name", &self.name)?;
        dict.set_item("base", &self.base)?;
        dict.set_item("version", &self.version)?;

        // Envs
        let envs_list = PyList::empty(py);
        for env in &self.envs {
            envs_list.append(env.to_dict(py)?)?;
        }
        dict.set_item("envs", envs_list)?;

        // Apps
        let apps_list = PyList::empty(py);
        for app in &self.apps {
            apps_list.append(app.to_dict(py)?)?;
        }
        dict.set_item("apps", apps_list)?;

        // Reqs and deps (deps as names for serialization)
        dict.set_item("reqs", PyList::new(py, &self.reqs)?)?;
        let dep_names: Vec<&str> = self.deps.iter().map(|d| d.name.as_str()).collect();
        dict.set_item("deps", PyList::new(py, &dep_names)?)?;

        // Tags and icon
        dict.set_item("tags", PyList::new(py, &self.tags)?)?;
        dict.set_item("icon", &self.icon)?;

        Ok(dict.into())
    }

    /// Create from dictionary.
    #[staticmethod]
    pub fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let base: String = dict
            .get_item("base")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'base'"))?
            .extract()?;

        let version: String = dict
            .get_item("version")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'version'"))?
            .extract()?;

        let mut pkg = Package::new(base, version);

        // Envs
        if let Some(envs_obj) = dict.get_item("envs")? {
            let envs_list: Vec<Bound<'_, PyDict>> = envs_obj.extract()?;
            for env_dict in envs_list {
                pkg.add_env(Env::from_dict(&env_dict)?);
            }
        }

        // Apps
        if let Some(apps_obj) = dict.get_item("apps")? {
            let apps_list: Vec<Bound<'_, PyDict>> = apps_obj.extract()?;
            for app_dict in apps_list {
                pkg.add_app(App::from_dict(&app_dict)?);
            }
        }

        // Reqs
        if let Some(reqs_obj) = dict.get_item("reqs")? {
            let reqs: Vec<String> = reqs_obj.extract()?;
            pkg.reqs = reqs;
        }

        // Deps - skip, they're populated by solve()
        // (from_dict doesn't restore full Package deps)

        // Tags
        if let Some(tags_obj) = dict.get_item("tags")? {
            let tags: Vec<String> = tags_obj.extract()?;
            pkg.tags = tags;
        }

        // Icon
        if let Some(icon_obj) = dict.get_item("icon")? {
            pkg.icon = icon_obj.extract().ok();
        }

        Ok(pkg)
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> PyResult<String> {
        use crate::error::IntoPyErr;
        serde_json::to_string(self).py_err()
    }

    /// Serialize to pretty JSON string.
    pub fn to_json_pretty(&self) -> PyResult<String> {
        use crate::error::IntoPyErr;
        serde_json::to_string_pretty(self).py_err()
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
            "Package({:?}, {:?}, {} envs, {} apps, {} reqs)",
            self.base,
            self.version,
            self.envs.len(),
            self.apps.len(),
            self.reqs.len()
        )
    }

    /// Hash based on name
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

    /// Resolve versions only - fills deps with unsolved packages.
    ///
    /// Uses PubGrub to resolve reqs into concrete versions.
    /// Deps will contain package clones but not recursively solved.
    pub fn solve_version(&mut self, available: Vec<Package>) -> PyResult<()> {
        self.solve_version_impl(&available)
    }

    /// Recursively solve all deps (must call solve_version first).
    ///
    /// Topological sort deps (leaves first), solve each recursively.
    pub fn solve_deps(&mut self, available: Vec<Package>) -> PyResult<()> {
        self.solve_deps_impl(&available)
    }

    /// Full solve: resolve versions + recursively solve deps.
    pub fn solve(&mut self, available: Vec<Package>) -> PyResult<()> {
        self.solve_version_impl(&available)?;
        self.solve_deps_impl(&available)?;
        Ok(())
    }
}

// Pure Rust impl with references
impl Package {
    /// Resolve versions (Rust API with slice).
    pub fn solve_version_impl(&mut self, available: &[Package]) -> PyResult<()> {
        use crate::solver::Solver;

        // If no reqs, nothing to solve
        if self.reqs.is_empty() {
            self.deps.clear();
            self.solve_status = SolveStatus::Solved;
            self.solve_error = None;
            return Ok(());
        }

        // Create solver
        let solver = match Solver::from_packages(available) {
            Ok(s) => s,
            Err(e) => {
                self.solve_status = SolveStatus::Failed;
                self.solve_error = Some(e.to_string());
                return Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string()));
            }
        };

        // Solve requirements
        match solver.solve_reqs(self.reqs.clone()) {
            Ok(solution) => {
                // Clone packages into deps - intentional ownership transfer
                // Makes Package self-contained, independent from Storage
                self.deps = solution
                    .iter()
                    .filter(|name| *name != &self.name)
                    .filter_map(|name| available.iter().find(|p| &p.name == name).cloned())
                    .collect();
                self.solve_status = SolveStatus::Solved;
                self.solve_error = None;
                Ok(())
            }
            Err(e) => {
                self.solve_status = SolveStatus::Failed;
                self.solve_error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Recursively solve all deps (Rust API with slice).
    /// 
    /// Cloning strategy: We clone packages intentionally to make each Package
    /// self-contained after solving. This allows accessing dep envs/apps without
    /// keeping Storage reference alive.
    pub fn solve_deps_impl(&mut self, available: &[Package]) -> PyResult<()> {
        if self.deps.is_empty() {
            return Ok(());
        }

        // Topological sort: packages with no deps first
        let mut sorted = Vec::new();
        // Clone deps for sorting - we'll replace them with solved versions later
        let mut remaining: Vec<Package> = self.deps.clone();
        
        while !remaining.is_empty() {
            // Find packages whose reqs are all satisfied by sorted
            let sorted_names: std::collections::HashSet<&str> = 
                sorted.iter().map(|p: &Package| p.name.as_str()).collect();
            
            let (ready, not_ready): (Vec<_>, Vec<_>) = remaining.into_iter().partition(|pkg| {
                pkg.reqs.iter().all(|req| {
                    // Check if req is satisfied by any sorted package
                    let base = req.split('@').next().unwrap_or(req);
                    sorted_names.iter().any(|n| n.starts_with(base))
                        || sorted_names.is_empty() && pkg.reqs.is_empty()
                }) || pkg.reqs.is_empty()
            });
            
            if ready.is_empty() && !not_ready.is_empty() {
                // No progress - just add remaining in order
                sorted.extend(not_ready);
                break;
            }
            
            sorted.extend(ready);
            remaining = not_ready;
        }

        // Solve each in order, building solved map
        let mut solved_map: std::collections::HashMap<String, Package> = 
            std::collections::HashMap::new();
        
        // Build available once, extend as we solve (avoids O(n*m) cloning)
        let mut pkg_available: Vec<Package> = available.to_vec();
        
        for mut pkg in sorted {
            // Solve this package against current available
            pkg.solve_version_impl(&pkg_available)?;
            pkg.solve_deps_impl(&pkg_available)?;
            
            // Add solved package to available for next iterations
            pkg_available.push(pkg.clone());
            solved_map.insert(pkg.name.clone(), pkg);
        }

        // Replace deps with solved versions (clone to own them)
        self.deps = self.deps
            .iter()
            .filter_map(|d| solved_map.get(&d.name).cloned())
            .collect();

        Ok(())
    }



    /// Check if dependencies are solved.
    ///
    /// Returns true if solve_status is Solved, or if no reqs exist.
    pub fn is_solved(&self) -> bool {
        self.solve_status == SolveStatus::Solved || self.reqs.is_empty()
    }

    /// Get detailed solve status.
    pub fn status(&self) -> SolveStatus {
        if self.reqs.is_empty() {
            SolveStatus::Solved
        } else {
            self.solve_status
        }
    }
}

// Pure Rust API
impl Package {
    /// Parse a package name into base and version.
    ///
    /// # Arguments
    /// * `name` - Full package name (e.g., "maya-2026.1.0")
    ///
    /// # Returns
    /// Tuple of (base, version) or error if invalid format.
    ///
    /// # Example
    /// ```ignore
    /// let (base, version) = Package::parse_name("maya-2026.1.0")?;
    /// assert_eq!(base, "maya");
    /// assert_eq!(version, "2026.1.0");
    /// ```
    pub fn parse_name(name: &str) -> Result<(String, String), PackageError> {
        let pkg_id = Self::parse_id(name)?;

        // Get version string (required for this function)
        let version_str = pkg_id.version().ok_or_else(|| PackageError::InvalidName {
            name: name.to_string(),
            reason: "Missing version".to_string(),
        })?;

        // Return version with variant if present
        let version = match pkg_id.variant {
            Some(v) => format!("{}-{}", version_str, v),
            None => version_str,
        };
        Ok((pkg_id.name, version))
    }

    /// Parse package ID string into components.
    ///
    /// # Example
    /// ```ignore
    /// let id = Package::parse_id("maya-2026.1.0-win64")?;
    /// assert_eq!(id.name, "maya");
    /// assert_eq!(id.version(), Some("2026.1.0".to_string()));
    /// assert_eq!(id.variant, Some("win64".to_string()));
    /// ```
    pub fn parse_id(name: &str) -> Result<crate::name::PackageId, PackageError> {
        use crate::name::PackageId;

        let pkg_id = PackageId::parse(name).ok_or_else(|| PackageError::InvalidName {
            name: name.to_string(),
            reason: "Invalid package ID format".to_string(),
        })?;

        // Validate version is valid semver (if present)
        if let Some(version_str) = pkg_id.version() {
            Version::parse(&version_str).map_err(|e| PackageError::InvalidVersion {
                version: version_str,
                reason: e.to_string(),
            })?;
        }

        Ok(pkg_id)
    }

    /// Create package from full name.
    ///
    /// Parses "maya-2026.1.0" into base and version.
    pub fn from_name(name: &str) -> Result<Self, PackageError> {
        let (base, version) = Self::parse_name(name)?;
        Ok(Self::new(base, version))
    }

    /// Get parsed SemVer version.
    pub fn parsed_version(&self) -> Result<Version, PackageError> {
        Version::parse(&self.version).map_err(|e| PackageError::InvalidVersion {
            version: self.version.clone(),
            reason: e.to_string(),
        })
    }

    /// Compare versions with another package of the same base.
    ///
    /// Returns ordering based on SemVer rules.
    pub fn version_cmp(&self, other: &Self) -> Result<std::cmp::Ordering, PackageError> {
        let v1 = self.parsed_version()?;
        let v2 = other.parsed_version()?;
        Ok(v1.cmp(&v2))
    }

    /// Check if this package is newer than another.
    pub fn is_newer_than(&self, other: &Self) -> Result<bool, PackageError> {
        Ok(self.version_cmp(other)? == std::cmp::Ordering::Greater)
    }

    /// Get env by name (internal Rust API).
    ///
    /// Tokens are always expanded. When deps=true, merges envs from dependencies first.
    /// For toolsets (packages without own envs), returns merged env from dependencies.
    pub fn _env(&self, name: &str, deps: bool) -> Option<Env> {
        use crate::env::Env;
        use log::debug;
        
        let own = self.envs.iter().find(|e| e.name == name).cloned();
        
        // Collect deps envs if requested
        // NOTE: After solve(), deps is a FLAT list of all resolved packages (direct + transitive).
        // We use deps=false for recursive calls because we only need each package's own env,
        // not their deps (which are already in our flat deps list).
        //
        // Order strategy for PATH: direct reqs first (in request order), then transitive deps.
        // Since insert prepends, we iterate: transitive first, then direct in reverse request order.
        let deps_env = if deps && !self.deps.is_empty() {
            // Build ordered list: direct reqs in request order, then transitive
            let req_bases: Vec<&str> = self.reqs.iter()
                .map(|r| r.split('@').next().unwrap_or(r).split('-').next().unwrap_or(r))
                .collect();
            
            // Find direct deps in request order
            let mut direct: Vec<&Package> = Vec::new();
            for base in &req_bases {
                if let Some(dep) = self.deps.iter().find(|d| &d.base.as_str() == base) {
                    direct.push(dep);
                }
            }
            
            // Transitive = all deps not in direct
            let direct_set: std::collections::HashSet<&str> = direct.iter().map(|d| d.name.as_str()).collect();
            let transitive: Vec<_> = self.deps.iter().filter(|d| !direct_set.contains(d.name.as_str())).collect();
            
            let mut merged: Option<Env> = None;
            // Transitive first (will end up last in PATH due to insert prepend)
            for dep in transitive.iter().rev() {
                if let Some(dep_env) = dep._env(name, false) {
                    merged = Some(match merged {
                        Some(m) => m.merge(&dep_env),
                        None => dep_env,
                    });
                }
            }
            // Direct reqs last in reverse order (first req will be first in PATH)
            for dep in direct.iter().rev() {
                if let Some(dep_env) = dep._env(name, false) {
                    merged = Some(match merged {
                        Some(m) => m.merge(&dep_env),
                        None => dep_env,
                    });
                }
            }
            merged
        } else {
            None
        };
        
        // Build result: own + deps, or just deps for toolsets
        // ALWAYS compress to merge same-name evars (e.g. PATH inserts)
        let result = match (own, deps_env) {
            (Some(o), Some(d)) => o.merge(&d).compress(),
            (Some(o), None) => o.compress(),
            (None, Some(d)) => d.compress(), // Toolset case: must compress deps!
            (None, None) => return None,
        };
        
        // ALWAYS expand tokens
        match result.solve_impl(10, true) {
            Ok(solved) => {
                debug!("Package::_env solved {} evars for {}", solved.evars.len(), name);
                Some(solved)
            }
            Err(e) => {
                log::warn!("Package::_env failed to solve tokens: {}", e);
                Some(result)
            }
        }
    }

    /// Get app by name (internal Rust API).
    pub fn _app(&self, name: &str, deps: bool) -> Option<App> {
        // Search in own apps first
        if let Some(app) = self.apps.iter().find(|a| a.name == name).cloned() {
            return Some(app);
        }
        
        // Search in deps if requested
        if deps {
            for dep in &self.deps {
                if let Some(app) = dep.apps.iter().find(|a| a.name == name).cloned() {
                    return Some(app);
                }
            }
        }
        
        None
    }

    /// Create a merged environment from all package envs.
    ///
    /// Merges all envs in order, then compresses the result.
    pub fn merged_env(&self) -> Env {
        if self.envs.is_empty() {
            return Env::new("merged".to_string());
        }

        let mut result = self.envs[0].clone();
        result.name = "merged".to_string();

        for env in &self.envs[1..] {
            result = result.merge(env);
        }

        result.compress()
    }

    /// Generate PKG_* environment variables for this package.
    ///
    /// Creates variables:
    /// - PKG_{BASE}_ROOT    - package root path (from first env's ROOT-like var or empty)
    /// - PKG_{BASE}_VERSION - full version string
    /// - PKG_{BASE}_MAJOR   - major version component
    /// - PKG_{BASE}_MINOR   - minor version component  
    /// - PKG_{BASE}_PATCH   - patch version component
    /// - PKG_{BASE}_VARIANT - prerelease/build metadata (if any)
    ///
    /// Where {BASE} is uppercase base name with dashes replaced by underscores.
    pub fn stamp(&self) -> Vec<crate::evar::Evar> {
        use crate::evar::Evar;
        use semver::Version;

        let mut result = Vec::new();
        
        // Normalize base name: uppercase, dashes -> underscores
        let prefix = format!("PKG_{}", self.base.to_uppercase().replace('-', "_"));
        
        // Try to find ROOT from package's env
        let root = self.envs.iter()
            .flat_map(|e| e.evars.iter())
            .find(|ev| {
                let name_upper = ev.name.to_uppercase();
                name_upper.ends_with("_ROOT") || name_upper == "ROOT"
            })
            .map(|ev| ev.value.clone())
            .unwrap_or_default();
        
        result.push(Evar::set(format!("{}_ROOT", prefix), root));
        result.push(Evar::set(format!("{}_VERSION", prefix), self.version.clone()));
        
        // Parse version components
        if let Ok(ver) = Version::parse(&self.version) {
            result.push(Evar::set(format!("{}_MAJOR", prefix), ver.major.to_string()));
            result.push(Evar::set(format!("{}_MINOR", prefix), ver.minor.to_string()));
            result.push(Evar::set(format!("{}_PATCH", prefix), ver.patch.to_string()));
            
            // Variant: prerelease or build metadata
            let variant = if !ver.pre.is_empty() {
                ver.pre.to_string()
            } else if !ver.build.is_empty() {
                ver.build.to_string()
            } else {
                String::new()
            };
            result.push(Evar::set(format!("{}_VARIANT", prefix), variant));
        } else {
            // Fallback: try simple split on dots
            let parts: Vec<&str> = self.version.split('.').collect();
            result.push(Evar::set(format!("{}_MAJOR", prefix), parts.get(0).unwrap_or(&"").to_string()));
            result.push(Evar::set(format!("{}_MINOR", prefix), parts.get(1).unwrap_or(&"").to_string()));
            result.push(Evar::set(format!("{}_PATCH", prefix), parts.get(2).unwrap_or(&"").to_string()));
            result.push(Evar::set(format!("{}_VARIANT", prefix), String::new()));
        }
        
        result
    }
}

impl Default for Package {
    fn default() -> Self {
        Self::new("unnamed".to_string(), "0.0.0".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evar::Evar;

    #[test]
    fn package_new() {
        let pkg = Package::new("maya".to_string(), "2026.1.0".to_string());
        assert_eq!(pkg.name, "maya-2026.1.0");
        assert_eq!(pkg.base, "maya");
        assert_eq!(pkg.version, "2026.1.0");
    }

    #[test]
    fn package_parse_name() {
        // Simple case
        let (base, ver) = Package::parse_name("maya-2026.1.0").unwrap();
        assert_eq!(base, "maya");
        assert_eq!(ver, "2026.1.0");

        // Dash in base name
        let (base2, ver2) = Package::parse_name("my-plugin-1.0.0").unwrap();
        assert_eq!(base2, "my-plugin");
        assert_eq!(ver2, "1.0.0");

        // Invalid: no version
        assert!(Package::parse_name("maya").is_err());

        // Invalid: bad version
        assert!(Package::parse_name("maya-notaversion").is_err());
    }

    #[test]
    fn package_from_name() {
        let pkg = Package::from_name("houdini-20.0.0").unwrap();
        assert_eq!(pkg.base, "houdini");
        assert_eq!(pkg.version, "20.0.0");
        assert_eq!(pkg.name, "houdini-20.0.0");
    }

    #[test]
    fn package_reqs() {
        let mut pkg = Package::new("maya".to_string(), "2026.0.0".to_string());
        pkg.add_req("redshift@>=3.5,<4.0".to_string());
        pkg.add_req("ocio@2".to_string());

        assert!(pkg.has_req("redshift"));
        assert!(pkg.has_req("ocio"));
        assert!(!pkg.has_req("unknown"));
    }

    #[test]
    fn package_envs_apps() {
        let mut pkg = Package::new("maya".to_string(), "2026.0.0".to_string());

        // Add env
        let mut env = Env::new("default".to_string());
        env.add(Evar::set("ROOT", "/opt/maya"));
        pkg.add_env(env);

        // Add app
        let app = App::named("maya").with_path("/opt/maya/bin/maya");
        pkg.add_app(app);

        assert!(pkg._env("default", true).is_some());
        assert!(pkg._app("maya", true).is_some());
        assert!(pkg.default_env().is_some());
        assert!(pkg.default_app().is_some());
    }

    #[test]
    fn package_version_compare() {
        let pkg1 = Package::new("maya".to_string(), "2025.0.0".to_string());
        let pkg2 = Package::new("maya".to_string(), "2026.0.0".to_string());
        let pkg3 = Package::new("maya".to_string(), "2026.1.0".to_string());

        assert!(pkg2.is_newer_than(&pkg1).unwrap());
        assert!(pkg3.is_newer_than(&pkg2).unwrap());
        assert!(!pkg1.is_newer_than(&pkg2).unwrap());
    }

    #[test]
    fn package_satisfies() {
        let pkg = Package::new("maya".to_string(), "2026.1.0".to_string());

        assert!(pkg.satisfies(">=2026.0.0").unwrap());
        assert!(pkg.satisfies(">=2026.0.0,<2027.0.0").unwrap());
        assert!(!pkg.satisfies("<2026.0.0").unwrap());
        assert!(!pkg.satisfies(">=2027.0.0").unwrap());
    }

    #[test]
    fn package_serialization() {
        let mut pkg = Package::new("maya".to_string(), "2026.0.0".to_string());
        pkg.add_req("redshift@3".to_string());

        let mut env = Env::new("default".to_string());
        env.add(Evar::set("ROOT", "/opt"));
        pkg.add_env(env);

        let json = serde_json::to_string(&pkg).unwrap();
        let pkg2: Package = serde_json::from_str(&json).unwrap();

        assert_eq!(pkg, pkg2);
    }

    #[test]
    fn package_solve() {
        // Create a package with requirements
        let mut pkg = Package::new("myapp".to_string(), "1.0.0".to_string());
        pkg.add_req("maya@>=2026".to_string());
        pkg.add_req("redshift@>=3.5".to_string());

        assert!(!pkg.is_solved());

        // Create available packages
        let available = vec![
            Package::new("maya".to_string(), "2026.0.0".to_string()),
            Package::new("maya".to_string(), "2026.1.0".to_string()),
            Package::new("redshift".to_string(), "3.5.0".to_string()),
            Package::new("redshift".to_string(), "3.6.0".to_string()),
        ];

        // Solve
        pkg.solve(available).unwrap();

        assert!(pkg.is_solved());
        assert!(pkg.deps.iter().any(|d| d.name.starts_with("maya-")));
        assert!(pkg.deps.iter().any(|d| d.name.starts_with("redshift-")));
    }

    #[test]
    fn package_solve_empty_reqs() {
        let mut pkg = Package::new("simple".to_string(), "1.0.0".to_string());
        // No reqs
        assert!(pkg.is_solved());
        
        pkg.solve(vec![]).unwrap();
        assert!(pkg.deps.is_empty());
    }

    #[test]
    fn package_stamp_basic() {
        let pkg = Package::new("maya".to_string(), "2026.1.0".to_string());
        let evars = pkg.stamp();
        
        assert_eq!(evars.len(), 6);
        assert_eq!(evars[0].name, "PKG_MAYA_ROOT");
        assert_eq!(evars[1].name, "PKG_MAYA_VERSION");
        assert_eq!(evars[1].value, "2026.1.0");
        assert_eq!(evars[2].name, "PKG_MAYA_MAJOR");
        assert_eq!(evars[2].value, "2026");
        assert_eq!(evars[3].name, "PKG_MAYA_MINOR");
        assert_eq!(evars[3].value, "1");
        assert_eq!(evars[4].name, "PKG_MAYA_PATCH");
        assert_eq!(evars[4].value, "0");
        assert_eq!(evars[5].name, "PKG_MAYA_VARIANT");
        assert_eq!(evars[5].value, "");
    }

    #[test]
    fn package_stamp_with_dashes() {
        let pkg = Package::new("my-cool-plugin".to_string(), "1.2.3".to_string());
        let evars = pkg.stamp();
        
        assert_eq!(evars[0].name, "PKG_MY_COOL_PLUGIN_ROOT");
        assert_eq!(evars[1].name, "PKG_MY_COOL_PLUGIN_VERSION");
    }

    #[test]
    fn package_stamp_with_variant() {
        let pkg = Package::new("beta-pkg".to_string(), "1.0.0-beta.2".to_string());
        let evars = pkg.stamp();
        
        assert_eq!(evars[5].name, "PKG_BETA_PKG_VARIANT");
        assert_eq!(evars[5].value, "beta.2");
    }

    #[test]
    fn package_stamp_with_root() {
        let mut pkg = Package::new("houdini".to_string(), "20.0.0".to_string());
        let mut env = Env::new("default".to_string());
        env.add(Evar::set("HOUDINI_ROOT", "C:/Program Files/Houdini"));
        pkg.add_env(env);
        
        let evars = pkg.stamp();
        assert_eq!(evars[0].name, "PKG_HOUDINI_ROOT");
        assert_eq!(evars[0].value, "C:/Program Files/Houdini");
    }
}
