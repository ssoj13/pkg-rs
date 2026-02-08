//! Scan locations command.

use log::{error, info, warn};
use pkg_lib::Storage;
use std::path::PathBuf;
use std::process::ExitCode;

/// Scan locations for packages and show statistics.
pub fn cmd_scan(paths: &[PathBuf]) -> ExitCode {
    let storage = if paths.is_empty() {
        Storage::scan_impl(None)
    } else {
        Storage::scan_impl(Some(paths))
    };

    match storage {
        Ok(storage) => {
            info!("Scanned locations:");
            for loc in storage.locations() {
                info!("  {}", loc);
            }

            info!("Found {} packages:", storage.count());
            for base in storage.bases() {
                let versions = storage.versions(&base);
                info!("  {} ({} versions)", base, versions.len());
            }

            if !storage.warnings.is_empty() {
                warn!("Warnings:");
                for w in &storage.warnings {
                    warn!("  - {}", w);
                }
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("Scan failed: {}", e);
            ExitCode::FAILURE
        }
    }
}
