//! List packages command.

use pkg_lib::{Package, Storage};
use std::process::ExitCode;

/// Simple glob matching with * and ? wildcards (case-insensitive).
pub fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();
    matches_glob_impl(pattern.as_bytes(), text.as_bytes())
}

fn matches_glob_impl(pattern: &[u8], text: &[u8]) -> bool {
    let mut p = 0;
    let mut t = 0;
    let mut star_p = None;
    let mut star_t = 0;

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star_p = Some(p);
            star_t = t;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_t += 1;
            t = star_t;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }

    p == pattern.len()
}

/// List packages with optional filtering.
pub fn cmd_list(
    storage: &Storage,
    patterns: Vec<String>,
    tags: Vec<String>,
    latest: bool,
    json: bool,
) -> ExitCode {
    let all_packages = storage.packages();
    let mut packages: Vec<&Package> = all_packages.iter().collect();

    // Filter by glob patterns (OR logic: any pattern matches)
    if !patterns.is_empty() {
        packages.retain(|p| {
            patterns.iter().any(|pat| {
                matches_glob(pat, &p.base) || matches_glob(pat, &p.name)
            })
        });
    }

    // Filter by tags (all specified tags must be present)
    if !tags.is_empty() {
        packages.retain(|p| tags.iter().all(|t| p.tags.contains(t)));
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    // Only latest versions
    if latest {
        let mut seen = std::collections::HashSet::new();
        packages.retain(|p| seen.insert(p.base.clone()));
    }

    if json {
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
