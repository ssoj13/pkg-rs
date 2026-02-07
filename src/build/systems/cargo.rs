//! Cargo build system.

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::error::BuildError;
use std::path::Path;

pub struct CargoBuildSystem;

impl BuildSystem for CargoBuildSystem {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn detects_source(&self, source_dir: &Path) -> bool {
        source_dir.join("Cargo.toml").exists()
    }

    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let mut argv = vec!["build".to_string()];
        argv.extend(args.build_args.iter().cloned());
        super::super::run_command("cargo", &argv, ctx.source_dir, ctx.env)
    }

    fn install(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let Some(prefix) = ctx.install_dir else {
            return Ok(());
        };

        let mut argv = vec![
            "install".to_string(),
            "--path".to_string(),
            ctx.source_dir.display().to_string(),
            "--root".to_string(),
            prefix.display().to_string(),
        ];
        argv.extend(
            args.build_args
                .iter()
                .filter(|arg| arg.as_str() != "--release")
                .cloned(),
        );
        super::super::run_command("cargo", &argv, ctx.source_dir, ctx.env)
    }
}
