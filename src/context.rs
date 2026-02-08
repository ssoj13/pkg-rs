//! Resolved context (rez-next–style): result of a solve with env and .rxt serialization.

use crate::package::Package;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Context status after resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextStatus {
    Success,
    Failed,
}

/// Resolved context: packages + environment + metadata (rez-next–style).
///
/// Built after solver runs; holds resolved packages and merged environment.
/// Serializes to .rxt (JSON) for `pkg context` / suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContext {
    /// Unique context id (e.g. UUID or timestamp-based).
    pub id: String,
    /// Optional context name.
    pub name: Option<String>,
    /// Original requirement strings that led to this resolution.
    pub requirements: Vec<String>,
    /// Resolved packages (order: dependency order).
    pub resolved_packages: Vec<Package>,
    /// Merged environment variables (after expand).
    pub environment_vars: HashMap<String, String>,
    /// Extra metadata.
    pub metadata: HashMap<String, String>,
    /// Creation time (Unix timestamp).
    pub created_at: i64,
    /// Suite name if part of a suite.
    pub suite: Option<String>,
    /// Platform (e.g. windows, linux).
    pub platform: Option<String>,
    /// Architecture (e.g. x86_64).
    pub arch: Option<String>,
    /// Resolution status.
    pub status: ContextStatus,
    /// Failure message if status is Failed.
    pub failure_description: Option<String>,
}

impl ResolvedContext {
    /// Build a successful context from resolved package and merged env map.
    pub fn from_resolved(
        requirements: Vec<String>,
        resolved_packages: Vec<Package>,
        environment_vars: HashMap<String, String>,
    ) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let id = format!("ctx_{}", created_at);
        Self {
            id,
            name: None,
            requirements,
            resolved_packages,
            environment_vars,
            metadata: HashMap::new(),
            created_at,
            suite: None,
            platform: Some(std::env::consts::OS.to_string()),
            arch: Some(std::env::consts::ARCH.to_string()),
            status: ContextStatus::Success,
            failure_description: None,
        }
    }

    /// Build a failed context (e.g. solve error).
    pub fn failed(requirements: Vec<String>, failure_description: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            id: format!("ctx_{}", created_at),
            name: None,
            requirements,
            resolved_packages: Vec::new(),
            environment_vars: HashMap::new(),
            metadata: HashMap::new(),
            created_at,
            suite: None,
            platform: Some(std::env::consts::OS.to_string()),
            arch: Some(std::env::consts::ARCH.to_string()),
            status: ContextStatus::Failed,
            failure_description: Some(failure_description),
        }
    }

    pub fn get_package_names(&self) -> Vec<String> {
        self.resolved_packages.iter().map(|p| p.name.clone()).collect()
    }

    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.resolved_packages.iter().find(|p| p.name == name || p.base == name)
    }

    pub fn has_package(&self, name: &str) -> bool {
        self.get_package(name).is_some()
    }

    /// Get merged environment (reference).
    pub fn get_environ(&self) -> &HashMap<String, String> {
        &self.environment_vars
    }

    /// Tools (app names -> path) from all resolved packages.
    pub fn get_tools(&self) -> HashMap<String, PathBuf> {
        let mut tools = HashMap::new();
        for pkg in &self.resolved_packages {
            for app in &pkg.apps {
                if let Some(ref path) = app.path {
                    tools.insert(app.name.clone(), PathBuf::from(path));
                }
            }
        }
        tools
    }

    /// Serialize to .rxt JSON bytes.
    pub fn to_rxt_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(self)
    }

    /// Serialize to .rxt JSON string.
    pub fn to_rxt_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write context to a .rxt file.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = self.to_rxt_string().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    /// Load context from a .rxt file.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = std::fs::read_to_string(path)?;
        let ctx: ResolvedContext = serde_json::from_str(&content)?;
        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::Package;

    #[test]
    fn resolved_context_roundtrip() {
        let pkg = Package::new("maya".to_string(), "2026.0.0".to_string());
        let mut env_vars = HashMap::new();
        env_vars.insert("PATH".to_string(), "/opt/maya/bin".to_string());
        let ctx = ResolvedContext::from_resolved(
            vec!["maya".to_string()],
            vec![pkg],
            env_vars,
        );
        let json = ctx.to_rxt_string().unwrap();
        let loaded: ResolvedContext = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.id, ctx.id);
        assert_eq!(loaded.get_package_names().len(), 1);
        assert_eq!(loaded.get_environ().get("PATH"), Some(&"/opt/maya/bin".to_string()));
    }
}
