//! Toolset definitions from TOML files.
//!
//! Toolsets are virtual packages defined in `.toolsets/*.toml` files.
//! Each section in a TOML file becomes a Package with requirements.
//!
//! # File Format
//!
//! ```toml
//! # .toolsets/studio.toml
//!
//! [maya-2026-full]
//! version = "1.0.0"
//! description = "Maya 2026 with Redshift"
//! requires = [
//!     "maya@2026.0",
//!     "redshift@>=3.5",
//!     "maya-bonus-tools"
//! ]
//!
//! [houdini-fx]
//! version = "2.0.0"
//! requires = ["houdini@21.0", "redshift@>=3.5"]
//! ```
//!
//! # Usage
//!
//! Toolsets are automatically loaded by Storage when scanning locations.
//! They appear as regular packages and can be used with `pkg run`, `pkg env`, etc.

use crate::package::Package;
use log::{debug, trace, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Single toolset definition from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsetDef {
    /// Version (default: "1.0.0")
    #[serde(default = "default_version")]
    pub version: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Required packages (e.g., `["maya@2026.0", "redshift@>=3.5"]`)
    #[serde(default)]
    pub requires: Vec<String>,

    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Parse a single .toml file containing multiple toolset definitions.
/// Returns a HashMap where key = toolset name (section name), value = ToolsetDef.
pub fn parse_toolsets_file(path: &Path) -> Result<HashMap<String, ToolsetDef>, String> {
    trace!("Parsing toolsets file: {:?}", path);
    
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    
    let toolsets: HashMap<String, ToolsetDef> = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;
    
    debug!("Parsed {} toolsets from {:?}", toolsets.len(), path);
    Ok(toolsets)
}

/// Convert ToolsetDef to Package.
/// The toolset name becomes the package base name.
pub fn toolset_to_package(name: &str, def: &ToolsetDef) -> Package {
    let mut pkg = Package::new(name.to_string(), def.version.clone());
    
    // Add requirements
    for req in &def.requires {
        pkg.add_req(req.clone());
    }
    
    // Add tags
    for tag in &def.tags {
        pkg.add_tag(tag.clone());
    }
    
    // Add "toolset" tag to identify it
    pkg.add_tag("toolset".to_string());
    
    trace!("Created toolset package: {} with {} reqs", pkg.name, pkg.reqs.len());
    pkg
}

/// Scan a directory for .toolsets subdirectory and load all toolsets.
/// Returns a list of Packages created from toolset definitions.
pub fn scan_toolsets_dir(location: &Path) -> Vec<Package> {
    let toolsets_dir = location.join(".toolsets");
    
    if !toolsets_dir.exists() || !toolsets_dir.is_dir() {
        return Vec::new();
    }
    
    debug!("Scanning toolsets directory: {:?}", toolsets_dir);
    
    let mut packages = Vec::new();
    
    // Read all .toml files in .toolsets directory
    let entries = match std::fs::read_dir(&toolsets_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read toolsets directory {:?}: {}", toolsets_dir, e);
            return Vec::new();
        }
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        // Skip non-toml files
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        
        // Parse the file
        match parse_toolsets_file(&path) {
            Ok(toolsets) => {
                for (name, def) in toolsets {
                    let pkg = toolset_to_package(&name, &def);
                    packages.push(pkg);
                }
            }
            Err(e) => {
                warn!("Failed to parse toolsets file {:?}: {}", path, e);
            }
        }
    }
    
    debug!("Found {} toolset packages in {:?}", packages.len(), toolsets_dir);
    packages
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_toolset_toml() {
        let toml_content = r#"
[maya-2026-full]
version = "1.0.0"
description = "Maya with Redshift"
requires = ["maya@2026.0", "redshift@>=3.5"]

[houdini-fx]
requires = ["houdini@21.0"]
tags = ["dcc", "fx"]
"#;

        let toolsets: HashMap<String, ToolsetDef> = toml::from_str(toml_content).unwrap();
        
        assert_eq!(toolsets.len(), 2);
        
        let maya = &toolsets["maya-2026-full"];
        assert_eq!(maya.version, "1.0.0");
        assert_eq!(maya.requires.len(), 2);
        assert_eq!(maya.description, Some("Maya with Redshift".to_string()));
        
        let houdini = &toolsets["houdini-fx"];
        assert_eq!(houdini.version, "1.0.0"); // default
        assert_eq!(houdini.requires.len(), 1);
        assert_eq!(houdini.tags, vec!["dcc", "fx"]);
    }

    #[test]
    fn test_toolset_to_package() {
        let def = ToolsetDef {
            version: "2.0.0".to_string(),
            description: Some("Test toolset".to_string()),
            requires: vec!["maya@2026".to_string(), "redshift@3".to_string()],
            tags: vec!["vfx".to_string()],
        };
        
        let pkg = toolset_to_package("my-toolset", &def);
        
        assert_eq!(pkg.name, "my-toolset-2.0.0");
        assert_eq!(pkg.base, "my-toolset");
        assert_eq!(pkg.version, "2.0.0");
        assert_eq!(pkg.reqs.len(), 2);
        assert!(pkg.has_tag("toolset"));
        assert!(pkg.has_tag("vfx"));
    }

    #[test]
    fn test_scan_toolsets_dir() {
        let temp = TempDir::new().unwrap();
        let toolsets_dir = temp.path().join(".toolsets");
        std::fs::create_dir(&toolsets_dir).unwrap();
        
        // Create a test .toml file
        let toml_path = toolsets_dir.join("studio.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        writeln!(file, r#"
[maya-full]
version = "1.0.0"
requires = ["maya@2026"]

[houdini-full]
requires = ["houdini@21"]
"#).unwrap();
        
        let packages = scan_toolsets_dir(temp.path());
        
        assert_eq!(packages.len(), 2);
        assert!(packages.iter().any(|p| p.base == "maya-full"));
        assert!(packages.iter().any(|p| p.base == "houdini-full"));
    }
}
