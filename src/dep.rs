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
//! - `name@version` - Exact version (e.g., `redshift@3.5.0`)
//!
//! Constraint syntax follows SemVer (VersionReq):
//! - `>=1.0.0` - Greater than or equal
//! - `<2.0.0` - Less than
//! - `^1.2.3` - Compatible (same major)
//! - `~1.2.3` - Compatible (same major.minor)
//! - `>=1.0,<2.0` - Multiple constraints (comma-separated)
//!
//! ## Resolved Dependencies
//!
//! Used in `Package.deps` for concrete solved versions:
//!
//! - `name-version` - Exact package (e.g., `redshift-3.5.2`)
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
//! // Parse resolved dependency
//! let resolved = DepSpec::parse("redshift-3.5.2")?;
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
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Dependency specification.
///
/// Represents either:
/// - A requirement with version constraint (`redshift@>=3.5`)
/// - A resolved dependency with exact version (`redshift-3.5.2`)
///
/// # Parsing Rules
///
/// 1. If contains `@`: Split on `@` → (base, constraint)
/// 2. If contains `-` followed by digit: Split → (base, exact version)
/// 3. Otherwise: base only, any version
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepSpec {
    /// Package base name (e.g., "redshift", "maya", "my-plugin").
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

    /// Check if this is an exact version (not a range).
    ///
    /// Returns true if constraint is a single exact version.
    pub fn is_exact(&self) -> bool {
        // Try to parse as exact version
        Version::parse(&self.constraint).is_ok()
    }

    /// Get exact version if this is an exact constraint.
    ///
    /// Returns None if this is a range constraint.
    pub fn exact_version(&self) -> Option<String> {
        if self.is_exact() {
            Some(self.constraint.clone())
        } else {
            None
        }
    }

    /// Check if this accepts any version.
    pub fn is_any(&self) -> bool {
        self.constraint == "*"
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
        if self.is_exact() {
            Some(format!("{}-{}", self.base, self.constraint))
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
            constraint: version,
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

        // Format 1: name@constraint (requirement)
        if let Some(at_pos) = spec.find('@') {
            let base = spec[..at_pos].to_string();
            let constraint = spec[at_pos + 1..].to_string();

            if base.is_empty() {
                return Err(PackageError::InvalidName {
                    name: spec.to_string(),
                    reason: "Empty base name".to_string(),
                });
            }

            // Validate constraint
            Self::validate_constraint(&constraint)?;

            return Ok(Self {
                base,
                constraint,
                original: spec.to_string(),
            });
        }

        // Format 2: name-version[-variant] (resolved dependency)
        // Uses shared name parsing logic from name module
        if let Some(pkg_id) = crate::name::PackageId::parse(spec) {
            // Only treat as resolved dependency if it has a version
            if let Some(version_str) = pkg_id.version() {
                // Validate as semver
                Version::parse(&version_str).map_err(|e| PackageError::InvalidVersion {
                    version: version_str.clone(),
                    reason: e.to_string(),
                })?;

                // Use full version+variant as constraint for exact match
                let constraint = match &pkg_id.variant {
                    Some(v) => format!("{}-{}", version_str, v),
                    None => version_str,
                };

                return Ok(Self {
                    base: pkg_id.name,
                    constraint,
                    original: spec.to_string(),
                });
            }
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
        if constraint == "*" {
            return Ok(());
        }

        // Try as exact version first
        if Version::parse(constraint).is_ok() {
            return Ok(());
        }

        // Try as version requirement
        VersionReq::parse(constraint).map_err(|e| PackageError::InvalidVersion {
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

        if self.constraint == "*" {
            return Ok(true);
        }

        // Try exact match first
        if let Ok(exact) = Version::parse(&self.constraint) {
            return Ok(ver == exact);
        }

        // Try as version requirement
        let req = VersionReq::parse(&self.constraint).map_err(|e| PackageError::InvalidVersion {
            version: self.constraint.clone(),
            reason: e.to_string(),
        })?;

        Ok(req.matches(&ver))
    }

    /// Get parsed VersionReq for solver integration.
    pub fn version_req(&self) -> Result<VersionReq, PackageError> {
        if self.constraint == "*" {
            return VersionReq::parse("*").map_err(|e| PackageError::InvalidVersion {
                version: "*".to_string(),
                reason: e.to_string(),
            });
        }

        // Exact version: convert to requirement
        if let Ok(ver) = Version::parse(&self.constraint) {
            let req_str = format!("={}", ver);
            return VersionReq::parse(&req_str).map_err(|e| PackageError::InvalidVersion {
                version: req_str,
                reason: e.to_string(),
            });
        }

        // Parse as requirement
        VersionReq::parse(&self.constraint).map_err(|e| PackageError::InvalidVersion {
            version: self.constraint.clone(),
            reason: e.to_string(),
        })
    }

    /// Get parsed Version for exact constraints.
    pub fn version(&self) -> Result<Version, PackageError> {
        Version::parse(&self.constraint).map_err(|e| PackageError::InvalidVersion {
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
        if let Some(dash_idx) = pkg.rfind('-') {
            let base = &pkg[..dash_idx];
            let version = &pkg[dash_idx + 1..];

            // Check base name match
            if base != spec.base {
                continue;
            }

            // Check version constraint
            if spec.matches_impl(version)? {
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

        // Exact version via @
        let spec2 = DepSpec::parse_impl("ocio@2.3.0").unwrap();
        assert_eq!(spec2.base, "ocio");
        assert_eq!(spec2.constraint, "2.3.0");
        assert!(spec2.is_exact());
    }

    #[test]
    fn depspec_parse_resolved() {
        let spec = DepSpec::parse_impl("redshift-3.5.2").unwrap();
        assert_eq!(spec.base, "redshift");
        assert_eq!(spec.constraint, "3.5.2");
        assert!(spec.is_exact());
        assert_eq!(spec.exact_version(), Some("3.5.2".to_string()));
    }

    #[test]
    fn depspec_parse_any() {
        let spec = DepSpec::parse_impl("redshift").unwrap();
        assert_eq!(spec.base, "redshift");
        assert_eq!(spec.constraint, "*");
        assert!(spec.is_any());
    }

    #[test]
    fn depspec_parse_dash_in_name() {
        // Dash in base name, followed by version
        let spec = DepSpec::parse_impl("my-plugin-1.0.0").unwrap();
        assert_eq!(spec.base, "my-plugin");
        assert_eq!(spec.constraint, "1.0.0");
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
        let exact = DepSpec::parse_impl("ocio@2.3.0").unwrap();
        assert!(exact.matches_impl("2.3.0").unwrap());
        assert!(!exact.matches_impl("2.3.1").unwrap());
        assert!(!exact.matches_impl("2.2.0").unwrap());

        // Any version
        let any = DepSpec::parse_impl("python").unwrap();
        assert!(any.matches_impl("3.11.0").unwrap());
        assert!(any.matches_impl("2.7.0").unwrap());
    }

    #[test]
    fn depspec_caret_tilde() {
        // Caret: same major
        let caret = DepSpec::parse_impl("pkg@^1.2.3").unwrap();
        assert!(caret.matches_impl("1.2.3").unwrap());
        assert!(caret.matches_impl("1.9.0").unwrap());
        assert!(!caret.matches_impl("2.0.0").unwrap());

        // Tilde: same minor
        let tilde = DepSpec::parse_impl("pkg@~1.2.3").unwrap();
        assert!(tilde.matches_impl("1.2.3").unwrap());
        assert!(tilde.matches_impl("1.2.9").unwrap());
        assert!(!tilde.matches_impl("1.3.0").unwrap());
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
        assert!(DepSpec::parse_impl("pkg@invalid").is_err());

        // Empty base
        assert!(DepSpec::parse_impl("@1.0.0").is_err());
    }
}
