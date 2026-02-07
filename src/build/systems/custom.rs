//! Custom build system (rez-style build_command).

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::build_command::BuildCommand;
use crate::error::BuildError;

pub struct CustomBuildSystem;

impl BuildSystem for CustomBuildSystem {
    fn name(&self) -> &'static str {
        "custom"
    }

    fn build(&self, ctx: &BuildContext<'_>, args: &BuildSystemArgs<'_>) -> Result<(), BuildError> {
        let Some(build_command) = &ctx.package.build_command else {
            return Err(BuildError::Config(
                "custom build requires build_command".to_string(),
            ));
        };

        if build_command.is_disabled() {
            return Ok(());
        }

        let root_abs = ctx
            .source_dir
            .canonicalize()
            .unwrap_or_else(|_| ctx.source_dir.to_path_buf());
        let build_abs = ctx
            .build_dir
            .canonicalize()
            .unwrap_or_else(|_| ctx.build_dir.to_path_buf());
        let install_abs = ctx
            .install_dir
            .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()));

        let variant_index = ctx
            .variant_index
            .map(|v| v.to_string())
            .unwrap_or_default();
        let install_flag = if ctx.install { "install" } else { "" };

        let expand = |text: &str| -> String {
            text.replace("{root}", &root_abs.display().to_string())
                .replace("{install}", install_flag)
                .replace("{build_path}", &build_abs.display().to_string())
                .replace(
                    "{install_path}",
                    &install_abs
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                )
                .replace("{name}", &ctx.package.base)
                .replace("{variant_index}", &variant_index)
                .replace("{version}", &ctx.package.version)
                .trim()
                .to_string()
        };

        match build_command {
            BuildCommand::String(cmd) => {
                let mut full_cmd = cmd.clone();
                if !args.build_args.is_empty() {
                    let quoted = args
                        .build_args
                        .iter()
                        .map(|a| super::super::shell_quote(a))
                        .collect::<Vec<_>>();
                    full_cmd.push(' ');
                    full_cmd.push_str(&quoted.join(" "));
                }
                let command = expand(&full_cmd);
                super::super::run_shell_command(&command, ctx.build_dir, ctx.env)
            }
            BuildCommand::List(list) => {
                let mut argv = list.clone();
                argv.extend(args.build_args.iter().cloned());
                let mut expanded = argv.iter().map(|s| expand(s)).collect::<Vec<_>>();
                if expanded.is_empty() {
                    return Err(BuildError::Config("empty build_command list".to_string()));
                }
                let program = expanded.remove(0);
                super::super::run_command(&program, &expanded, ctx.build_dir, ctx.env)
            }
            BuildCommand::Disabled(value) => Err(BuildError::Config(format!(
                "invalid build_command boolean: {}",
                value
            ))),
        }
    }

    fn supports_install(&self) -> bool {
        false
    }
}
