//! Dependency specification parsing.
//!
//! This module provides [`DepSpec`] for parsing and representing dependency
//! requirements like `redshift@>=3.5,<4.0` or `maya-2026.1.0`.
//!
//! # Formats Supported
//!
//! ## Requirements (constraints)
//!
//! Used in `Package.reqs` to specify acceptable versions:
//!
//! - `name` - Any version (e.g., `redshift`)
//! - `name@constraint` - Version constraint (e.g., `redshift@>=3.5,<4.0`)
//! - `name==version` - Exact version (e.g., `redshift==3.5.0`)
//! - `name-version` - Rez-style range shorthand (superset, e.g., `redshift-3.5`)
//!
//! Constraint syntax follows Rez version ranges:
//! - `3` - Superset (3.x)
//! - `2+` or `>=2` - Inclusive lower bound
//! - `<4` or `<=4` - Upper bound
//! - `1+<4` or `>=1,<4` - Bounded range
//! - `1..4` - Inclusive bounded range
//! - `3|5+` - Union (OR)
//! - `==2.3.0` - Exact version
//!
//! ## Resolved Dependencies
//!
//! Used in `Package.deps` for concrete solved versions:
//!
//! - `name-version` - Package identifier (e.g., `redshift-3.5.2`)
//!   (In requirements, this represents a version range superset.)
//!
//! # Examples
//!
//! ```ignore
//! use pkg::DepSpec;
//!
//! // Parse requirement
//! let spec = DepSpec::parse("redshift@>=3.5,<4.0")?;
//! assert_eq!(spec.base, "redshift");
//! assert!(spec.matches_version("3.5.2"));
//! assert!(!spec.matches_version("4.0.0"));
//!
//! // Parse exact requirement
//! let resolved = DepSpec::parse("redshift==3.5.2")?;
//! assert_eq!(resolved.base, "redshift");
//! assert_eq!(resolved.exact_version(), Some("3.5.2"));
//! ```
//!
//! # Solver Integration
//!
//! [`DepSpec`] is used by the [`Solver`](crate::solver::Solver) to:
//! 1. Parse package requirements from `Package.reqs`
//! 2. Check if available packages satisfy constraints
//! 3. Build the PubGrub dependency graph

use crate::error::PackageError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::rez_version::{Requirement, Version, VersionRange};

/// Dependency specification.
///
/// Represents either:
/// - A requirement with version constraint (`redshift@>=3.5`)
/// - A resolved dependency with exact version (`redshift-3.5.2`)
///
/// # Parsing Rules
///
/// 1. If contains `@`: Split on `@` → (base, constraint)
/// 2. Otherwise: Parse as Rez requirement (name + optional range)
/// 3. If no range: base only, any version
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepSpec {
    /// Package base name (e.g., "redshift", "maya", "my_plugin").
    #[pyo3(get)]
    pub base: String,

    /// Version constraint string (e.g., ">=3.5,<4.0", "3.5.2", "*").
    /// "*" means any version.
    #[pyo3(get)]
    pub constraint: String,

    /// Original input string for reference.
    #[pyo3(get)]
    pub original: String,
}

#[pymethods]
impl DepSpec {
    /// Create a new DepSpec.
    ///
    /// # Arguments
    /// * `base` - Package base name
    /// * `constraint` - Version constraint (use "*" for any)
    #[new]
    #[pyo3(signature = (base, constraint = None))]
    pub fn new(base: String, constraint: Option<String>) -> Self {
        let constraint = constraint.unwrap_or_else(|| "*".to_string());
        let original = if constraint == "*" {
            base.clone()
        } else {
            format!("{}@{}", base, constraint)
        };

        Self {
            base,
            constraint,
            original,
        }
    }

    /// Parse a dependency specification string.
    ///
    /// Handles multiple formats:
    /// - `name` → any version
    /// - `name@constraint` → version constraint
    /// - `name-version` → exact version (resolved dependency)
    ///
    /// # Arguments
    /// * `spec` - Specification string
    ///
    /// # Returns
    /// Parsed DepSpec or error if invalid format.
    #[staticmethod]
    pub fn parse(spec: &str) -> PyResult<Self> {
        Ok(Self::parse_impl(spec)?)
    }

    /// Check if a version matches this constraint.
    ///
    /// # Arguments
    /// * `version` - Version string to check (e.g., "3.5.2")
    ///
    /// # Returns
    /// True if version satisfies the constraint.
    pub fn matches(&self, version: &str) -> PyResult<bool> {
        Ok(self.matches_impl(version)?)
    }

