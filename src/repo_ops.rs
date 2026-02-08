//! Filesystem repository operations for Rez-style commands.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Rez-style ignore file prefix (matches `.ignore<version>`).
pub const IGNORE_PREFIX: &str = ".ignore";

pub fn package_dir(repo: &Path, name: &str, version: &str) -> PathBuf {
    repo.join(name).join(version)
}

pub fn ignore_marker_path(repo: &Path, name: &str, version: &str) -> PathBuf {
    repo.join(name).join(format!("{}{}", IGNORE_PREFIX, version))
}

pub fn package_exists(repo: &Path, name: &str, version: &str) -> bool {
    package_dir(repo, name, version).is_dir()
}

pub fn ignore_package(
    repo: &Path,
    name: &str,
    version: &str,
    allow_missing: bool,
) -> Result<i32, String> {
    let pkg_dir = package_dir(repo, name, version);
    if !pkg_dir.exists() && !allow_missing {
        return Ok(-1);
    }

    let ignore_file = ignore_marker_path(repo, name, version);
    if ignore_file.exists() {
        return Ok(0);
    }

    if let Some(parent) = ignore_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::write(&ignore_file, "").map_err(|e| e.to_string())?;
    Ok(1)
}

pub fn unignore_package(repo: &Path, name: &str, version: &str) -> Result<i32, String> {
    let ignore_file = ignore_marker_path(repo, name, version);
    let pkg_exists = package_exists(repo, name, version);

    let mut removed = false;
    if ignore_file.exists() {
        std::fs::remove_file(&ignore_file).map_err(|e| e.to_string())?;
        removed = true;
    }

    if pkg_exists {
        if removed {
            Ok(1)
        } else {
            Ok(0)
        }
    } else {
        Ok(-1)
    }
}

pub fn remove_package(repo: &Path, name: &str, version: &str) -> Result<bool, String> {
    let pkg_dir = package_dir(repo, name, version);
    if !pkg_dir.exists() {
        return Ok(false);
    }

    let _ = ignore_package(repo, name, version, true);
    std::fs::remove_dir_all(&pkg_dir).map_err(|e| e.to_string())?;
    let _ = unignore_package(repo, name, version);
    Ok(true)
}

pub fn remove_package_family(repo: &Path, name: &str, force: bool) -> Result<bool, String> {
    let family_dir = repo.join(name);
    if !family_dir.exists() {
        return Ok(false);
    }

    if !force {
        let mut has_entries = false;
        if let Ok(entries) = std::fs::read_dir(&family_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if file_name.starts_with('.') && file_name != "." && file_name != ".." {
                    // Treat ignore files as non-empty too.
                    has_entries = true;
                    break;
                }
                has_entries = true;
                break;
            }
        }
        if has_entries {
            return Err("Package family is not empty".to_string());
        }
    }

    std::fs::remove_dir_all(&family_dir).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn remove_ignored_since(
    repo: &Path,
    days: i64,
    dry_run: bool,
    verbose: bool,
) -> Result<usize, String> {
    let mut removed = 0usize;
    let now = SystemTime::now();

    let Ok(entries) = std::fs::read_dir(repo) else {
        return Ok(0);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let family_name = entry.file_name().to_string_lossy().to_string();
        let Ok(fam_entries) = std::fs::read_dir(&path) else {
            continue;
        };

        for fam_entry in fam_entries.flatten() {
            let name = fam_entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(IGNORE_PREFIX) {
                continue;
            }

            let ver_str = name[IGNORE_PREFIX.len()..].to_string();
            let meta = match fam_entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let timestamp = meta
                .created()
                .or_else(|_| meta.modified())
                .unwrap_or(UNIX_EPOCH);
            let age_secs = now
                .duration_since(timestamp)
                .unwrap_or_default()
                .as_secs_f64();
            let age_days = age_secs / 86_400.0;

            if age_days < days as f64 {
                continue;
            }

            if dry_run {
                if verbose {
                    eprintln!(
                        "Would remove {}-{} from {}",
                        family_name,
                        ver_str,
                        repo.display()
                    );
                }
                removed += 1;
                continue;
            }

            if remove_package(repo, &family_name, &ver_str)? {
                removed += 1;
                if verbose {
                    eprintln!(
                        "Removed {}-{} from {}",
                        family_name,
                        ver_str,
                        repo.display()
                    );
                }
            }
        }
    }

    Ok(removed)
}

pub fn repo_is_empty(repo: &Path) -> Result<bool, String> {
    if !repo.exists() {
        return Ok(true);
    }

    let mut walker = jwalk::WalkDir::new(repo).into_iter();
    while let Some(entry) = walker.next() {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name().to_string_lossy() == "package.py" {
            return Ok(false);
        }
    }

    Ok(true)
}
