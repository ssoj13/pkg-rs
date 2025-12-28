//! Package name parsing utilities.
//!
//! Provides unified logic for parsing package identifiers into components.
//! Used by [`Package`](crate::package::Package) and `DepSpec` for consistent parsing.
//!
//! # Package ID Format
//!
//! ```text
//! {name}-{version}[-{variant}]
//!       ^         ^
//!       |         |
//!       |         next "-" after version = end of version, start of variant
//!       |
//!       first "-digit" = start of version
//! ```
//!
//! ## Components
//!
//! | Component | Required | Description |
//! |-----------|----------|-------------|
//! | `name`    | Yes      | Package name, may contain dashes (e.g., "my-cool-plugin") |
//! | `version` | No       | Version string: `major[.minor[.patch]]` (max 3 components) |
//! | `variant` | No       | Build variant after version (e.g., "win64", "py310", "debug") |
//!
//! ## Parsing Rules
//!
//! 1. **Name ends** at first `-` followed by digit (start of version)
//! 2. **Version ends** at next `-` after version start (start of variant)
//! 3. **Version components** are split by `.`, max 3 (major.minor.patch)
//! 4. **Variant** is everything after version's trailing `-`
//!
//! ## Examples
//!
//! | Input | name | major | minor | patch | variant |
//! |-------|------|-------|-------|-------|---------|
//! | `maya` | maya | None | None | None | None |
//! | `maya-2026` | maya | 2026 | None | None | None |
//! | `maya-2026.1` | maya | 2026 | 1 | None | None |
//! | `maya-2026.1.0` | maya | 2026 | 1 | 0 | None |
//! | `maya-2026.1.0-win64` | maya | 2026 | 1 | 0 | win64 |
//! | `pkg-1-win64` | pkg | 1 | None | None | win64 |
//! | `pkg-1-123` | pkg | 1 | None | None | 123 |
//! | `pkg-1.0.0-123` | pkg | 1 | 0 | 0 | 123 |
//! | `my-plugin-1.0-beta` | my-plugin | 1 | 0 | None | beta |
//! | `USD-24.11-py310-win64` | USD | 24 | 11 | None | py310-win64 |
//!
//! # Usage
//!
//! ```ignore
//! use pkg_lib::name::PackageId;
//!
//! // Parse full ID
//! let id = PackageId::parse("maya-2026.1.0-win64").unwrap();
//! assert_eq!(id.name, "maya");
//! assert_eq!(id.major, Some(2026));
//! assert_eq!(id.minor, Some(1));
//! assert_eq!(id.patch, Some(0));
//! assert_eq!(id.variant, Some("win64".to_string()));
//!
//! // Reconstruct strings
//! assert_eq!(id.version(), Some("2026.1.0".to_string()));
//! assert_eq!(id.id(), "maya-2026.1.0-win64");
//!
//! // Name-only package
//! let id = PackageId::parse("maya").unwrap();
//! assert_eq!(id.version(), None);
//! ```

use std::fmt;

/// Parsed package identifier with structured version components.
///
/// # Fields
///
/// - `name`: Package base name (always present)
/// - `major`, `minor`, `patch`: Version components (None if not specified)
/// - `variant`: Build variant (None if not specified)
///
/// # Version Rules
///
/// - Max 3 version components (major.minor.patch)
/// - Each component is `Option<u32>` - None means not specified
/// - Version starts at first `-digit` in the ID string
/// - Version ends at next `-` (which starts variant)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// Package base name (e.g., "maya", "my-cool-plugin")
    pub name: String,

    /// Major version component (e.g., 2026 in "maya-2026.1.0")
    pub major: Option<u32>,

    /// Minor version component (e.g., 1 in "maya-2026.1.0")
    pub minor: Option<u32>,

    /// Patch version component (e.g., 0 in "maya-2026.1.0")
    pub patch: Option<u32>,

    /// Build variant (e.g., "win64", "py310", "debug")
    /// Everything after version's trailing dash
    pub variant: Option<String>,
}

