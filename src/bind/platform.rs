//! Bind module: platform (OS name).

use super::{common, BindHandler};
use std::path::Path;

pub struct PlatformHandler;

impl BindHandler for PlatformHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let version = std::env::consts::OS.to_string();
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        common::write_basic_package(path, name, &version)
    }
}
