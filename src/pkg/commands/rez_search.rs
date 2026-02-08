//! rez search command (native).

use super::common::matches_glob;
use crate::cli::SearchArgs;
use pkg_lib::{Package, Storage};
use std::process::ExitCode;

/// List/search packages with optional filtering.
pub fn cmd_rez_search(storage: &Storage, args: &SearchArgs) -> ExitCode {
    let all_packages = storage.packages();
    let mut packages: Vec<&Package> = all_packages.iter().collect();

    // Filter by glob patterns (OR logic: any pattern matches)
    if !args.patterns.is_empty() {
        packages.retain(|p| {
            args.patterns.iter().any(|pat| {
                matches_glob(pat, &p.base) || matches_glob(pat, &p.name)
            })
        });
    }

    // Filter by tags (all specified tags must be present)
    if !args.tags.is_empty() {
        packages.retain(|p| args.tags.iter().all(|t| p.tags.contains(t)));
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    // Only latest versions
    if args.latest {
        let mut seen = std::collections::HashSet::new();
        packages.retain(|p| seen.insert(p.base.clone()));
    }

    if args.json {
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        println!("{}", serde_json::to_string_pretty(&names).unwrap_or_default());
    } else {
        if packages.is_empty() {
            println!("No packages found.");
        } else {
            println!("Available packages ({}):", packages.len());
            for pkg in packages {
                println!("  {} ({})", pkg.name, pkg.base);
            }
        }
    }

    ExitCode::SUCCESS
}
