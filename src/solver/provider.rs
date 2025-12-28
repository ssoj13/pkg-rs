//! PubGrub DependencyProvider implementation.
//!
//! Bridges our PackageIndex with PubGrub's resolution algorithm.

use super::ranges::depspec_to_ranges;
use super::PackageIndex;
use crate::dep::DepSpec;
use crate::error::SolverError;
use pubgrub::{Dependencies, DependencyProvider, Map, PackageResolutionStatistics, Ranges};
use semver::Version;
use std::cmp::Reverse;

/// PubGrub dependency provider.
///
/// Wraps PackageIndex and provides version/dependency info to PubGrub solver.
pub struct PubGrubProvider<'a> {
    index: &'a PackageIndex,
    /// Optional root dependencies for multi-requirement solving.
    root_deps: Option<Vec<DepSpec>>,
}

impl<'a> PubGrubProvider<'a> {
    /// Create provider from package index.
    pub fn new(index: &'a PackageIndex) -> Self {
        Self {
            index,
            root_deps: None,
        }
    }

    /// Create provider with virtual root dependencies.
    ///
    /// Used for solving multiple requirements at once.
    /// The virtual "__root__" package depends on all given specs.
    pub fn with_root_deps(index: &'a PackageIndex, deps: &[DepSpec]) -> Self {
        Self {
            index,
            root_deps: Some(deps.to_vec()),
        }
    }
}

impl DependencyProvider for PubGrubProvider<'_> {
    /// Package identifier (base name).
    type P = String;

    /// Version type.
    type V = Version;

    /// Version set (ranges).
    type VS = Ranges<Version>;

    /// Priority for package selection (higher = pick first).
    /// We use Reverse<Version> to prefer newest versions.
    type Priority = Reverse<Version>;

    /// Message for unavailable packages.
    type M = String;

    /// Error type.
    type Err = SolverError;

    /// Prioritize packages - prefer newest versions.
    fn prioritize(
        &self,
        package: &Self::P,
        _range: &Self::VS,
        _stats: &PackageResolutionStatistics,
    ) -> Self::Priority {
        // Return highest version as priority (Reverse makes higher = better)
        if let Some(ver) = self.index.versions(package).first() {
            Reverse((*ver).clone())
        } else {
            Reverse(Version::new(0, 0, 0))
        }
    }

    /// Choose best version matching the range.
    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        // Virtual root package
        if package == "__root__" {
            return Ok(Some(Version::new(0, 0, 0)));
        }

        // Get all versions (already sorted newest first)
        let versions = self.index.versions(package);

        // Find first matching version
        for ver in versions {
            if range.contains(ver) {
                return Ok(Some(ver.clone()));
            }
        }

        Ok(None)
    }

    /// Get dependencies for a package version.
    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        // Virtual root: return root_deps
        if package == "__root__" {
            if let Some(ref deps) = self.root_deps {
                let mut constraints: Map<String, Ranges<Version>> = Map::default();

                for spec in deps {
                    let range = depspec_to_ranges(spec)?;
                    constraints.insert(spec.base.clone(), range);
                }

                return Ok(Dependencies::Available(constraints));
            } else {
                return Ok(Dependencies::Available(Map::default()));
            }
        }

        // Get package dependencies
        let Some(deps) = self.index.deps(package, version) else {
            return Ok(Dependencies::Unavailable(format!(
                "Package {}-{} not found",
                package, version
            )));
        };

        // Convert DepSpecs to PubGrub constraints
        let mut constraints: Map<String, Ranges<Version>> = Map::default();

        for spec in deps {
            // Check if dependency exists in index
            if !self.index.has(&spec.base) {
                return Ok(Dependencies::Unavailable(format!(
                    "Dependency {} not found",
                    spec.base
                )));
            }

            let range = depspec_to_ranges(spec)?;

            // Merge with existing constraint (intersection)
            if let Some(existing) = constraints.get(&spec.base) {
                constraints.insert(spec.base.clone(), existing.intersection(&range));
            } else {
                constraints.insert(spec.base.clone(), range);
            }
        }

        Ok(Dependencies::Available(constraints))
    }
}

