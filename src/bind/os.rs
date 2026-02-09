//! Bind module: os (OS + version, e.g. windows-10.0.19041, osx-14.0).

use super::{common, BindHandler};
use std::path::Path;

pub struct OsHandler;

impl BindHandler for OsHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let version = common::detect_os();
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        common::write_basic_package(path, name, &version)
    }
}
