//! Python (PEP517 / setup.py) build system.

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::error::BuildError;
use std::path::Path;

pub struct PythonBuildSystem;

impl BuildSystem for PythonBuildSystem {
    fn name(&self) -> &'static str {
        "python"
    }

    fn detects_source(&self, source_dir: &Path) -> bool {
        source_dir.join("pyproject.toml").exists() || source_dir.join("setup.py").exists()
    }

    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let python = std::env::var("PKG_PYTHON").unwrap_or_else(|_| "python".to_string());

        if ctx.source_dir.join("pyproject.toml").exists() {
            let mut argv = vec![
                "-m".to_string(),
                "build".to_string(),
                "--wheel".to_string(),
                "--outdir".to_string(),
                ctx.build_dir.display().to_string(),
            ];
            argv.extend(args.build_args.iter().cloned());
            return super::super::run_command(&python, &argv, ctx.source_dir, ctx.env);
        }

        if ctx.source_dir.join("setup.py").exists() {
            let mut argv = vec![
                "setup.py".to_string(),
                "build".to_string(),
                "--build-base".to_string(),
                ctx.build_dir.display().to_string(),
            ];
            argv.extend(args.build_args.iter().cloned());
            return super::super::run_command(&python, &argv, ctx.source_dir, ctx.env);
        }

        Err(BuildError::Config(
            "python build requires pyproject.toml or setup.py".to_string(),
        ))
    }

    fn install(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let Some(prefix) = ctx.install_dir else {
            return Ok(());
        };

        let python = std::env::var("PKG_PYTHON").unwrap_or_else(|_| "python".to_string());
        let install_mode = python_install_mode();

        if ctx.source_dir.join("pyproject.toml").exists() {
            let mut argv = vec!["-m".to_string(), "pip".to_string(), "install".to_string(), ".".to_string()];
            match install_mode {
                PythonInstallMode::Prefix => {
                    argv.push("--prefix".to_string());
                    argv.push(prefix.display().to_string());
                }
                PythonInstallMode::Target => {
                    let target = prefix.join("python");
                    argv.push("--target".to_string());
                    argv.push(target.display().to_string());
                }
            }
            argv.push("--no-deps".to_string());
            argv.push("--no-build-isolation".to_string());
            argv.extend(args.child_build_args.iter().cloned());
            return super::super::run_command(&python, &argv, ctx.source_dir, ctx.env);
        }

        if ctx.source_dir.join("setup.py").exists() {
            if install_mode == PythonInstallMode::Target {
                let target = prefix.join("python");
                let mut argv = vec![
                    "-m".to_string(),
                    "pip".to_string(),
                    "install".to_string(),
                    ".".to_string(),
                    "--target".to_string(),
                    target.display().to_string(),
                    "--no-deps".to_string(),
                ];
                argv.extend(args.child_build_args.iter().cloned());
                return super::super::run_command(&python, &argv, ctx.source_dir, ctx.env);
            } else {
                let mut argv = vec![
                    "setup.py".to_string(),
                    "install".to_string(),
                    "--prefix".to_string(),
                    prefix.display().to_string(),
                ];
                argv.extend(args.child_build_args.iter().cloned());
                return super::super::run_command(&python, &argv, ctx.source_dir, ctx.env);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PythonInstallMode {
    Prefix,
    Target,
}

fn python_install_mode() -> PythonInstallMode {
    let mode = std::env::var("PKG_PYTHON_INSTALL_MODE")
        .ok()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if mode == "target" {
        return PythonInstallMode::Target;
    }

    let target_flag = std::env::var("PKG_PYTHON_INSTALL_TARGET")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    if target_flag {
        PythonInstallMode::Target
    } else {
        PythonInstallMode::Prefix
    }
}
