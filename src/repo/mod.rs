//! Package repository abstractions (Rez-style).
//!
//! Provides trait and types aligned with rez-next: RepositoryMetadata,
//! PackageSearchCriteria, RepositoryStats, and extended Repository API.

pub mod filesystem;
pub mod memory;

use crate::cache::Cache;
use crate::config;
use crate::dep::PackageRequirement;
use crate::package::Package;
use crate::plugins;
use crate::rez_version::Version;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub use filesystem::FilesystemRepository;
pub use memory::MemoryRepository;

/// Repository type (rez-next–style).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryType {
    FileSystem,
    Memory,
}

/// Repository metadata (rez-next–style interface).
#[derive(Debug, Clone)]
pub struct RepositoryMetadata {
    pub name: String,
    pub path: PathBuf,
    pub repository_type: RepositoryType,
    pub priority: i32,
    pub read_only: bool,
    pub description: Option<String>,
    pub config: HashMap<String, String>,
}

/// Package search criteria (rez-next–style).
#[derive(Debug, Clone, Default)]
pub struct PackageSearchCriteria {
    pub name_pattern: Option<String>,
    pub version_requirement: Option<String>,
    pub requirements: Vec<PackageRequirement>,
    pub limit: Option<usize>,
    pub include_prerelease: bool,
}

/// Package family entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageFamily {
    pub name: String,
}

/// Package resource entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageResource {
    pub name: String,
    pub version: String,
    pub location: PathBuf,
    pub source: PathBuf,
}

/// Variant resource entry (index-based).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantResource {
    pub package: PackageResource,
    pub index: Option<usize>,
}

/// Repository stats (rez-next–style: counts + timing).
#[derive(Debug, Clone, Copy, Default)]
pub struct RepositoryStats {
    pub package_load_time: f64,
    pub package_count: usize,
    pub version_count: usize,
    pub variant_count: usize,
    pub size_bytes: u64,
    pub last_scan_time: Option<i64>,
    pub last_scan_duration_ms: Option<u64>,
}

/// Package repository trait (Rez parity: extended API).
pub trait PackageRepository: Send + Sync {
    fn repo_type(&self) -> &'static str;
    fn location(&self) -> &PathBuf;
    fn packages(&self) -> &[Package];
    fn warnings(&self) -> &[String];

    /// Metadata for this repository (name, path, type, priority).
    fn metadata(&self) -> RepositoryMetadata {
        let path = self.location().clone();
        let name = path.to_string_lossy().to_string();
        let repository_type = match self.repo_type() {
            "memory" => RepositoryType::Memory,
            _ => RepositoryType::FileSystem,
        };
        RepositoryMetadata {
            name,
            path,
            repository_type,
            priority: 0,
            read_only: true,
            description: None,
            config: HashMap::new(),
        }
    }

    /// Whether the repository has been loaded (always true for sync impls).
    fn is_initialized(&self) -> bool {
        true
    }

    /// No-op for sync impls; refresh by re-scanning to get new repos.
    fn refresh(&self) {}

    fn is_empty(&self) -> bool {
        self.packages().is_empty()
    }