impl PackageId {
    /// Parse a package ID string into components.
    ///
    /// # Parsing Algorithm
    ///
    /// ```text
    /// "my-plugin-1.0.0-win64"
    ///  ^        ^^   ^^
    ///  |        ||   ||
    ///  |        ||   |+-- variant = "win64"
    ///  |        ||   +--- version ends here (dash after version)
    ///  |        |+------- version = "1.0.0"
    ///  |        +-------- version starts here (first "-digit")
    ///  +----------------- name = "my-plugin"
    /// ```
    ///
    /// # Returns
    ///
    /// - `Some(PackageId)` on successful parse
    /// - `None` if name is empty or version has >3 components
    pub fn parse(id: &str) -> Option<Self> {
        if id.is_empty() {
            return None;
        }

        // Step 1: Find version start (first "-digit")
        let version_start = find_version_start(id);

        match version_start {
            None => {
                // No version - name only (e.g., "maya")
                Some(PackageId {
                    name: id.to_string(),
                    major: None,
                    minor: None,
                    patch: None,
                    variant: None,
                })
            }
            Some(start_pos) => {
                // Extract name (everything before version)
                let name = &id[..start_pos];
                if name.is_empty() {
                    return None;
                }

                // Step 2: Find version end (next "-" after version start)
                let after_dash = start_pos + 1; // skip the "-"
                let rest = &id[after_dash..];

                let (version_str, variant) = split_version_variant(rest);

                // Step 3: Parse version components (max 3)
                let (major, minor, patch) = parse_version_components(version_str)?;

                Some(PackageId {
                    name: name.to_string(),
                    major: Some(major),
                    minor,
                    patch,
                    variant: variant.map(|s| s.to_string()),
                })
            }
        }
    }

    /// Reconstruct version string from components.
    ///
    /// Returns `None` if no version components are set.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // major=1, minor=0, patch=0 -> "1.0.0"
    /// // major=1, minor=0, patch=None -> "1.0"
    /// // major=1, minor=None, patch=None -> "1"
    /// // major=None -> None
    /// ```
    pub fn version(&self) -> Option<String> {
        let major = self.major?;

        Some(match (self.minor, self.patch) {
            (Some(minor), Some(patch)) => format!("{}.{}.{}", major, minor, patch),
            (Some(minor), None) => format!("{}.{}", major, minor),
            (None, _) => format!("{}", major),
        })
    }

    /// Reconstruct full ID string: `name[-version][-variant]`
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // name="maya", version=2026.1.0, variant=win64 -> "maya-2026.1.0-win64"
    /// // name="maya", version=None, variant=None -> "maya"
    /// ```
    pub fn id(&self) -> String {
        let mut result = self.name.clone();

        if let Some(version) = self.version() {
            result.push('-');
            result.push_str(&version);
        }

        if let Some(ref variant) = self.variant {
            result.push('-');
            result.push_str(variant);
        }

        result
    }

    /// Check if this ID has any version information.
    pub fn has_version(&self) -> bool {
        self.major.is_some()
    }

    /// Check if this ID has a variant.
    pub fn has_variant(&self) -> bool {
        self.variant.is_some()
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Find position of version start (first "-" followed by digit).
///
/// Returns byte position of the "-" or None if no version found.
///
/// ```text
/// "my-plugin-1.0.0" -> Some(9)  (position of "-1")
/// "maya-2026"       -> Some(4)  (position of "-2")
/// "maya"            -> None
/// ```
fn find_version_start(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();

    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'-' && bytes[i + 1].is_ascii_digit() {
            return Some(i);
        }
    }

    None
}

/// Split version string at first "-" (variant separator).
///
/// ```text
/// "1.0.0-win64" -> ("1.0.0", Some("win64"))
/// "1.0.0"       -> ("1.0.0", None)
/// "1-win64"     -> ("1", Some("win64"))
/// ```
fn split_version_variant(s: &str) -> (&str, Option<&str>) {
    match s.find('-') {
        Some(pos) => {
            let version = &s[..pos];
            let variant = &s[pos + 1..];
            if variant.is_empty() {
                (version, None)
            } else {
                (version, Some(variant))
            }
        }
        None => (s, None),
    }
}

/// Parse version string into (major, minor?, patch?) components.
///
/// Returns `None` if more than 3 components or invalid numbers.
///
/// ```text
/// "1"       -> (1, None, None)
/// "1.0"     -> (1, Some(0), None)
/// "1.0.0"   -> (1, Some(0), Some(0))
/// "1.0.0.1" -> None (too many components)
/// ```
fn parse_version_components(version: &str) -> Option<(u32, Option<u32>, Option<u32>)> {
    let parts: Vec<&str> = version.split('.').collect();

    // Max 3 components allowed
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }

