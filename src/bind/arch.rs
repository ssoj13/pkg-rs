//! Bind module: arch (processor architecture).

use super::{common, BindHandler};
use std::path::Path;

pub struct ArchHandler;

impl BindHandler for ArchHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let version = common::detect_arch();
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        common::write_basic_package(path, name, &version)
    }
}
