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

/// Save a toolset definition to a TOML file.
///
/// If the file exists, updates/adds the toolset section.
/// If the file doesn't exist, creates it with just this toolset.
///
/// # Arguments
/// * `path` - Path to .toml file
/// * `name` - Toolset name (becomes TOML section)
/// * `def` - Toolset definition
///
/// # Example
/// ```ignore
/// let def = ToolsetDef {
///     version: "1.0.0".to_string(),
///     description: Some("Maya with Redshift".to_string()),
///     requires: vec!["maya@2026".to_string(), "redshift@>=3.5".to_string()],
///     tags: vec!["dcc".to_string()],
/// };
/// save_toolset(Path::new("studio.toml"), "maya-full", &def)?;
/// ```
pub fn save_toolset(path: &Path, name: &str, def: &ToolsetDef) -> Result<(), String> {
    use std::fs;
    use toml_edit::{DocumentMut, Item, Array, value};

    debug!("Saving toolset '{}' to {:?}", name, path);

    // Load existing file or create empty document
    let mut doc: DocumentMut = if path.exists() {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
        content.parse::<DocumentMut>()
            .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?
    } else {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
        }
        DocumentMut::new()
    };

    // Create or update the toolset section
    let table = doc[name].or_insert(toml_edit::table());
    if let Item::Table(t) = table {
        t.insert("version", value(&def.version));
        
        if let Some(desc) = &def.description {
            t.insert("description", value(desc));
        } else {
            t.remove("description");
        }

        // Requires array
        let mut reqs = Array::new();
        for r in &def.requires {
            reqs.push(r.as_str());
        }
        t.insert("requires", value(reqs));

        // Tags array (only if non-empty)
        if !def.tags.is_empty() {
            let mut tags = Array::new();
            for tag in &def.tags {
                tags.push(tag.as_str());
            }
            t.insert("tags", value(tags));
        } else {
            t.remove("tags");
        }
    }

    // Write back
    fs::write(path, doc.to_string())
        .map_err(|e| format!("Failed to write {:?}: {}", path, e))?;

    debug!("Saved toolset '{}' to {:?}", name, path);
    Ok(())
}

/// Delete a toolset from a TOML file.
///
/// Removes the section with the given name.
/// Returns Ok(true) if deleted, Ok(false) if not found.
pub fn delete_toolset(path: &Path, name: &str) -> Result<bool, String> {
    use std::fs;
    use toml_edit::DocumentMut;

    if !path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    let mut doc: DocumentMut = content.parse()
        .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

    if doc.contains_key(name) {
        doc.remove(name);
        fs::write(path, doc.to_string())
            .map_err(|e| format!("Failed to write {:?}: {}", path, e))?;
        debug!("Deleted toolset '{}' from {:?}", name, path);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get default user toolsets directory.
///
/// Returns ~/.pkg-rs/packages/.toolsets/
pub fn user_toolsets_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".pkg-rs").join("packages").join(".toolsets"))
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

    #[test]
    fn test_save_toolset() {
        let temp = TempDir::new().unwrap();
        let toml_path = temp.path().join("test.toml");

        // Save new toolset
        let def = ToolsetDef {
            version: "1.0.0".to_string(),
            description: Some("Test toolset".to_string()),
            requires: vec!["maya@2026".to_string(), "redshift@>=3.5".to_string()],
            tags: vec!["dcc".to_string()],
        };
        save_toolset(&toml_path, "my-toolset", &def).unwrap();

        // Verify file exists and can be parsed
        let content = std::fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("[my-toolset]"));
        assert!(content.contains("version = \"1.0.0\""));
        assert!(content.contains("maya@2026"));

        // Add another toolset to same file
        let def2 = ToolsetDef {
            version: "2.0.0".to_string(),
            description: None,
            requires: vec!["houdini@21".to_string()],
            tags: vec![],
        };
        save_toolset(&toml_path, "houdini-env", &def2).unwrap();

        // Parse and verify both exist
        let toolsets = parse_toolsets_file(&toml_path).unwrap();
        assert_eq!(toolsets.len(), 2);
        assert!(toolsets.contains_key("my-toolset"));
        assert!(toolsets.contains_key("houdini-env"));
    }

    #[test]
    fn test_delete_toolset() {
        let temp = TempDir::new().unwrap();
        let toml_path = temp.path().join("test.toml");

        // Create file with two toolsets
        let def = ToolsetDef {
            version: "1.0.0".to_string(),
            description: None,
            requires: vec!["maya@2026".to_string()],
            tags: vec![],
        };
        save_toolset(&toml_path, "toolset-a", &def).unwrap();
        save_toolset(&toml_path, "toolset-b", &def).unwrap();

        // Delete one
        let deleted = delete_toolset(&toml_path, "toolset-a").unwrap();
        assert!(deleted);

        // Verify only one remains
        let toolsets = parse_toolsets_file(&toml_path).unwrap();
        assert_eq!(toolsets.len(), 1);
        assert!(!toolsets.contains_key("toolset-a"));
        assert!(toolsets.contains_key("toolset-b"));

        // Delete non-existent
        let deleted = delete_toolset(&toml_path, "not-exists").unwrap();
        assert!(!deleted);
    }
}
