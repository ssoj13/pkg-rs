//! Package name utilities (pathlib-like) with Rez-style versioning.
//!
//! This module provides a small, immutable object for working with package
//! identifiers like `maya-2026.1.0` or `my-plugin-1.2.3--win64`, with helpers
//! to convert between name-only, resolved, and requirement forms.

use crate::name::PackageId;
use crate::rez_version::{Requirement, Version, VersionError, VersionRange};
use std::fmt;
use std::cmp::Ordering;
use std::str::FromStr;

/// Parsed package identifier with typed version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName {
    base: String,
    version: Option<Version>,
    variant: Option<String>,
}

impl PackageName {
    /// Parse a package identifier.
    ///
    /// Examples:
    /// - `maya-2026.1.0`
    /// - `my-plugin-1.2.3`
    /// - `maya-2026.1.0--win64`
    /// - `foo` (name only)
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let id = PackageId::parse(input)
            .ok_or_else(|| VersionError::new("Invalid package identifier"))?;

        let version = match id.version.as_ref() {
            Some(v) => Some(Version::parse(v)?),
            None => None,
        };

        Ok(Self {
            base: id.name,
            version,
            variant: id.variant,
        })
    }

    /// Create from parts (validates name and version).
    pub fn from_parts(
        base: impl Into<String>,
        version: Option<&str>,
        variant: Option<impl Into<String>>,
    ) -> Result<Self, VersionError> {
        let base = base.into();
        validate_base(&base)?;
        let version = match version {
            Some(v) => Some(Version::parse(v)?),
            None => None,
        };
        Ok(Self {
            base,
            version,
            variant: variant.map(|v| v.into()),
        })
    }

    /// Base package name (family).
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Alias for base name (pathlib-like).
    pub fn name(&self) -> &str {
        &self.base
    }

    /// Parsed version (if present).
    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    /// Version string (if present).
    pub fn version_string(&self) -> Option<String> {
        self.version.as_ref().map(|v| v.to_string())
    }

    /// Variant (if present).
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    /// Major version token (if present).
    pub fn major(&self) -> Option<String> {
        self.version_tokens().and_then(|t| t.get(0).cloned())
    }

    /// Minor version token (if present).
    pub fn minor(&self) -> Option<String> {
        self.version_tokens().and_then(|t| t.get(1).cloned())
    }

    /// Patch version token (if present).
    pub fn patch(&self) -> Option<String> {
        self.version_tokens().and_then(|t| t.get(2).cloned())
    }

    /// Variant tokens from the version (tokens after major/minor/patch).
    pub fn version_variant(&self) -> Option<String> {
        let tokens = self.version_tokens()?;
        if tokens.len() > 3 {
            Some(tokens[3..].join("."))
        } else {
            None
        }
    }

    /// Variant value: explicit `--variant` if set, otherwise version variant.
    pub fn variant_string(&self) -> Option<String> {
        if let Some(v) = self.variant() {
            Some(v.to_string())
        } else {
            self.version_variant()
        }
    }

    /// True if no version is present.
    pub fn is_base_only(&self) -> bool {
        self.version.is_none()
    }

    /// True if a version is present.
    pub fn is_qualified(&self) -> bool {
        self.version.is_some()
    }

    /// Return a name-only version (like `Path::parent()`).
    pub fn parent(&self) -> Self {
        Self {
            base: self.base.clone(),
            version: None,
            variant: None,
        }
    }

    /// Return a new instance with a different base name.
    pub fn with_base(&self, base: impl Into<String>) -> Result<Self, VersionError> {
        let base = base.into();
        validate_base(&base)?;
        Ok(Self {
            base,
            version: self.version.clone(),
            variant: self.variant.clone(),
        })
    }

    /// Return a new instance with a different version (or no version).
    pub fn with_version_str(&self, version: Option<&str>) -> Result<Self, VersionError> {
        let version = match version {
            Some(v) => Some(Version::parse(v)?),
            None => None,
        };
        Ok(Self {
            base: self.base.clone(),
            version,
            variant: self.variant.clone(),
        })
    }

    /// Return a new instance with a different variant (or no variant).
    pub fn with_variant(&self, variant: Option<impl Into<String>>) -> Self {
        Self {
            base: self.base.clone(),
            version: self.version.clone(),
            variant: variant.map(|v| v.into()),
        }
    }

    /// Convert to full identifier: `name[-version][--variant]`.
    pub fn to_id(&self) -> String {
        let mut out = self.base.clone();
        if let Some(ver) = &self.version {
            out.push('-');
            out.push_str(&ver.to_string());
        }
        if let Some(variant) = &self.variant {
            out.push_str("--");
            out.push_str(variant);
        }
        out
    }

    /// Convert to resolved dependency string (`name-version`), if version exists.
    pub fn to_resolved(&self) -> Option<String> {
        self.version
            .as_ref()
            .map(|v| format!("{}-{}", self.base, v))
    }

    /// Convert to exact requirement (`name==version`), if version exists.
    pub fn to_exact_requirement(&self) -> Option<String> {
        self.version
            .as_ref()
            .map(|v| format!("{}=={}", self.base, v))
    }

    /// Convert to requirement string (`name` or `name==version`).
    pub fn to_requirement(&self) -> String {
        self.to_exact_requirement().unwrap_or_else(|| self.base.clone())
    }

    fn version_tokens(&self) -> Option<Vec<String>> {
        self.version.as_ref().map(|v| v.as_tuple())
    }

    /// Compare versions with another package name (same base required).
    pub fn cmp_version(&self, other: &Self) -> Result<Ordering, VersionError> {
        if self.base != other.base {
            return Err(VersionError::new("Cannot compare different package bases"));
        }
        let v1 = self
            .version
            .as_ref()
            .ok_or_else(|| VersionError::new("Missing version on left operand"))?;
        let v2 = other
            .version
            .as_ref()
            .ok_or_else(|| VersionError::new("Missing version on right operand"))?;
        Ok(v1.cmp(v2))
    }

    /// True if this version is newer than the other (same base required).
    pub fn is_newer_than(&self, other: &Self) -> Result<bool, VersionError> {
        Ok(self.cmp_version(other)? == Ordering::Greater)
    }

    /// Test whether this package matches a version range string.
    ///
    /// The range applies to this package's version; if no version is present,
    /// this returns false.
    pub fn matches_range_str(&self, range: &str) -> Result<bool, VersionError> {
        let version = match &self.version {
            Some(v) => v,
            None => return Ok(false),
        };
        let range = VersionRange::parse(range)?;
        Ok(range.contains_version(version))
    }

    /// Create an exact range for this package version (if present).
    pub fn to_exact_range(&self) -> Result<PackageNameRange, VersionError> {
        let version = self
            .version
            .as_ref()
            .ok_or_else(|| VersionError::new("Missing version"))?;
        let range = VersionRange::parse(&format!("=={}", version))?;
        Ok(PackageNameRange {
            base: self.base.clone(),
            range,
            conflict: false,
            weak: false,
        })
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_id())
    }
}

