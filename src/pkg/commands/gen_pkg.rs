//! Generate package.py template command.

use log::{error, info, warn};
use pkg_lib::name::PackageId;
use std::path::Path;
use std::process::ExitCode;

/// Generate package.py template for given package identifier.
pub fn cmd_gen_pkg(package_id: &str) -> ExitCode {
    // Parse package ID
    let pkg_id = match PackageId::parse(package_id) {
        Some(id) => id,
        None => {
            error!(
                "Invalid package ID: '{}'. Expected format: name-version[-variant]",
                package_id
            );
            error!("Examples: maya-2026.1.0, my-plugin-1.0.0-win64");
            return ExitCode::FAILURE;
        }
    };

    info!(
        "Generating package.py for: name='{}', version={:?}, variant={:?}",
        pkg_id.name, pkg_id.version(), pkg_id.variant
    );

    // Check if package.py already exists
    let target_path = Path::new("package.py");
    if target_path.exists() {
        warn!("package.py already exists in current directory. Not overwriting.");
        return ExitCode::FAILURE;
    }

    // Generate template
    let template = generate_template(&pkg_id);

    // Write to file
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

/// Generate full package.py template with all fields.
fn generate_template(pkg_id: &PackageId) -> String {
    let variant_line = match &pkg_id.variant {
        Some(v) => format!("variant = \"{}\"", v),
        None => "# variant = \"\"  # Optional: win64, linux, py310, etc.".to_string(),
    };

    format!(
        r##"# -*- coding: utf-8 -*-
"""
Package definition for {name} {version}.

This file defines the package metadata, dependencies, environment variables,
and applications provided by this package.
"""

# =============================================================================
# Required fields
# =============================================================================

name = "{name}"
version = "{version}"
{variant}

# =============================================================================
# Package metadata
# =============================================================================

description = ""  # Short description of the package
authors = []      # List of authors: ["Name <email>"]
tags = []         # Tags for filtering: ["dcc", "maya", "plugin"]

# =============================================================================
# Dependencies
# =============================================================================

# Package requirements (supports version constraints)
# Examples:
#   "maya"           - any version
#   "maya-2026"      - exact version
#   "maya>=2024"     - version 2024 or higher
#   "maya>=2024,<2027" - version range
requires = []

# Build-time only dependencies (not propagated)
build_requires = []

# Optional features that can be enabled
# variants = ["py39", "py310", "py311"]

# =============================================================================
# Environment variables
# =============================================================================

# Environment modifications applied when package is activated
# Supports: prepend, append, set operations
# Available tokens: {{root}}, {{name}}, {{version}}, {{variant}}

env = {{
    # "PATH": {{
    #     "prepend": ["{{root}}/bin"],
    # }},
    # "PYTHONPATH": {{
    #     "prepend": ["{{root}}/python"],
    # }},
    # "{name_upper}_ROOT": {{
    #     "set": "{{root}}",
    # }},
}}

# Platform-specific environment (optional)
# env_win = {{}}
# env_linux = {{}}
# env_macos = {{}}

# =============================================================================
# Applications (executables provided by this package)
# =============================================================================

# apps = {{
#     "{name}": {{
#         "path": "{{root}}/bin/{name}.exe",  # Use .exe on Windows
#         "args": [],                          # Default arguments
#         "env": {{}},                          # App-specific env overrides
#         "properties": {{
#             "console": True,                 # Show console window
#             "cwd": "{{root}}",                # Working directory
#         }},
#     }},
# }}

# =============================================================================
# Hooks (lifecycle callbacks)
# =============================================================================

# def pre_activate():
#     """Called before environment is activated."""
#     pass

# def post_activate():
#     """Called after environment is activated."""
#     pass

# def pre_build():
#     """Called before package is built."""
#     pass

# def post_build():
#     """Called after package is built."""
#     pass
"##,
        name = pkg_id.name,
        version = pkg_id.version().unwrap_or_else(|| "0.0.0".to_string()),
        variant = variant_line,
        name_upper = pkg_id.name.to_uppercase().replace('-', "_"),
    )
}