/// Convert PubGrub error to SolverError.
pub fn pubgrub_error_to_solver_error(
    error: pubgrub::PubGrubError<PubGrubProvider<'_>>,
) -> SolverError {
    use pubgrub::{DefaultStringReporter, PubGrubError, Reporter};

    match error {
        PubGrubError::NoSolution(tree) => {
            // Generate human-readable conflict explanation
            let report = DefaultStringReporter::report(&tree);
            SolverError::Conflict {
                message: report,
            }
        }
        PubGrubError::ErrorInShouldCancel(e) => {
            SolverError::NoSolution {
                reason: format!("Cancelled: {}", e),
            }
        }
        PubGrubError::ErrorChoosingVersion { package, source } => {
            SolverError::NoMatchingVersion {
                package,
                constraint: source.to_string(),
            }
        }
        PubGrubError::ErrorRetrievingDependencies { package, version, source } => {
            SolverError::InvalidDependency {
                package: format!("{}-{}", package, version),
                dependency: "".to_string(),
                reason: source.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::Package;

    fn make_pkg(name: &str, version: &str, reqs: Vec<&str>) -> Package {
        let mut pkg = Package::new(name.to_string(), version.to_string());
        for req in reqs {
            pkg.add_req(req.to_string());
        }
        pkg
    }

    fn build_index(packages: Vec<Package>) -> PackageIndex {
        let mut index = PackageIndex::new();
        for pkg in packages {
            index.add(&pkg).unwrap();
        }
        index
    }

    #[test]
    fn provider_choose_version() {
        let index = build_index(vec![
            make_pkg("maya", "2026.0.0", vec![]),
            make_pkg("maya", "2026.1.0", vec![]),
            make_pkg("maya", "2025.0.0", vec![]),
        ]);

        let provider = PubGrubProvider::new(&index);

        // Full range: newest
        let range = Ranges::full();
        let ver = provider
            .choose_version(&"maya".to_string(), &range)
            .unwrap();
        assert_eq!(ver, Some(Version::parse("2026.1.0").unwrap()));

        // Constrained range
        let range2 = Ranges::strictly_lower_than(Version::parse("2026.0.0").unwrap());
        let ver2 = provider
            .choose_version(&"maya".to_string(), &range2)
            .unwrap();
        assert_eq!(ver2, Some(Version::parse("2025.0.0").unwrap()));

        // No match
        let range3 = Ranges::strictly_higher_than(Version::parse("3000.0.0").unwrap());
        let ver3 = provider
            .choose_version(&"maya".to_string(), &range3)
            .unwrap();
        assert_eq!(ver3, None);
    }

    #[test]
    fn provider_get_deps() {
        let index = build_index(vec![
            make_pkg("maya", "2026.0.0", vec!["redshift@>=3.0"]),
            make_pkg("redshift", "3.5.0", vec![]),
        ]);

        let provider = PubGrubProvider::new(&index);
        let ver = Version::parse("2026.0.0").unwrap();

        let deps = provider
            .get_dependencies(&"maya".to_string(), &ver)
            .unwrap();

        if let Dependencies::Available(map) = deps {
            assert!(map.contains_key("redshift"));
            let range = map.get("redshift").unwrap();
            assert!(range.contains(&Version::parse("3.5.0").unwrap()));
            assert!(!range.contains(&Version::parse("2.9.0").unwrap()));
        } else {
            panic!("Expected Available dependencies");
        }
    }

    #[test]
    fn provider_virtual_root() {
        let index = build_index(vec![
            make_pkg("maya", "2026.0.0", vec![]),
            make_pkg("houdini", "20.0.0", vec![]),
        ]);

        let specs = vec![
            DepSpec::parse_impl("maya@>=2026").unwrap(),
            DepSpec::parse_impl("houdini").unwrap(),
        ];

        let provider = PubGrubProvider::with_root_deps(&index, &specs);

        // Virtual root version
        let ver = provider
            .choose_version(&"__root__".to_string(), &Ranges::full())
            .unwrap();
        assert_eq!(ver, Some(Version::new(0, 0, 0)));

        // Virtual root deps
        let deps = provider
            .get_dependencies(&"__root__".to_string(), &Version::new(0, 0, 0))
            .unwrap();

        if let Dependencies::Available(map) = deps {
            assert!(map.contains_key("maya"));
            assert!(map.contains_key("houdini"));
        } else {
            panic!("Expected Available dependencies");
        }
    }
}