    /// Check if this is an exact version constraint.
    ///
    /// Returns true if constraint is `==version` (or `=version`).
    pub fn is_exact(&self) -> bool {
        self.constraint.starts_with("==") || self.constraint.starts_with('=')
    }

    /// Get exact version if this is an exact constraint.
    ///
    /// Returns None if this is a range constraint.
    pub fn exact_version(&self) -> Option<String> {
        if self.is_exact() {
            let mut s = self.constraint.as_str();
            if let Some(rest) = s.strip_prefix("==") {
                s = rest;
            } else if let Some(rest) = s.strip_prefix('=') {
                s = rest;
            }
            Some(s.to_string())
        } else {
            None
        }
    }

    /// Check if this accepts any version.
    pub fn is_any(&self) -> bool {
        self.constraint.is_empty() || self.constraint == "*"
    }

    /// Convert to requirement format (`name@constraint`).
    pub fn to_req_str(&self) -> String {
        if self.is_any() {
            self.base.clone()
        } else {
            format!("{}@{}", self.base, self.constraint)
        }
    }

    /// Convert to resolved format (`name-version`).
    ///
    /// Only works if this is an exact version constraint.
    /// Returns None if constraint is a range.
    pub fn to_resolved_str(&self) -> Option<String> {
        if let Some(ver) = self.exact_version() {
            Some(format!("{}-{}", self.base, ver))
        } else {
            None
        }
    }

    /// Create a resolved DepSpec from base and exact version.
    #[staticmethod]
    pub fn resolved(base: String, version: String) -> PyResult<Self> {
        // Validate version
        use crate::error::IntoPyErr;
        Version::parse(&version).py_err()?;

        Ok(Self {
            original: format!("{}-{}", base, version),
            base,
            constraint: format!("=={}", version),
        })
    }

    fn __repr__(&self) -> String {
        format!("DepSpec({:?}, {:?})", self.base, self.constraint)
    }

    fn __str__(&self) -> String {
        self.original.clone()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.base.hash(&mut hasher);
        self.constraint.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.base == other.base && self.constraint == other.constraint
    }
}

// Pure Rust API
impl DepSpec {
    /// Internal parse implementation.
    pub fn parse_impl(spec: &str) -> Result<Self, PackageError> {
        let spec = spec.trim();

        if spec.is_empty() {
            return Err(PackageError::InvalidName {
                name: spec.to_string(),
                reason: "Empty dependency spec".to_string(),
            });
        }

        // Format 1: name@constraint (explicit requirement)
        if let Some(at_pos) = spec.find('@') {
            let base = spec[..at_pos].to_string();
            let mut constraint = spec[at_pos + 1..].to_string();

            if base.is_empty() {
                return Err(PackageError::InvalidName {
                    name: spec.to_string(),
                    reason: "Empty base name".to_string(),
                });
            }

            if constraint.is_empty() {
                constraint = "*".to_string();
            }

            // Validate constraint
            Self::validate_constraint(&constraint)?;

            return Ok(Self {
                base,
                constraint,
                original: spec.to_string(),
            });
        }

        // Rez-style requirement (name-version or name<range)
        if spec.chars().any(|c| matches!(c, '-' | '#' | '<' | '>' | '=' | '!' | '~')) {
            let req = Requirement::parse(spec).map_err(|e| PackageError::InvalidVersion {
                version: spec.to_string(),
                reason: e.to_string(),
            })?;

            if req.conflict {
                return Err(PackageError::InvalidVersion {
                    version: spec.to_string(),
                    reason: "Conflict/weak requirements are not supported yet".to_string(),
                });
            }

            let constraint = match req.range {
                None => "*".to_string(),
                Some(range) => range.to_string(),
            };

            return Ok(Self {
                base: req.name,
                constraint,
                original: spec.to_string(),
            });
        }

        // Format 3: just name (any version)
        Ok(Self {
            base: spec.to_string(),
            constraint: "*".to_string(),
            original: spec.to_string(),
        })
    }

