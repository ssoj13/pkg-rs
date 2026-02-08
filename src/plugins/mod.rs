//! Plugin configuration and registries.
//!
//! This module provides a lightweight plugin configuration layer that mirrors
//! Rez's plugin enable/disable behavior. It does not dynamically load external
//! plugins yet; instead it controls which built-in plugins are active.

use crate::config;
use log::warn;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub build_systems: Vec<String>,
    pub build_processes: Vec<String>,
    pub release_hooks: Vec<String>,
    pub release_vcs: Vec<String>,
    pub shells: Vec<String>,
    pub package_filters: Vec<String>,
    pub package_orderers: Vec<String>,
    pub package_repositories: Vec<String>,
    pub commands: Vec<String>,
}

impl PluginConfig {
    pub fn from_config(cfg: Option<&config::Config>) -> Self {
        let cfg = Self {
            build_systems: list_from_config(cfg, "plugins.pkg_rs.build_systems", true),
            build_processes: list_from_config(cfg, "plugins.pkg_rs.build_processes", true),
            release_hooks: list_from_config(cfg, "plugins.pkg_rs.release_hooks", true),
            release_vcs: list_from_config(cfg, "plugins.pkg_rs.release_vcs", true),
            shells: list_from_config(cfg, "plugins.pkg_rs.shells", true),
            package_filters: list_from_config(cfg, "plugins.pkg_rs.package_filters", true),
            package_orderers: list_from_config(cfg, "plugins.pkg_rs.package_orderers", true),
            package_repositories: list_from_config(cfg, "plugins.pkg_rs.package_repositories", true),
            commands: list_from_config(cfg, "plugins.pkg_rs.commands", true),
        };

        registry().validate(&cfg);
        cfg
    }
}

pub fn list_from_json(value: &JsonValue) -> Vec<String> {
    match value {
        JsonValue::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        JsonValue::String(value) => value
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn is_enabled(list: &[String], name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return false;
    }

    list.iter().any(|item| {
        let item = item.trim().to_ascii_lowercase();
        if item.is_empty() {
            return false;
        }
        if item == "*" || item == "all" {
            return true;
        }
        item == name
    })
}

fn list_from_config(cfg: Option<&config::Config>, key: &str, default_all: bool) -> Vec<String> {
    let value = cfg.and_then(|c| config::get_json(c, key));
    if let Some(value) = value {
        let list = list_from_json(&value);
        if !list.is_empty() {
            return list;
        }
    }

    if default_all {
        vec!["*".to_string()]
    } else {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginType {
    BuildSystem,
    BuildProcess,
    ReleaseHook,
    ReleaseVcs,
    Shell,
    PackageFilter,
    PackageOrderer,
    PackageRepository,
    Command,
}

impl PluginType {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginType::BuildSystem => "build_system",
            PluginType::BuildProcess => "build_process",
            PluginType::ReleaseHook => "release_hook",
            PluginType::ReleaseVcs => "release_vcs",
            PluginType::Shell => "shell",
            PluginType::PackageFilter => "package_filter",
            PluginType::PackageOrderer => "package_orderer",
            PluginType::PackageRepository => "package_repository",
            PluginType::Command => "command",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginOrigin {
    Builtin,
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub plugin_type: PluginType,
    pub origin: PluginOrigin,
}

#[derive(Debug, Clone)]
pub struct PluginRegistry {
    plugins: HashMap<PluginType, Vec<PluginInfo>>,
}

impl PluginRegistry {
    fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
        };

        registry.register_builtin(PluginType::BuildSystem, &[
            "custom", "make", "cmake", "cargo", "python",
        ]);
        registry.register_builtin(PluginType::BuildProcess, &["local", "central"]);
        registry.register_builtin(PluginType::ReleaseHook, &["builtin"]);
        registry.register_builtin(PluginType::ReleaseVcs, &["git"]);
        registry.register_builtin(PluginType::Shell, &[
            "cmd", "powershell", "pwsh", "bash", "sh", "zsh", "csh", "tcsh", "gitbash",
        ]);
        registry.register_builtin(PluginType::PackageFilter, &["builtin"]);
        registry.register_builtin(PluginType::PackageOrderer, &["builtin"]);
        registry.register_builtin(PluginType::PackageRepository, &["filesystem"]);
        registry.register_builtin(PluginType::Command, &["builtin"]);

        registry
    }

    fn register_builtin(&mut self, plugin_type: PluginType, names: &[&str]) {
        let entries = self.plugins.entry(plugin_type).or_default();
        for name in names {
            entries.push(PluginInfo {
                name: name.to_string(),
                plugin_type,
                origin: PluginOrigin::Builtin,
            });
        }
    }

    pub fn list(&self, plugin_type: PluginType) -> Vec<String> {
        self.plugins
            .get(&plugin_type)
            .map(|items| items.iter().map(|i| i.name.clone()).collect())
            .unwrap_or_default()
    }

    pub fn is_known(&self, plugin_type: PluginType, name: &str) -> bool {
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty() {
            return false;
        }
        self.plugins
            .get(&plugin_type)
            .map(|items| {
                items
                    .iter()
                    .any(|info| info.name.eq_ignore_ascii_case(&name))
            })
            .unwrap_or(false)
    }

    pub fn validate(&self, config: &PluginConfig) {
        self.validate_list(PluginType::BuildSystem, &config.build_systems);
        self.validate_list(PluginType::BuildProcess, &config.build_processes);
        self.validate_list(PluginType::ReleaseHook, &config.release_hooks);
        self.validate_list(PluginType::ReleaseVcs, &config.release_vcs);
        self.validate_list(PluginType::Shell, &config.shells);
        self.validate_list(PluginType::PackageFilter, &config.package_filters);
        self.validate_list(PluginType::PackageOrderer, &config.package_orderers);
        self.validate_list(PluginType::PackageRepository, &config.package_repositories);
        self.validate_list(PluginType::Command, &config.commands);
    }

    fn validate_list(&self, plugin_type: PluginType, enabled: &[String]) {
        for name in enabled {
            let name = name.trim();
            if name.is_empty() || name == "*" || name.eq_ignore_ascii_case("all") {
                continue;
            }
            if !self.is_known(plugin_type, name) {
                warn!(
                    "Unknown {} plugin '{}'; available: {:?}",
                    plugin_type.as_str(),
                    name,
                    self.list(plugin_type)
                );
            }
        }
    }
}

static REGISTRY: OnceLock<PluginRegistry> = OnceLock::new();

pub fn registry() -> &'static PluginRegistry {
    REGISTRY.get_or_init(PluginRegistry::new)
}