impl FromStr for PackageName {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for PackageName {
    type Error = VersionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

fn validate_base(base: &str) -> Result<(), VersionError> {
    let id = PackageId::parse(base).ok_or_else(|| VersionError::new("Invalid base name"))?;
    if id.version.is_some() {
        return Err(VersionError::new("Base name contains version separator"));
    }
    if id.variant.is_some() {
        return Err(VersionError::new("Base name must not include variant"));
    }
    Ok(())
}

/// Parsed requirement with a package base and version range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageNameRange {
    pub base: String,
    pub range: VersionRange,
    pub conflict: bool,
    pub weak: bool,
}

impl PackageNameRange {
    /// Parse requirement strings like:
    /// - `foo`
    /// - `foo-1.2`
    /// - `foo@>=1,<3`
    /// - `foo==2.0`
    /// - `!foo-2` / `~foo-2`
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let req = Requirement::parse(input)?;
        let range = match req.range {
            Some(r) => r,
            None => VersionRange::parse("")?,
        };

        Ok(Self {
            base: req.name,
            range,
            conflict: req.conflict,
            weak: req.weak,
        })
    }

    /// Check if a version is within this range (ignores conflict/weak flags).
    pub fn contains_version(&self, version: &Version) -> bool {
        self.range.contains_version(version)
    }