    let major: u32 = parts[0].parse().ok()?;
    let minor: Option<u32> = parts.get(1).map(|s| s.parse().ok()).flatten();
    let patch: Option<u32> = parts.get(2).map(|s| s.parse().ok()).flatten();

    // If parsing failed for existing parts, return None
    if parts.len() >= 2 && minor.is_none() {
        return None;
    }
    if parts.len() >= 3 && patch.is_none() {
        return None;
    }

    Some((major, minor, patch))
}

// =============================================================================
// Legacy compatibility - deprecated, will be removed
// =============================================================================

/// Split a package ID into base name and version string.
///
/// **Deprecated:** Use [`PackageId::parse()`] instead.
#[deprecated(note = "Use PackageId::parse() instead")]
pub fn split_name_version(id: &str) -> Option<(&str, &str)> {
    let start = find_version_start(id)?;
    let name = &id[..start];
    let version = &id[start + 1..];

    if name.is_empty() || version.is_empty() {
        return None;
    }

    Some((name, version))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Basic parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn parse_name_only() {
        let id = PackageId::parse("maya").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.major, None);
        assert_eq!(id.minor, None);
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_name_with_dashes() {
        let id = PackageId::parse("my-cool-plugin").unwrap();
        assert_eq!(id.name, "my-cool-plugin");
        assert_eq!(id.major, None);
    }

    #[test]
    fn parse_major_only() {
        let id = PackageId::parse("maya-2026").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.major, Some(2026));
        assert_eq!(id.minor, None);
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_major_minor() {
        let id = PackageId::parse("maya-2026.1").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.major, Some(2026));
        assert_eq!(id.minor, Some(1));
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_full_version() {
        let id = PackageId::parse("maya-2026.1.0").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.major, Some(2026));
        assert_eq!(id.minor, Some(1));
        assert_eq!(id.patch, Some(0));
        assert_eq!(id.variant, None);
    }

    #[test]
    fn parse_with_variant() {
        let id = PackageId::parse("maya-2026.1.0-win64").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.major, Some(2026));
        assert_eq!(id.minor, Some(1));
        assert_eq!(id.patch, Some(0));
        assert_eq!(id.variant, Some("win64".to_string()));
    }

    // -------------------------------------------------------------------------
    // Complex name tests
    // -------------------------------------------------------------------------

    #[test]
    fn parse_dashed_name_with_version() {
        let id = PackageId::parse("my-plugin-1.0.0").unwrap();
        assert_eq!(id.name, "my-plugin");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.minor, Some(0));
        assert_eq!(id.patch, Some(0));
    }

    #[test]
    fn parse_dashed_name_with_variant() {
        let id = PackageId::parse("my-cool-plugin-1.0.0-win64").unwrap();
        assert_eq!(id.name, "my-cool-plugin");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.minor, Some(0));
        assert_eq!(id.patch, Some(0));
        assert_eq!(id.variant, Some("win64".to_string()));
    }

    // -------------------------------------------------------------------------
    // Short version + variant tests
    // -------------------------------------------------------------------------

    #[test]
    fn parse_major_with_variant() {
        let id = PackageId::parse("pkg-1-win64").unwrap();
        assert_eq!(id.name, "pkg");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.minor, None);
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, Some("win64".to_string()));
    }

    #[test]
    fn parse_major_minor_with_variant() {
        let id = PackageId::parse("pkg-1.0-beta").unwrap();
        assert_eq!(id.name, "pkg");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.minor, Some(0));
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, Some("beta".to_string()));
    }

    #[test]
    fn parse_numeric_variant() {
        // "123" after version is variant (version ends at first "-")
        let id = PackageId::parse("pkg-1-123").unwrap();
        assert_eq!(id.name, "pkg");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.variant, Some("123".to_string()));
    }

    #[test]
    fn parse_numeric_variant_full_version() {
        let id = PackageId::parse("pkg-1.0.0-123").unwrap();
        assert_eq!(id.name, "pkg");
        assert_eq!(id.major, Some(1));
        assert_eq!(id.minor, Some(0));
        assert_eq!(id.patch, Some(0));
        assert_eq!(id.variant, Some("123".to_string()));
    }

    // -------------------------------------------------------------------------
    // Complex variant tests
    // -------------------------------------------------------------------------

    #[test]
    fn parse_compound_variant() {
        let id = PackageId::parse("USD-24.11-py310-win64").unwrap();
        assert_eq!(id.name, "USD");
        assert_eq!(id.major, Some(24));
        assert_eq!(id.minor, Some(11));
        assert_eq!(id.patch, None);
        assert_eq!(id.variant, Some("py310-win64".to_string()));
    }

    #[test]
    fn parse_long_variant() {
        let id = PackageId::parse("tool-1.0.0-linux-x86_64-debug").unwrap();
        assert_eq!(id.name, "tool");
        assert_eq!(id.version(), Some("1.0.0".to_string()));
        assert_eq!(id.variant, Some("linux-x86_64-debug".to_string()));
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn parse_empty() {
        assert!(PackageId::parse("").is_none());
    }

    #[test]
    fn parse_too_many_components() {
        // 4 version components = error
        assert!(PackageId::parse("pkg-1.0.0.1").is_none());
    }

    #[test]
    fn parse_no_version_suffix() {
        // No -digit pattern = name only, no version
        let id = PackageId::parse("pkg-abc").unwrap();
        assert_eq!(id.name, "pkg-abc");
        assert!(id.major.is_none());
        assert!(id.variant.is_none());
    }

    // -------------------------------------------------------------------------
    // Reconstruction tests
    // -------------------------------------------------------------------------

    #[test]
    fn version_reconstruction() {
        let id = PackageId::parse("maya-2026.1.0").unwrap();
        assert_eq!(id.version(), Some("2026.1.0".to_string()));

        let id = PackageId::parse("maya-2026.1").unwrap();
        assert_eq!(id.version(), Some("2026.1".to_string()));

        let id = PackageId::parse("maya-2026").unwrap();
        assert_eq!(id.version(), Some("2026".to_string()));

        let id = PackageId::parse("maya").unwrap();
        assert_eq!(id.version(), None);
    }

    #[test]
    fn id_reconstruction() {
        let id = PackageId::parse("maya-2026.1.0-win64").unwrap();
        assert_eq!(id.id(), "maya-2026.1.0-win64");

        let id = PackageId::parse("maya-2026.1.0").unwrap();
        assert_eq!(id.id(), "maya-2026.1.0");

        let id = PackageId::parse("maya").unwrap();
        assert_eq!(id.id(), "maya");
    }

    #[test]
    fn display_trait() {
        let id = PackageId::parse("app-2.0.0-debug").unwrap();
        assert_eq!(format!("{}", id), "app-2.0.0-debug");
    }

    // -------------------------------------------------------------------------
    // Real-world examples
    // -------------------------------------------------------------------------

    #[test]
    fn real_world_maya() {
        let id = PackageId::parse("maya-2026.1.0").unwrap();
        assert_eq!(id.name, "maya");
        assert_eq!(id.version(), Some("2026.1.0".to_string()));
    }

    #[test]
    fn real_world_houdini() {
        let id = PackageId::parse("houdini-20.5.332-py310").unwrap();
        assert_eq!(id.name, "houdini");
        assert_eq!(id.major, Some(20));
        assert_eq!(id.minor, Some(5));
        assert_eq!(id.patch, Some(332));
        assert_eq!(id.variant, Some("py310".to_string()));
    }

    #[test]
    fn real_world_nuke() {
        let id = PackageId::parse("nuke-15.1").unwrap();
        assert_eq!(id.name, "nuke");
        assert_eq!(id.major, Some(15));
        assert_eq!(id.minor, Some(1));
        assert_eq!(id.patch, None);
    }

    #[test]
    fn real_world_usd() {
        let id = PackageId::parse("USD-24.11-py310-win64").unwrap();
        assert_eq!(id.name, "USD");
        assert_eq!(id.major, Some(24));
        assert_eq!(id.minor, Some(11));
        assert_eq!(id.variant, Some("py310-win64".to_string()));
    }

    // -------------------------------------------------------------------------
    // Helper method tests
    // -------------------------------------------------------------------------

    #[test]
    fn has_version_check() {
        assert!(PackageId::parse("pkg-1.0.0").unwrap().has_version());
        assert!(!PackageId::parse("pkg").unwrap().has_version());
    }

    #[test]
    fn has_variant_check() {
        assert!(PackageId::parse("pkg-1.0.0-win64").unwrap().has_variant());
        assert!(!PackageId::parse("pkg-1.0.0").unwrap().has_variant());
    }
}
