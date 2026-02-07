//! Make build system.

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::error::BuildError;

pub struct MakeBuildSystem;

impl BuildSystem for MakeBuildSystem {
    fn name(&self) -> &'static str {
        "make"
    }

    fn detects_source(&self, source_dir: &std::path::Path) -> bool {
        source_dir.join("Makefile").exists()
    }

    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let make_dir = if ctx.build_dir.join("Makefile").exists() {
            ctx.build_dir
        } else {
            ctx.source_dir
        };

        let mut argv: Vec<String> = Vec::new();
        argv.extend(args.build_args.iter().cloned());

        if !argv.iter().any(|a| a.starts_with("-j")) {
            let thread_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            argv.insert(0, format!("-j{}", thread_count));
        }

        if let Some(prefix) = ctx.install_dir {
            if !argv.iter().any(|a| a.starts_with("DESTDIR=")) {
                argv.push(format!("DESTDIR={}", prefix.display()));
            }
        }

        super::super::run_command("make", &argv, make_dir, ctx.env)?;

        Ok(())
    }

    fn install(&self, ctx: &BuildContext<'_>, _args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let make_dir = if ctx.build_dir.join("Makefile").exists() {
            ctx.build_dir
        } else {
            ctx.source_dir
        };

        super::super::run_command("make", &["install".to_string()], make_dir, ctx.env)
    }
}
