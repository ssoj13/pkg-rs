//! CMake build system.

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::error::BuildError;
use std::path::{Path, PathBuf};

pub struct CmakeBuildSystem;

impl BuildSystem for CmakeBuildSystem {
    fn name(&self) -> &'static str {
        "cmake"
    }

    fn detects_source(&self, source_dir: &Path) -> bool {
        source_dir.join("CMakeLists.txt").exists()
    }

    fn configure(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let abs_path = |path: &Path| -> PathBuf {
            if path.is_absolute() {
                path.to_path_buf()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(path)
            } else {
                path.to_path_buf()
            }
        };

        let source_dir_abs = abs_path(ctx.source_dir);
        let build_root_abs = abs_path(ctx.build_dir);

        let mut configure_args = vec![
            "-S".to_string(),
            source_dir_abs.display().to_string(),
            "-B".to_string(),
            build_root_abs.display().to_string(),
        ];
        configure_args.extend(args.build_args.iter().cloned());
        if let Some(prefix) = ctx.install_dir {
            configure_args.push(format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()));
        }

        super::super::run_command("cmake", &configure_args, &build_root_abs, ctx.env)?;
        Ok(())
    }

    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let abs_path = |path: &Path| -> PathBuf {
            if path.is_absolute() {
                path.to_path_buf()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(path)
            } else {
                path.to_path_buf()
            }
        };

        let build_root_abs = abs_path(ctx.build_dir);

        let mut build_cmd_args = vec!["--build".to_string(), build_root_abs.display().to_string()];
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        build_cmd_args.push("--parallel".to_string());
        build_cmd_args.push(thread_count.to_string());

        if !args.child_build_args.is_empty() {
            build_cmd_args.push("--".to_string());
            build_cmd_args.extend(args.child_build_args.iter().cloned());
        }

        super::super::run_command("cmake", &build_cmd_args, &build_root_abs, ctx.env)?;
        Ok(())
    }

    fn install(&self, ctx: &BuildContext<'_>, _args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let abs_path = |path: &Path| -> PathBuf {
            if path.is_absolute() {
                path.to_path_buf()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(path)
            } else {
                path.to_path_buf()
            }
        };

        let build_root_abs = abs_path(ctx.build_dir);
        let install_args = vec!["--install".to_string(), build_root_abs.display().to_string()];
        super::super::run_command("cmake", &install_args, &build_root_abs, ctx.env)?;
        Ok(())
    }
}