    fn families(&self) -> Vec<PackageFamily> {
        let mut seen = HashMap::new();
        for pkg in self.packages() {
            seen.insert(pkg.base.clone(), true);
        }
        let mut out: Vec<PackageFamily> = seen
            .keys()
            .map(|k| PackageFamily { name: k.clone() })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    fn packages_for(&self, base: &str) -> Vec<PackageResource> {
        self.packages()
            .iter()
            .filter(|p| p.base == base)
            .map(|p| PackageResource {
                name: p.base.clone(),
                version: p.version.clone(),
                location: self.location().clone(),
                source: PathBuf::from(p.package_source.clone().unwrap_or_default()),
            })
            .collect()
    }

    fn variants_for(&self, package: &PackageResource) -> Vec<VariantResource> {
        let pkg = self
            .packages()
            .iter()
            .find(|p| p.base == package.name && p.version == package.version);
        let Some(pkg) = pkg else {
            return Vec::new();
        };
        if pkg.variants.is_empty() {
            return vec![VariantResource {
                package: package.clone(),
                index: None,
            }];
        }
        (0..pkg.variants.len())
            .map(|idx| VariantResource {
                package: package.clone(),
                index: Some(idx),
            })
            .collect()
    }

    /// Find packages matching criteria (name pattern, version requirement, limit).
    fn find_packages(&self, criteria: &PackageSearchCriteria) -> Vec<Package> {
        let mut out: Vec<Package> = self.packages().to_vec();
        if let Some(ref pattern) = criteria.name_pattern {
            let pattern = pattern.trim();
            if !pattern.is_empty() && pattern != "*" {
                out.retain(|p| match_name_pattern(&p.base, pattern));
            }
        }
        if let Some(ref ver_req) = criteria.version_requirement {
            let ver_req = ver_req.trim();
            if !ver_req.is_empty() {
                out.retain(|p| match_version_requirement(&p.version, ver_req));
            }
        }
        if let Some(limit) = criteria.limit {
            out.truncate(limit);
        }
        out
    }

    /// Get a package by base name and optional version (None = latest).
    fn get_package(&self, name: &str, version: Option<&str>) -> Option<Package> {
        let mut matches: Vec<&Package> = self
            .packages()
            .iter()
            .filter(|p| p.base == name)
            .collect();
        if matches.is_empty() {
            return None;
        }
        match version {
            Some(v) => matches.into_iter().find(|p| p.version == v).cloned(),
            None => {
                matches.sort_by(|a, b| {
                    crate::rez_version::Version::parse(&a.version)
                        .ok()
                        .zip(crate::rez_version::Version::parse(&b.version).ok())
                        .map(|(va, vb)| vb.cmp(&va))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                matches.first().map(|p| (*p).clone())
            }
        }
    }

    /// All version strings for a package base.
    fn get_package_versions(&self, name: &str) -> Vec<String> {
        let mut versions: Vec<String> = self
            .packages()
            .iter()
            .filter(|p| p.base == name)
            .map(|p| p.version.clone())
            .collect();
        versions.sort_by(|a, b| {
            Version::parse(a)
                .ok()
                .zip(Version::parse(b).ok())
                .map(|(va, vb)| vb.cmp(&va))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        versions
    }

    /// Check if a package exists.
    fn package_exists(&self, name: &str, version: Option<&str>) -> bool {
        self.get_package(name, version).is_some()
    }

    /// All unique package base names.
    fn get_package_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.packages().iter().map(|p| p.base.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Repository statistics (counts and timing).
    fn get_stats(&self) -> RepositoryStats {
        let packages = self.packages();
        let package_count = packages.len();
        let version_count = packages.len();
        let variant_count = packages.iter().map(|p| p.variants.len()).sum();
        RepositoryStats {
            package_load_time: repository_stats().package_load_time,
            package_count,
            version_count,
            variant_count,
            size_bytes: 0,
            last_scan_time: None,
            last_scan_duration_ms: None,
        }
    }
}

fn match_name_pattern(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return name.starts_with(parts[0]) && name.ends_with(parts[1]);
        }
        if pattern.starts_with('*') {
            return name.ends_with(&pattern[1..]);
        }
        if pattern.ends_with('*') {
            return name.starts_with(&pattern[..pattern.len() - 1]);
        }
    }
    name == pattern
}

fn match_version_requirement(version: &str, ver_req: &str) -> bool {
    if let Ok(v) = Version::parse(version) {
        if let Ok(range) = crate::rez_version::VersionRange::parse(ver_req) {
            return range.contains_version(&v);
        }
    }
    false
}

static REPO_STATS: OnceLock<Mutex<RepositoryStats>> = OnceLock::new();

fn repo_stats_lock() -> &'static Mutex<RepositoryStats> {
    REPO_STATS.get_or_init(|| Mutex::new(RepositoryStats::default()))
}

pub fn with_package_loading<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64();
    if let Ok(mut stats) = repo_stats_lock().lock() {
        stats.package_load_time += elapsed;
    }
    result
}

pub fn repository_stats() -> RepositoryStats {
    repo_stats_lock().lock().map(|s| *s).unwrap_or_default()
}

/// Repository manager: holds multiple repositories and queries across them (rez-next–style).
#[derive(Default)]
pub struct RepositoryManager {
    repositories: Vec<Box<dyn PackageRepository>>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    pub fn add_repository(&mut self, repo: Box<dyn PackageRepository>) {
        self.repositories.push(repo);
    }

    pub fn repository_count(&self) -> usize {
        self.repositories.len()
    }

    /// Find packages across all repositories; deduplicates by name-version (first wins).
    pub fn find_packages(&self, criteria: &PackageSearchCriteria) -> Vec<Package> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for repo in &self.repositories {
            for pkg in repo.find_packages(criteria) {
                if seen.insert(pkg.name.clone()) {
                    out.push(pkg);
                }
            }
        }
        out
    }

    /// Get a package from the first repository that contains it.
    pub fn get_package(&self, name: &str, version: Option<&str>) -> Option<Package> {
        for repo in &self.repositories {
            if let Some(pkg) = repo.get_package(name, version) {
                return Some(pkg);
            }
        }
        None
    }

    /// Initialize all repositories (no-op for sync impls).
    pub fn initialize_all(&self) {
        for repo in &self.repositories {
            let _ = repo.is_initialized();
        }
    }

    /// Refresh all repositories (no-op for sync impls).
    pub fn refresh_all(&self) {
        for repo in &self.repositories {
            repo.refresh();
        }
    }
}

/// Build repositories from locations using plugin config.
pub fn scan_repositories(
    locations: &[PathBuf],
    cfg: Option<&config::Config>,
    cache: &mut Cache,
) -> Vec<Box<dyn PackageRepository>> {
    let plugin_config = plugins::PluginConfig::from_config(cfg);
    if !plugins::is_enabled(&plugin_config.package_repositories, "filesystem") {
        log::warn!("Filesystem repository is disabled via plugins.pkg_rs.package_repositories");
        return Vec::new();
    }

    locations
        .iter()
        .map(|location| Box::new(FilesystemRepository::scan(location, cache)) as Box<dyn PackageRepository>)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::Package;

    #[test]
    fn repository_metadata_and_stats() {
        let pkg1 = Package::new("maya".to_string(), "2026.0.0".to_string());
        let pkg2 = Package::new("maya".to_string(), "2025.0.0".to_string());
        let pkg3 = Package::new("redshift".to_string(), "3.5.0".to_string());
        let repo = MemoryRepository::new(vec![pkg1, pkg2, pkg3.clone()]);
        let meta = repo.metadata();
        assert_eq!(meta.repository_type, RepositoryType::Memory);
        assert!(repo.is_initialized());
        let stats = repo.get_stats();
        assert_eq!(stats.package_count, 3);
        assert_eq!(stats.version_count, 3);
        assert_eq!(repo.get_package_names(), vec!["maya", "redshift"]);
        let maya_versions = repo.get_package_versions("maya");
        assert_eq!(maya_versions.len(), 2);
        assert!(maya_versions.contains(&"2025.0.0".to_string()));
        assert!(maya_versions.contains(&"2026.0.0".to_string()));
        let latest = repo.get_package("maya", None).unwrap();
        assert_eq!(latest.version, "2026.0.0");
        let exact = repo.get_package("redshift", Some("3.5.0")).unwrap();
        assert_eq!(exact.name, "redshift-3.5.0");
        assert!(repo.package_exists("redshift", Some("3.5.0")));
        assert!(!repo.package_exists("redshift", Some("4.0.0")));
    }

    #[test]
    fn repository_find_packages() {
        let pkgs = vec![
            Package::new("maya".to_string(), "2026.0.0".to_string()),
            Package::new("maya".to_string(), "2025.0.0".to_string()),
            Package::new("redshift".to_string(), "3.5.0".to_string()),
        ];
        let repo = MemoryRepository::new(pkgs);
        let criteria = PackageSearchCriteria {
            name_pattern: Some("maya*".to_string()),
            ..Default::default()
        };
        let found = repo.find_packages(&criteria);
        assert_eq!(found.len(), 2);
        let limit = PackageSearchCriteria {
            name_pattern: None,
            limit: Some(2),
            ..Default::default()
        };
        let limited = repo.find_packages(&limit);
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn repository_manager() {
        let repo1 = MemoryRepository::new(vec![
            Package::new("maya".to_string(), "2026.0.0".to_string()),
        ]);
        let repo2 = MemoryRepository::new(vec![
            Package::new("redshift".to_string(), "3.5.0".to_string()),
        ]);
        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(repo1));
        manager.add_repository(Box::new(repo2));
        assert_eq!(manager.repository_count(), 2);
        let pkg = manager.get_package("maya", None).unwrap();
        assert_eq!(pkg.name, "maya-2026.0.0");
        let pkg2 = manager.get_package("redshift", None).unwrap();
        assert_eq!(pkg2.name, "redshift-3.5.0");
        let criteria = PackageSearchCriteria::default();
        let all = manager.find_packages(&criteria);
        assert_eq!(all.len(), 2);
    }
}
