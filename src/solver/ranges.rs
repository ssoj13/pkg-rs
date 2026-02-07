//! DepSpec to PubGrub Ranges conversion.
//!
//! Converts semver-style constraints into PubGrub Ranges<Version>.

use crate::dep::DepSpec;
use crate::error::SolverError;
use pubgrub::Ranges;
use semver::Version;

/// Convert DepSpec constraint to PubGrub Ranges.
///
/// Handles semver constraint syntax:
/// - `*` → full range (any version)
/// - `1.0.0` → singleton (exact version)
/// - `=1.0.0` → singleton
/// - `>1.0.0` → strictly_higher_than
/// - `>=1.0.0` → higher_than
/// - `<1.0.0` → strictly_lower_than
/// - `<=1.0.0` → lower_than
/// - `^1.2.3` → [1.2.3, 2.0.0) (caret)
/// - `~1.2.3` → [1.2.3, 1.3.0) (tilde)
/// - `>=1.0,<2.0` → intersection of constraints
pub fn depspec_to_ranges(spec: &DepSpec) -> Result<Ranges<Version>, SolverError> {
    let constraint = spec.constraint.trim();

    // Any version
    if constraint == "*" {
        return Ok(Ranges::full());
    }

    // Union (OR) constraints
    if constraint.contains('|') {
        let mut union_range: Option<Ranges<Version>> = None;
        for part in constraint.split('|') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let range = parse_constraint_part(part)?;
            union_range = Some(match union_range {
                None => range,
                Some(existing) => existing.union(&range),
            });
        }
        return union_range.ok_or_else(|| SolverError::InvalidDependency {
            package: "".to_string(),
            dependency: constraint.to_string(),
            reason: "Empty union constraint".to_string(),
        });
    }

    // Try as exact version first
    if let Ok(ver) = Version::parse(constraint) {
        return Ok(Ranges::singleton(ver));
    }

    // Handle comma-separated constraints (intersection)
    parse_constraint_part(constraint)
}

fn parse_constraint_part(constraint: &str) -> Result<Ranges<Version>, SolverError> {
    if constraint.contains(',') {
        return parse_intersection(constraint);
    }
    parse_single_constraint(constraint)
}

/// Parse comma-separated constraints as intersection.
fn parse_intersection(constraint: &str) -> Result<Ranges<Version>, SolverError> {
    let parts: Vec<&str> = constraint.split(',').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(SolverError::InvalidDependency {
            package: "".to_string(),
            dependency: constraint.to_string(),
            reason: "Empty constraint".to_string(),
        });
    }

    // Start with first constraint
    let mut result = parse_single_constraint(parts[0])?;

    // Intersect with remaining
    for part in &parts[1..] {
        let range = parse_single_constraint(part)?;
        result = result.intersection(&range);
    }

    Ok(result)
}

/// Parse a single constraint (no commas).
fn parse_single_constraint(constraint: &str) -> Result<Ranges<Version>, SolverError> {
    let constraint = constraint.trim();

    // Caret: ^1.2.3 → [1.2.3, 2.0.0)
    if let Some(rest) = constraint.strip_prefix('^') {
        return parse_caret(rest);
    }

    // Tilde: ~1.2.3 → [1.2.3, 1.3.0)
    if let Some(rest) = constraint.strip_prefix('~') {
        return parse_tilde(rest);
    }

    // Comparison operators (order matters: >= before >)
    if let Some(rest) = constraint.strip_prefix(">=") {
        let ver = parse_version(rest.trim())?;
        return Ok(Ranges::higher_than(ver));
    }

    if let Some(rest) = constraint.strip_prefix('>') {
        let ver = parse_version(rest.trim())?;
        return Ok(Ranges::strictly_higher_than(ver));
    }

    if let Some(rest) = constraint.strip_prefix("<=") {
        let ver = parse_version(rest.trim())?;
        return Ok(Ranges::lower_than(ver));
    }

    if let Some(rest) = constraint.strip_prefix('<') {
        let ver = parse_version(rest.trim())?;
        return Ok(Ranges::strictly_lower_than(ver));
    }

    if let Some(rest) = constraint.strip_prefix('=') {
        let ver = parse_version(rest.trim())?;
        return Ok(Ranges::singleton(ver));
    }

    // Try as bare version (exact)
    if let Ok(ver) = Version::parse(constraint) {
        return Ok(Ranges::singleton(ver));
    }

    Err(SolverError::InvalidDependency {
        package: "".to_string(),
        dependency: constraint.to_string(),
        reason: format!("Cannot parse constraint: {}", constraint),
    })
}

