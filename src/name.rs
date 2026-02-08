//! Package name parsing utilities (Rez-style).
//!
//! Rez treats package names and versions as separate components with a
//! single separator. Package names must not contain `-`, `@`, or `#`.
//! Versions may contain alphanumeric tokens separated by `.` or `-`.
//!
//! This module parses identifiers of the form:
//!
//! ```text
//! {name}{sep}{version}[--{variant}]
//! ```
//!
//! Where `{sep}` is one of `-`, `@`, `#`.
//!
//! Notes:
//! - The optional `--{variant}` suffix is a pkg-rs extension to keep the
//!   variant distinct from the version string. It is **not** part of Rez.
//! - If no separator is present, the identifier is treated as name-only.

use crate::rez_version::Version;

/// Parsed package identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// Package base name (e.g., "maya").
    pub name: String,
    /// Version string (Rez-style), if present.
    pub version: Option<String>,
    /// Optional variant suffix (pkg-rs extension, after `--`).
    pub variant: Option<String>,
}

impl PackageId {
    /// Parse a package ID string into components.
    ///
    /// Examples:
    /// - `maya-2026.1.0` => name="maya", version="2026.1.0"
    /// - `houdini@20.5` => name="houdini", version="20.5"
    /// - `foo` => name="foo", version=None
    /// - `maya-2026.1.0--win64` => variant="win64" (pkg-rs extension)
    pub fn parse(id: &str) -> Option<Self> {
        let id = id.trim();
        if id.is_empty() {
            return None;
        }

        let (main, variant) = if let Some(idx) = id.find("--") {
            let (left, right) = id.split_at(idx);
            let variant = right.strip_prefix("--").unwrap_or("");
            (left, if variant.is_empty() { None } else { Some(variant.to_string()) })
        } else {
            (id, None)
        };

        let mut sep_idx = None;
        for (i, ch) in main.char_indices() {
            if matches!(ch, '-' | '@' | '#') {
                sep_idx = Some((i, ch));
                break;
            }
        }

        if let Some((idx, _sep)) = sep_idx {
            let name = &main[..idx];
            let version = &main[idx + 1..];
            if name.is_empty() || version.is_empty() {
                return None;
            }
            if name.contains(['-', '@', '#']) {
                return None;
            }
            if Version::parse(version).is_err() {
                return None;
            }

            Some(PackageId {
                name: name.to_string(),
                version: Some(version.to_string()),
                variant,
            })
        } else {
            if main.contains(['-', '@', '#']) {
                return None;
            }
            Some(PackageId {
                name: main.to_string(),
                version: None,
                variant,
            })
        }
    }

    /// Return version string, if present.
    pub fn version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Reconstruct full ID string: `name[-version][--variant]`.
    pub fn id(&self) -> String {
        let mut out = self.name.clone();
        if let Some(version) = &self.version {
            out.push('-');
            out.push_str(version);
        }
        if let Some(variant) = &self.variant {
            out.push_str("--");
            out.push_str(variant);
        }
        out
    }

    /// Check if this ID has any version information.
    pub fn has_version(&self) -> bool {
        self.version.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name_only() {
        let id = PackageId::parse("maya").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.version, None);
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_version() {
        let id = PackageId::parse("maya-2026.1.0").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.version, Some("2026.1.0".to_string()));
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_variant_extension() {
        let id = PackageId::parse("maya-2026.1.0--win64").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.version, Some("2026.1.0".to_string()));
        assert_eq!(id.variant, Some("win64".to_string()));
    }

    #[test]
    fn reject_invalid() {
        assert!(PackageId::parse("").is_none());
        assert!(PackageId::parse("maya-").is_none());
        assert!(PackageId::parse("maya-@").is_none());
        assert!(PackageId::parse("my-plugin-1.0").is_none());
    }
}
