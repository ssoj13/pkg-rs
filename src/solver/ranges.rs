//! DepSpec to PubGrub Ranges conversion (Rez versioning).

use crate::dep::DepSpec;
use crate::error::SolverError;
use crate::rez_version::Version;
use pubgrub::Ranges;

/// Convert DepSpec constraint to PubGrub Ranges.
///
/// Uses Rez-style version ranges (`1+<4`, `>=2,<=5`, `==1.0`, `3|5+`).
pub fn depspec_to_ranges(spec: &DepSpec) -> Result<Ranges<Version>, SolverError> {
    let range = spec.version_range().map_err(|e| SolverError::InvalidDependency {
        package: spec.base.clone(),
        dependency: spec.constraint.clone(),
        reason: e.to_string(),
    })?;
    Ok(range.to_pubgrub())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rez_version::VersionRange;

    fn spec(constraint: &str) -> DepSpec {
        DepSpec::new("pkg".to_string(), Some(constraint.to_string()))
    }

    #[test]
    fn ranges_any() {
        let range = depspec_to_ranges(&spec("*")).unwrap();
        assert!(range.contains(&Version::parse("0").unwrap()));
        assert!(range.contains(&Version::parse("999").unwrap()));
    }

    #[test]
    fn ranges_exact() {
        let range = depspec_to_ranges(&spec("==1.2.3")).unwrap();
        assert!(range.contains(&Version::parse("1.2.3").unwrap()));
        assert!(!range.contains(&Version::parse("1.2.4").unwrap()));
    }

    #[test]
    fn ranges_superset() {
        let range = depspec_to_ranges(&spec("1.2")).unwrap();
        assert!(range.contains(&Version::parse("1.2").unwrap()));
        assert!(range.contains(&Version::parse("1.2.9").unwrap()));
        assert!(!range.contains(&Version::parse("1.3").unwrap()));
    }

    #[test]
    fn ranges_bounded() {
        let range = depspec_to_ranges(&spec("1+<4.3")).unwrap();
        assert!(range.contains(&Version::parse("1.0").unwrap()));
        assert!(range.contains(&Version::parse("4.2.9").unwrap()));
        assert!(!range.contains(&Version::parse("4.3").unwrap()));
    }

    #[test]
    fn ranges_union() {
        let range = depspec_to_ranges(&spec("1|3")).unwrap();
        assert!(range.contains(&Version::parse("1.0").unwrap()));
        assert!(range.contains(&Version::parse("3.1").unwrap()));
        assert!(!range.contains(&Version::parse("2.0").unwrap()));
    }

    #[test]
    fn version_range_display_roundtrip() {
        let range = VersionRange::parse("1+<4.3").unwrap();
        assert_eq!(range.to_string(), "1+<4.3");
    }
}