    /// Validate a version constraint string.
    fn validate_constraint(constraint: &str) -> Result<(), PackageError> {
        if constraint.is_empty() || constraint == "*" {
            return Ok(());
        }

        VersionRange::parse(constraint).map_err(|e| PackageError::InvalidVersion {
            version: constraint.to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Check if version matches (internal implementation).
    pub fn matches_impl(&self, version: &str) -> Result<bool, PackageError> {
        let ver = Version::parse(version).map_err(|e| PackageError::InvalidVersion {
            version: version.to_string(),
            reason: e.to_string(),
        })?;

        if self.is_any() {
            return Ok(true);
        }

        let range = self.version_range()?;
        Ok(range.contains_version(&ver))
    }

    /// Get parsed VersionRange for solver integration.
    pub fn version_range(&self) -> Result<VersionRange, PackageError> {
        if self.is_any() {
            return VersionRange::parse("").map_err(|e| PackageError::InvalidVersion {
                version: self.constraint.clone(),
                reason: e.to_string(),
            });
        }

        VersionRange::parse(&self.constraint).map_err(|e| PackageError::InvalidVersion {
            version: self.constraint.clone(),
            reason: e.to_string(),
        })
    }
}

impl fmt::Display for DepSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl Default for DepSpec {
    fn default() -> Self {
        Self::new("unnamed".to_string(), Some("*".to_string()))
    }
}

/// Package requirement (rez-next–style interface).
///
/// Typed requirement with name, optional version spec, and weak flag.
/// Parses strings like `python-3.9` or `maya>=2023`; delegates to [`DepSpec`] internally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRequirement {
    /// Package base name.
    pub name: String,
    /// Version constraint (e.g. `>=3.5,<4.0`); `None` = any version.
    pub version_spec: Option<String>,
    /// Weak (optional) requirement.
    pub weak: bool,
}

impl PackageRequirement {
    /// Create a requirement with no version constraint.
    pub fn new(name: String) -> Self {
        Self {
            name,
            version_spec: None,
            weak: false,
        }
    }

    /// Create a requirement with version constraint.
    pub fn with_version(name: String, version_spec: String) -> Self {
        Self {
            name,
            version_spec: Some(version_spec),
            weak: false,
        }
    }

    /// Parse a requirement string (e.g. `python-3.9`, `maya@>=2023`, `redshift`).
    pub fn parse(spec: &str) -> Result<Self, PackageError> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(PackageError::InvalidName {
                name: spec.to_string(),
                reason: "Empty requirement".to_string(),
            });
        }
        let dep = DepSpec::parse_impl(spec)?;
        let version_spec = if dep.is_any() {
            None
        } else {
            Some(dep.constraint)
        };
        Ok(Self {
            name: dep.base,
            version_spec,
            weak: false,
        })
    }

    /// Check if a version string satisfies this requirement.
    pub fn satisfied_by(&self, version: &str) -> Result<bool, PackageError> {
        let constraint = self
            .version_spec
            .as_deref()
            .unwrap_or("*");
        let dep = DepSpec::new(self.name.clone(), Some(constraint.to_string()));
        dep.matches_impl(version)
    }

    /// Requirement string for display/serialization (`name` or `name@constraint`).
    pub fn to_string(&self) -> String {
        match &self.version_spec {
            Some(s) if !s.is_empty() && s != "*" => format!("{}@{}", self.name, s),
            _ => self.name.clone(),
        }
    }
}

impl fmt::Display for PackageRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Parse multiple dependency specs from a list of strings.
///
/// # Arguments
/// * `specs` - List of spec strings
///
/// # Returns
/// Vector of parsed DepSpecs or first error encountered.
pub fn parse_deps(specs: &[String]) -> Result<Vec<DepSpec>, PackageError> {
    specs.iter().map(|s| DepSpec::parse_impl(s)).collect()
}