/// Parse caret constraint: ^1.2.3 → [1.2.3, 2.0.0)
fn parse_caret(version_str: &str) -> Result<Ranges<Version>, SolverError> {
    let ver = parse_version(version_str)?;

    // Caret rules:
    // ^1.2.3 → >=1.2.3, <2.0.0 (major bump)
    // ^0.2.3 → >=0.2.3, <0.3.0 (minor bump for 0.x)
    // ^0.0.3 → >=0.0.3, <0.0.4 (patch bump for 0.0.x)
    let upper = if ver.major > 0 {
        Version::new(ver.major + 1, 0, 0)
    } else if ver.minor > 0 {
        Version::new(0, ver.minor + 1, 0)
    } else {
        Version::new(0, 0, ver.patch + 1)
    };

    Ok(Ranges::between(ver, upper))
}

/// Parse tilde constraint: ~1.2.3 → [1.2.3, 1.3.0)
fn parse_tilde(version_str: &str) -> Result<Ranges<Version>, SolverError> {
    let ver = parse_version(version_str)?;

    // Tilde: same major.minor, any patch
    let upper = Version::new(ver.major, ver.minor + 1, 0);

    Ok(Ranges::between(ver, upper))
}

/// Parse version string to semver::Version.
/// Handles partial versions: "1" -> "1.0.0", "1.2" -> "1.2.0"
fn parse_version(s: &str) -> Result<Version, SolverError> {
    let s = s.trim();

    // Try direct parse first
    if let Ok(v) = Version::parse(s) {
        return Ok(v);
    }

    // Try adding missing parts
    let parts: Vec<&str> = s.split('.').collect();
    let normalized = match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => s.to_string(),
    };

    Version::parse(&normalized).map_err(|e| SolverError::InvalidVersion {
        package: "".to_string(),
        version: s.to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn spec(constraint: &str) -> DepSpec {
        DepSpec::new("pkg".to_string(), Some(constraint.to_string()))
    }

    #[test]
    fn ranges_any() {
        let range = depspec_to_ranges(&spec("*")).unwrap();
        assert!(range.contains(&v("0.0.1")));
        assert!(range.contains(&v("999.999.999")));
    }

    #[test]
    fn ranges_exact() {
        let range = depspec_to_ranges(&spec("1.2.3")).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(!range.contains(&v("1.2.4")));
        assert!(!range.contains(&v("1.2.2")));
    }

    #[test]
    fn ranges_gte() {
        let range = depspec_to_ranges(&spec(">=1.0.0")).unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("0.9.9")));
    }

    #[test]
    fn ranges_gt() {
        let range = depspec_to_ranges(&spec(">1.0.0")).unwrap();
        assert!(!range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.0.1")));
    }

    #[test]
    fn ranges_lte() {
        let range = depspec_to_ranges(&spec("<=2.0.0")).unwrap();
        assert!(range.contains(&v("2.0.0")));
        assert!(range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("2.0.1")));
    }

    #[test]
    fn ranges_lt() {
        let range = depspec_to_ranges(&spec("<2.0.0")).unwrap();
        assert!(!range.contains(&v("2.0.0")));
        assert!(range.contains(&v("1.9.9")));
    }

    #[test]
    fn ranges_caret() {
        // ^1.2.3 → [1.2.3, 2.0.0)
        let range = depspec_to_ranges(&spec("^1.2.3")).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.9.9")));
        assert!(!range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("1.2.2")));

        // ^0.2.3 → [0.2.3, 0.3.0)
        let range0 = depspec_to_ranges(&spec("^0.2.3")).unwrap();
        assert!(range0.contains(&v("0.2.3")));
        assert!(range0.contains(&v("0.2.9")));
        assert!(!range0.contains(&v("0.3.0")));
    }

    #[test]
    fn ranges_tilde() {
        // ~1.2.3 → [1.2.3, 1.3.0)
        let range = depspec_to_ranges(&spec("~1.2.3")).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.9")));
        assert!(!range.contains(&v("1.3.0")));
        assert!(!range.contains(&v("1.2.2")));
    }

    #[test]
    fn ranges_intersection() {
        // >=1.0.0,<2.0.0
        let range = depspec_to_ranges(&spec(">=1.0.0,<2.0.0")).unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.9.9")));
        assert!(!range.contains(&v("0.9.9")));
        assert!(!range.contains(&v("2.0.0")));
    }
}
