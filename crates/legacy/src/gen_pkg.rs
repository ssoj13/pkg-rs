//! Generate package.py template (legacy implementation).

use log::{error, info, warn};
use pkg_lib::package_name::PackageName;
use std::path::Path;
use std::process::ExitCode;

/// Generate package.py template for given package identifier.
pub fn cmd_gen_pkg(package_id: &str) -> ExitCode {
    let pkg_id = match PackageName::parse(package_id) {
        Ok(id) => id,
        Err(_) => {
            error!(
                "Invalid package ID: '{}'. Expected format: name-version[--variant]",
                package_id
            );
            error!("Examples: maya-2026.1.0, my-plugin-1.0.0, maya-2026.1.0--win64");
            return ExitCode::FAILURE;
        }
    };

    info!(
        "Generating package.py for: name='{}', version={:?}, variant={:?}",
        pkg_id.base(),
        pkg_id.version_string(),
        pkg_id.variant()
    );

    let target_path = Path::new("package.py");
    if target_path.exists() {
        warn!("package.py already exists in current directory. Not overwriting.");
        return ExitCode::FAILURE;
    }

    let template = generate_template(&pkg_id);

    match std::fs::write(target_path, &template) {
        Ok(()) => {
            info!("Created package.py");
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("Failed to write package.py: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn generate_template(pkg_id: &PackageName) -> String {
    let variant_line = match pkg_id.variant() {
        Some(v) => format!("variant = \"{}\"", v),
        None => "# variant = \"\"  # Optional: win64, linux, py310, etc.".to_string(),
    };

    format!(
        r##"# -*- coding: utf-8 -*-
"""
Package definition for {name} {version}.
"""

name = "{name}"
version = "{version}"
{variant}

description = ""
authors = []
tags = []
requires = []
build_requires = []

env = {{
}}

# apps = {{}}
"##,
        name = pkg_id.base(),
        version = pkg_id.version_string().unwrap_or_else(|| "0.0.0".to_string()),
        variant = variant_line,
    )
}