/// Filter packages by a dependency spec.
///
/// Given a list of package names (base-version format),
/// returns those matching the spec.
pub fn filter_by_spec<'a>(
    spec: &DepSpec,
    packages: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<&'a str>, PackageError> {
    let mut matches = Vec::new();

    for pkg in packages {
        // Parse package name
        if let Ok((base, version)) = crate::Package::parse_name(pkg) {
            if base != spec.base {
                continue;
            }
            if spec.matches_impl(&version)? {
                matches.push(pkg);
            }
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depspec_parse_requirement() {
        // With constraint
        let spec = DepSpec::parse_impl("redshift@>=3.5,<4.0").unwrap();
        assert_eq!(spec.base, "redshift");
        assert_eq!(spec.constraint, ">=3.5,<4.0");
        assert!(!spec.is_exact());
        assert!(!spec.is_any());

        // Exact version via ==
        let spec2 = DepSpec::parse_impl("ocio==2.3.0").unwrap();
        assert_eq!(spec2.base, "ocio");
        assert_eq!(spec2.constraint, "==2.3.0");
        assert!(spec2.is_exact());
    }

    #[test]
    fn depspec_parse_resolved() {
        let spec = DepSpec::parse_impl("redshift-3.5.2").unwrap();
        assert_eq!(spec.base, "redshift");
        assert_eq!(spec.constraint, "3.5.2");
        assert!(!spec.is_exact());
        assert_eq!(spec.exact_version(), None);
    }

    #[test]
    fn depspec_parse_any() {
        let spec = DepSpec::parse_impl("redshift").unwrap();
        assert_eq!(spec.base, "redshift");
        assert_eq!(spec.constraint, "*");
        assert!(spec.is_any());
    }

    #[test]
    fn depspec_parse_dash_separator() {
        // Dash separator indicates a version range in Rez syntax
        let spec = DepSpec::parse_impl("myplugin-1.0.0").unwrap();
        assert_eq!(spec.base, "myplugin");
        assert_eq!(spec.constraint, "1.0.0");
        assert!(!spec.is_exact());

        // Hyphenated package name should still split at hyphen-digit
        let spec2 = DepSpec::parse_impl("my-plugin-1.0.0").unwrap();
        assert_eq!(spec2.base, "my-plugin");
        assert_eq!(spec2.constraint, "1.0.0");
    }

    #[test]
    fn depspec_matches() {
        // Range constraint
        let spec = DepSpec::parse_impl("redshift@>=3.5,<4.0").unwrap();
        assert!(spec.matches_impl("3.5.0").unwrap());
        assert!(spec.matches_impl("3.5.2").unwrap());
        assert!(spec.matches_impl("3.9.9").unwrap());
        assert!(!spec.matches_impl("3.4.9").unwrap());
        assert!(!spec.matches_impl("4.0.0").unwrap());
        assert!(!spec.matches_impl("4.1.0").unwrap());

        // Exact version
        let exact = DepSpec::parse_impl("ocio==2.3.0").unwrap();
        assert!(exact.matches_impl("2.3.0").unwrap());
        assert!(!exact.matches_impl("2.3.1").unwrap());
        assert!(!exact.matches_impl("2.2.0").unwrap());

        // Any version
        let any = DepSpec::parse_impl("python").unwrap();
        assert!(any.matches_impl("3.11.0").unwrap());
        assert!(any.matches_impl("2.7.0").unwrap());
    }

    #[test]
    fn depspec_to_formats() {
        let req = DepSpec::new("redshift".to_string(), Some(">=3.5".to_string()));
        assert_eq!(req.to_req_str(), "redshift@>=3.5");
        assert!(req.to_resolved_str().is_none());

        let exact = DepSpec::resolved("redshift".to_string(), "3.5.2".to_string()).unwrap();
        assert_eq!(exact.to_resolved_str(), Some("redshift-3.5.2".to_string()));
    }

    #[test]
    fn filter_packages() {
        let packages = vec![
            "redshift-3.5.0",
            "redshift-3.5.2",
            "redshift-3.9.0",
            "redshift-4.0.0",
            "maya-2026.0.0",
        ];

        let spec = DepSpec::parse_impl("redshift@>=3.5,<4.0").unwrap();
        let matches: Vec<&str> = filter_by_spec(&spec, packages.iter().map(|s| *s)).unwrap();

        assert_eq!(matches.len(), 3);
        assert!(matches.contains(&"redshift-3.5.0"));
        assert!(matches.contains(&"redshift-3.5.2"));
        assert!(matches.contains(&"redshift-3.9.0"));
        assert!(!matches.contains(&"redshift-4.0.0"));
    }

    #[test]
    fn depspec_invalid() {
        // Empty
        assert!(DepSpec::parse_impl("").is_err());

        // Invalid constraint
        assert!(DepSpec::parse_impl("pkg@1..").is_err());

        // Empty base
        assert!(DepSpec::parse_impl("@1.0.0").is_err());
    }

    #[test]
    fn package_requirement_parse() {
        let r = PackageRequirement::parse("python-3.9").unwrap();
        assert_eq!(r.name, "python");
        assert_eq!(r.version_spec.as_deref(), Some("3.9"));
        assert!(!r.weak);

        let r2 = PackageRequirement::parse("maya@>=2023").unwrap();
        assert_eq!(r2.name, "maya");
        assert_eq!(r2.version_spec.as_deref(), Some(">=2023"));

        let r3 = PackageRequirement::parse("redshift").unwrap();
        assert_eq!(r3.name, "redshift");
        assert!(r3.version_spec.is_none());
    }

    #[test]
    fn package_requirement_satisfied_by() {
        let r = PackageRequirement::parse("redshift@>=3.5,<4.0").unwrap();
        assert!(r.satisfied_by("3.5.2").unwrap());
        assert!(!r.satisfied_by("4.0.0").unwrap());

        let any = PackageRequirement::parse("python").unwrap();
        assert!(any.satisfied_by("3.11.0").unwrap());
    }
}
