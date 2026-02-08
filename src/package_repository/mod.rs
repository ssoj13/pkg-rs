//! Package repository abstractions (Rez-style).

pub mod filesystem;
pub mod memory;

use crate::cache::Cache;
use crate::config;
use crate::package::Package;
use crate::plugins;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub use filesystem::FilesystemRepository;
pub use memory::MemoryRepository;

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

/// Package repository trait (minimal Rez parity).
pub trait PackageRepository {
    fn repo_type(&self) -> &'static str;
    fn location(&self) -> &PathBuf;
    fn packages(&self) -> &[Package];
    fn warnings(&self) -> &[String];

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
}

/// Repository stats (Rez-like).
#[derive(Debug, Clone, Copy, Default)]
pub struct RepositoryStats {
    pub package_load_time: f64,
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
