//! Filesystem package repository.

use super::{with_package_loading, PackageRepository};
use crate::cache::Cache;
use crate::loader::Loader;
use crate::package::Package;
use crate::repo_ops::IGNORE_PREFIX;
use jwalk::WalkDir;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FilesystemRepository {
    location: PathBuf,
    packages: Vec<Package>,
    warnings: Vec<String>,
}

impl FilesystemRepository {
    pub fn scan(location: &Path, cache: &mut Cache) -> Self {
        let mut warnings = Vec::new();
        let mut packages = HashMap::new();

        if !location.exists() {
            return Self {
                location: location.to_path_buf(),
                packages: Vec::new(),
                warnings,
            };
        }

        let _ = pyo3::Python::initialize();
        let mut loader = Loader::new(Some(false));

        let package_files: Vec<PathBuf> = WalkDir::new(location)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.file_name().to_string_lossy() == "package.py")
            .map(|e| e.path())
            .collect();

        for path in package_files {
            if is_ignored_package(&path) {
                continue;
            }

            if let Some(pkg) = cache.get(&path) {
                if packages.contains_key(&pkg.name) {
                    warnings.push(format!(
                        "Duplicate package '{}' in {} (first wins)",
                        pkg.name,
                        path.display()
                    ));
                    continue;
                }
                packages.insert(pkg.name.clone(), pkg);
                continue;
            }

            let result = with_package_loading(|| loader.load_path(&path));
            match result {
                Ok(mut pkg) => {
                    pkg.package_source = Some(path.to_string_lossy().to_string());
                    cache.insert(path.clone(), pkg.clone());
                    if packages.contains_key(&pkg.name) {
                        warnings.push(format!(
                            "Duplicate package '{}' in {} (first wins)",
                            pkg.name,
                            path.display()
                        ));
                    } else {
                        packages.insert(pkg.name.clone(), pkg);
                    }
                }
                Err(e) => {
                    warnings.push(format!("Failed to load {}: {}", path.display(), e));
                }
            }
        }

        Self {
            location: location.to_path_buf(),
            packages: packages.into_values().collect(),
            warnings,
        }
    }

    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

impl PackageRepository for FilesystemRepository {
    fn repo_type(&self) -> &'static str {
        "filesystem"
    }

    fn location(&self) -> &PathBuf {
        &self.location
    }

    fn packages(&self) -> &[Package] {
        &self.packages
    }

    fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

fn is_ignored_package(path: &Path) -> bool {
    let Some(version_dir) = path.parent() else {
        return false;
    };
    let Some(family_dir) = version_dir.parent() else {
        return false;
    };

    let Some(version) = version_dir.file_name().and_then(|s| s.to_str()) else {
        return false;
    };

    let ignore_file = family_dir.join(format!("{}{}", IGNORE_PREFIX, version));
    ignore_file.exists()
}
