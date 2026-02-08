//! rez pkg-cache command (native, package metadata cache).

use crate::cli::PkgCacheArgs;
use pkg_lib::cache::Cache;
use std::process::ExitCode;

pub fn cmd_rez_pkg_cache(args: &PkgCacheArgs) -> ExitCode {
    let Some(path) = Cache::cache_path() else {
        eprintln!("rez pkg-cache: no cache path available");
        return ExitCode::FAILURE;
    };

    if args.clear {
        if path.exists() {
            if let Err(err) = std::fs::remove_file(&path) {
                eprintln!("rez pkg-cache: {}", err);
                return ExitCode::FAILURE;
            }
            println!("Cleared cache: {}", path.display());
        } else {
            println!("Cache file does not exist: {}", path.display());
        }
        return ExitCode::SUCCESS;
    }

    let cache = Cache::load();

    if args.stats || (!args.stats && !args.list) {
        println!("Cache path: {}", path.display());
        println!("Entries: {}", cache.len());
    }

    if args.list {
        if cache.entries.is_empty() {
            println!("No cached packages.");
            return ExitCode::SUCCESS;
        }

        println!("Cached package entries:");
        let mut items: Vec<_> = cache.entries.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        for (path, entry) in items {
            println!("  {} (mtime={})", path.display(), entry.mtime);
        }
    }

    ExitCode::SUCCESS
}
