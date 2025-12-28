//! Package cache for faster rescanning.
//!
//! Stores parsed packages with mtime for invalidation.
//! Cache file is located next to the binary (pkg.cache).

use crate::package::Package;
use log::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache entry for a single package.py file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Modification time (seconds since UNIX epoch).
    pub mtime: u64,
    /// Parsed package data.
    pub package: Package,
}

/// Package cache.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cache {
    /// Entries indexed by package.py path.
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl Cache {
    /// Create empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get cache file path (next to binary).
    pub fn cache_path() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("pkg.cache")))
    }

    /// Load cache from disk.
    pub fn load() -> Self {
        let Some(path) = Self::cache_path() else {
            debug!("Cache: no cache path available");
            return Self::new();
        };

        if !path.exists() {
            debug!("Cache: no cache file at {}", path.display());
            return Self::new();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(cache) => {
                    info!("Cache: loaded from {}", path.display());
                    cache
                }
                Err(e) => {
                    warn!("Cache: parse error, starting fresh: {}", e);
                    Self::new()
                }
            },
            Err(e) => {
                warn!("Cache: read error, starting fresh: {}", e);
                Self::new()
            }
        }
    }

    /// Save cache to disk.
    pub fn save(&self) {
        let Some(path) = Self::cache_path() else {
            debug!("Cache: no cache path available");
            return;
        };

        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    warn!("Cache: write error: {}", e);
                } else {
                    info!("Cache: saved {} entries to {}", self.entries.len(), path.display());
                }
            }
            Err(e) => {
                warn!("Cache: serialize error: {}", e);
            }
        }
    }

    /// Get cached package if still valid (mtime matches).
    pub fn get(&self, path: &Path) -> Option<&Package> {
        let entry = self.entries.get(path)?;
        let current_mtime = get_mtime(path)?;

        if entry.mtime == current_mtime {
            trace!("Cache: hit for {}", path.display());
            Some(&entry.package)
        } else {
            trace!("Cache: stale for {} (cached={}, current={})", 
                   path.display(), entry.mtime, current_mtime);
            None
        }
    }

    /// Insert or update cache entry.
    pub fn insert(&mut self, path: PathBuf, package: Package) {
        if let Some(mtime) = get_mtime(&path) {
            trace!("Cache: storing {} (mtime={})", path.display(), mtime);
            self.entries.insert(path, CacheEntry { mtime, package });
        }
    }

    /// Remove stale entries (files that no longer exist).
    pub fn prune(&mut self) {
        let before = self.entries.len();
        self.entries.retain(|path, _| path.exists());
        let removed = before - self.entries.len();
        if removed > 0 {
            debug!("Cache: pruned {} stale entries", removed);
        }
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Get file modification time as seconds since UNIX epoch.
fn get_mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_basic() {
        let mut cache = Cache::new();
        assert!(cache.is_empty());

        let pkg = Package::new("test".to_string(), "1.0.0".to_string());
        cache.entries.insert(
            PathBuf::from("/fake/path"),
            CacheEntry { mtime: 12345, package: pkg },
        );

        assert_eq!(cache.len(), 1);
    }
}
