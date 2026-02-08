//! Package cache for faster rescanning.
//!
//! Stores parsed packages with mtime for invalidation.
//! Cache file is located next to the binary (pkg.cache).

use crate::package::Package;
use log::{debug, info, trace, warn};
use moka::sync::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DEFAULT_CACHE_CAPACITY: u64 = 100_000;
const MIN_EXTRA_CAPACITY: u64 = 1_024;

/// Cache entry for a single package.py file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Modification time (seconds since UNIX epoch).
    pub mtime: u64,
    /// Parsed package data.
    pub package: Package,
}

/// Package cache.
#[derive(Debug, Clone)]
pub struct Cache {
    /// Entries indexed by package.py path.
    entries: MokaCache<PathBuf, CacheEntry>,
}

impl Cache {
    /// Create empty cache.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Create cache with explicit capacity.
    pub fn with_capacity(capacity: u64) -> Self {
        let entries = MokaCache::builder().max_capacity(capacity).build();
        Self { entries }
    }

    /// Create a cache from existing entries (for tests or reload).
    pub fn from_entries(entries: HashMap<PathBuf, CacheEntry>) -> Self {
        let capacity = Self::capacity_for_entries(entries.len());
        let cache = MokaCache::builder().max_capacity(capacity).build();
        for (path, entry) in entries {
            cache.insert(path, entry);
        }
        Self { entries: cache }
    }

    /// Snapshot current entries into a HashMap.
    pub fn entries_snapshot(&self) -> HashMap<PathBuf, CacheEntry> {
        let mut map = HashMap::new();
        for (key, value) in self.entries.iter() {
            map.insert((*key).clone(), value.clone());
        }
        map
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
            Ok(content) => match serde_json::from_str::<HashMap<PathBuf, CacheEntry>>(&content) {
                Ok(entries) => {
                    info!("Cache: loaded from {}", path.display());
                    Cache::from_entries(entries)
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

        let snapshot = self.entries_snapshot();
        match serde_json::to_string_pretty(&snapshot) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    warn!("Cache: write error: {}", e);
                } else {
                    info!("Cache: saved {} entries to {}", snapshot.len(), path.display());
                }
            }
            Err(e) => {
                warn!("Cache: serialize error: {}", e);
            }
        }
    }

    /// Get cached package if still valid (mtime matches).
    pub fn get(&self, path: &Path) -> Option<Package> {
        let entry = self.entries.get(path)?;
        let current_mtime = get_mtime(path)?;

        if entry.mtime == current_mtime {
            trace!("Cache: hit for {}", path.display());
            Some(entry.package.clone())
        } else {
            trace!(
                "Cache: stale for {} (cached={}, current={})",
                path.display(),
                entry.mtime,
                current_mtime
            );
            self.entries.invalidate(path);
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
        let mut stale = Vec::new();
        for (key, _value) in self.entries.iter() {
            if !key.exists() {
                stale.push((*key).clone());
            }
        }
        let removed = stale.len();
        for path in &stale {
            self.entries.invalidate(path);
        }
        if removed > 0 {
            debug!("Cache: pruned {} stale entries", removed);
        }
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.iter().count()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn capacity_for_entries(len: usize) -> u64 {
        let base = len as u64;
        let mut cap = base.saturating_add(MIN_EXTRA_CAPACITY);
        if cap < DEFAULT_CACHE_CAPACITY {
            cap = DEFAULT_CACHE_CAPACITY;
        }
        cap
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
        let mut entries = HashMap::new();
        let pkg = Package::new("test".to_string(), "1.0.0".to_string());
        entries.insert(
            PathBuf::from("/fake/path"),
            CacheEntry { mtime: 12345, package: pkg },
        );
        let cache = Cache::from_entries(entries);
        assert_eq!(cache.len(), 1);
    }
}
