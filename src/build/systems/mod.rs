//! Build system registry and interfaces.

mod cargo;
mod cmake;
mod custom;
mod make;
mod python;

use crate::error::BuildError;
use crate::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use cargo::CargoBuildSystem;
pub use cmake::CmakeBuildSystem;
pub use custom::CustomBuildSystem;
pub use make::MakeBuildSystem;
pub use python::PythonBuildSystem;

/// Context passed to build system implementations.
#[derive(Debug, Clone, Copy)]
pub struct BuildContext<'a> {
    pub package: &'a Package,
    pub source_dir: &'a Path,
    pub build_dir: &'a Path,
    pub install_dir: Option<&'a PathBuf>,
    pub env: &'a HashMap<String, String>,
    pub variant_index: Option<usize>,
    pub install: bool,
}

/// Arguments passed to build system implementations.
#[derive(Debug, Clone, Copy)]
pub struct BuildSystemArgs<'a> {
    pub build_args: &'a [String],
    pub child_build_args: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    Configure,
    Build,
    Install,
}

/// Trait implemented by build systems.
pub trait BuildSystem {
    fn name(&self) -> &'static str;
    fn detects_source(&self, _source_dir: &Path) -> bool {
        false
    }
    fn before_phase(
        &self,
        _phase: BuildPhase,
        _ctx: &BuildContext<'_>,
        _args: &BuildSystemArgs<'_>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
    fn after_phase(
        &self,
        _phase: BuildPhase,
        _ctx: &BuildContext<'_>,
        _args: &BuildSystemArgs<'_>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
    fn configure(
        &self,
        _ctx: &BuildContext<'_>,
        _args: &BuildSystemArgs<'_>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError>;
    fn install(
        &self,
        _ctx: &BuildContext<'_>,
        _args: &BuildSystemArgs<'_>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
    fn supports_install(&self) -> bool {
        true
    }
}

/// Registry for available build systems.
pub struct BuildSystemRegistry {
    systems: Vec<Box<dyn BuildSystem>>,
}

impl BuildSystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: vec![
                Box::new(CustomBuildSystem),
                Box::new(MakeBuildSystem),
                Box::new(CmakeBuildSystem),
                Box::new(CargoBuildSystem),
                Box::new(PythonBuildSystem),
            ],
        }
    }

    pub fn by_name(&self, name: &str) -> Option<&dyn BuildSystem> {
        self.systems
            .iter()
            .map(|s| s.as_ref())
            .find(|s| s.name() == name)
    }

    pub fn detect(&self, source_dir: &Path) -> Option<&dyn BuildSystem> {
        self.systems
            .iter()
            .map(|s| s.as_ref())
            .find(|s| s.detects_source(source_dir))
    }
}