    /// Check if a package name matches this requirement (conflict respected).
    pub fn matches(&self, name: &PackageName) -> Result<bool, VersionError> {
        if name.base != self.base {
            return Ok(false);
        }
        let version = match &name.version {
            Some(v) => v,
            None => return Ok(false),
        };
        let ok = self.range.contains_version(version);
        if self.conflict {
            Ok(!ok)
        } else {
            Ok(ok)
        }
    }

    /// Convert back to requirement string.
    pub fn to_requirement(&self) -> String {
        if self.range.is_any() {
            if self.conflict {
                format!("!{}", self.base)
            } else if self.weak {
                format!("~{}", self.base)
            } else {
                self.base.clone()
            }
        } else if self.conflict {
            format!("!{}", join_req(&self.base, &self.range.to_string()))
        } else if self.weak {
            format!("~{}", join_req(&self.base, &self.range.to_string()))
        } else {
            join_req(&self.base, &self.range.to_string())
        }
    }
}

fn join_req(base: &str, range: &str) -> String {
    if range.is_empty() {
        return base.to_string();
    }
    let first = range.chars().next().unwrap();
    if first.is_ascii_alphanumeric() || first == '_' {
        format!("{}-{}", base, range)
    } else {
        format!("{}{}", base, range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name_only() {
        let p = PackageName::parse("maya").unwrap();
        assert_eq!(p.base(), "maya");
        assert!(p.version().is_none());
        assert!(p.variant().is_none());
        assert_eq!(p.to_id(), "maya");
    }

    #[test]
    fn parse_versioned() {
        let p = PackageName::parse("maya-2026.1.0").unwrap();
        assert_eq!(p.base(), "maya");
        assert_eq!(p.version_string(), Some("2026.1.0".to_string()));
        assert_eq!(p.to_resolved(), Some("maya-2026.1.0".to_string()));
        assert_eq!(p.to_exact_requirement(), Some("maya==2026.1.0".to_string()));
        assert_eq!(p.major(), Some("2026".to_string()));
        assert_eq!(p.minor(), Some("1".to_string()));
        assert_eq!(p.patch(), Some("0".to_string()));
        assert_eq!(p.variant_string(), None);
    }

    #[test]
    fn parse_hyphenated_base() {
        let p = PackageName::parse("my-plugin-1.2.3").unwrap();
        assert_eq!(p.base(), "my-plugin");
        assert_eq!(p.version_string(), Some("1.2.3".to_string()));
    }

    #[test]
    fn parse_with_version_variant() {
        let p = PackageName::parse("maya-boost-2026.0.1-linux").unwrap();
        assert_eq!(p.base(), "maya-boost");
        assert_eq!(p.version_string(), Some("2026.0.1-linux".to_string()));
        assert_eq!(p.major(), Some("2026".to_string()));
        assert_eq!(p.minor(), Some("0".to_string()));
        assert_eq!(p.patch(), Some("1".to_string()));
        assert_eq!(p.variant_string(), Some("linux".to_string()));
    }

    #[test]
    fn parse_variant() {
        let p = PackageName::parse("maya-2026.1.0--win64").unwrap();
        assert_eq!(p.variant(), Some("win64"));
        assert_eq!(p.to_id(), "maya-2026.1.0--win64");
    }

    #[test]
    fn withers() {
        let p = PackageName::parse("maya-2026.1.0").unwrap();
        let q = p.with_version_str(Some("2027.0.0")).unwrap();
        assert_eq!(q.to_id(), "maya-2027.0.0");
        let r = q.with_variant(Some("win64"));
        assert_eq!(r.to_id(), "maya-2027.0.0--win64");
        let s = r.parent();
        assert_eq!(s.to_id(), "maya");
    }

    #[test]
    fn range_parse_and_match() {
        let r = PackageNameRange::parse("maya@>=2026,<2027").unwrap();
        let p = PackageName::parse("maya-2026.1.0").unwrap();
        assert!(r.matches(&p).unwrap());
        let p2 = PackageName::parse("maya-2027.0.0").unwrap();
        assert!(!r.matches(&p2).unwrap());
    }

    #[test]
    fn compare_versions() {
        let a = PackageName::parse("maya-2026.0.0").unwrap();
        let b = PackageName::parse("maya-2026.1.0").unwrap();
        assert!(b.is_newer_than(&a).unwrap());
    }
}
