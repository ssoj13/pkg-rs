//! CMake build system.

use super::{BuildContext, BuildSystem, BuildSystemArgs};
use crate::error::BuildError;
use crate::{config, py};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct CmakeSettings {
    build_system: String,
    build_target: String,
    cmake_args: Vec<String>,
    cmake_binary: Option<String>,
    make_binary: Option<String>,
    install_pyc: bool,
}

impl Default for CmakeSettings {
    fn default() -> Self {
        let build_system = if cfg!(windows) {
            "nmake".to_string()
        } else {
            "make".to_string()
        };
        Self {
            build_system,
            build_target: "Release".to_string(),
            cmake_args: vec!["-Wno-dev".to_string(), "--no-warn-unused-cli".to_string()],
            cmake_binary: None,
            make_binary: None,
            install_pyc: true,
        }
    }
}

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

        let settings = load_cmake_settings();
        let cmake_bin = settings
            .cmake_binary
            .as_deref()
            .unwrap_or("cmake")
            .to_string();

        let source_dir_abs = abs_path(ctx.source_dir);
        let build_root_abs = abs_path(ctx.build_dir);

        let mut configure_args = vec![
            "-S".to_string(),
            source_dir_abs.display().to_string(),
            "-B".to_string(),
            build_root_abs.display().to_string(),
        ];

        if !settings.cmake_args.is_empty() {
            configure_args.extend(settings.cmake_args.iter().cloned());
        }
        configure_args.extend(args.build_args.iter().cloned());

        if let Some(prefix) = ctx.install_dir {
            if !has_arg_prefix(&configure_args, "-DCMAKE_INSTALL_PREFIX=") {
                configure_args.push(format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()));
            }
        }

        if let Some(module_path) = cmake_module_path() {
            if !has_arg_prefix(&configure_args, "-DCMAKE_MODULE_PATH=") {
                configure_args.push(format!("-DCMAKE_MODULE_PATH={}", module_path));
            }
        }

        if !has_arg_prefix(&configure_args, "-DCMAKE_BUILD_TYPE=") {
            let build_type = env_or_ctx(ctx, "CMAKE_BUILD_TYPE").unwrap_or(settings.build_target);
            configure_args.push(format!("-DCMAKE_BUILD_TYPE={}", build_type));
        }

        if !has_arg_prefix(&configure_args, "-DREZ_BUILD_TYPE=") {
            if let Some(build_type) = ctx.env.get("REZ_BUILD_TYPE") {
                configure_args.push(format!("-DREZ_BUILD_TYPE={}", build_type));
            }
        }

        if !has_arg_prefix(&configure_args, "-DREZ_BUILD_INSTALL=") {
            configure_args.push(format!(
                "-DREZ_BUILD_INSTALL={}",
                if ctx.install { "1" } else { "0" }
            ));
        }

        if !has_arg_prefix(&configure_args, "-DCMAKE_TOOLCHAIN_FILE=") {
            if let Some(toolchain) = env_or_ctx(ctx, "CMAKE_TOOLCHAIN_FILE") {
                configure_args.push(format!("-DCMAKE_TOOLCHAIN_FILE={}", toolchain));
            }
        }

        if !has_arg(&configure_args, "-A") {
            if let Some(platform) = env_or_ctx(ctx, "CMAKE_GENERATOR_PLATFORM") {
                configure_args.push("-A".to_string());
                configure_args.push(platform);
            }
        }

        if !has_arg(&configure_args, "-T") {
            if let Some(toolset) = env_or_ctx(ctx, "CMAKE_GENERATOR_TOOLSET") {
                configure_args.push("-T".to_string());
                configure_args.push(toolset);
            }
        }

        let generator = env_or_ctx(ctx, "CMAKE_GENERATOR")
            .or_else(|| cmake_generator(&settings.build_system).map(|s| s.to_string()));
        if let Some(generator) = generator {
            if !has_generator_arg(&configure_args) {
                configure_args.push("-G".to_string());
                configure_args.push(generator.clone());
            }
            warn_if_missing_windows_sdk(ctx, &generator);
        }

        dedupe_generator_args(&mut configure_args);

        super::super::run_command(&cmake_bin, &configure_args, &build_root_abs, ctx.env)?;
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

        let settings = load_cmake_settings();
        let cmake_bin = settings
            .cmake_binary
            .as_deref()
            .unwrap_or("cmake")
            .to_string();

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

        super::super::run_command(&cmake_bin, &build_cmd_args, &build_root_abs, ctx.env)?;
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

        let settings = load_cmake_settings();
        let cmake_bin = settings
            .cmake_binary
            .as_deref()
            .unwrap_or("cmake")
            .to_string();

        let build_root_abs = abs_path(ctx.build_dir);
        let install_args = vec!["--install".to_string(), build_root_abs.display().to_string()];
        super::super::run_command(&cmake_bin, &install_args, &build_root_abs, ctx.env)?;
        Ok(())
    }
}

fn load_cmake_settings() -> CmakeSettings {
    let mut settings = CmakeSettings::default();
    let Ok(cfg) = config::get() else {
        return settings;
    };

    if let Some(value) = config::get_str(cfg, "plugins.build_system.cmake.build_system") {
        settings.build_system = value;
    }
    if let Some(value) = config::get_str(cfg, "plugins.build_system.cmake.build_target") {
        settings.build_target = value;
    }
    if let Some(value) = config::get_json(cfg, "plugins.build_system.cmake.cmake_args") {
        settings.cmake_args = json_to_vec(value);
    }
    if let Some(value) = config::get_str(cfg, "plugins.build_system.cmake.cmake_binary") {
        if !value.trim().is_empty() {
            settings.cmake_binary = Some(value);
        }
    }
    if let Some(value) = config::get_str(cfg, "plugins.build_system.cmake.make_binary") {
        if !value.trim().is_empty() {
            settings.make_binary = Some(value);
        }
    }
    if let Some(value) = config::get_bool(cfg, "plugins.build_system.cmake.install_pyc") {
        settings.install_pyc = value;
    }

    settings
}

fn json_to_vec(value: JsonValue) -> Vec<String> {
    match value {
        JsonValue::Array(items) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        JsonValue::String(value) => vec![value],
        _ => Vec::new(),
    }
}

fn cmake_generator(build_system: &str) -> Option<&'static str> {
    match build_system {
        "eclipse" => Some("Eclipse CDT4 - Unix Makefiles"),
        "codeblocks" => Some("CodeBlocks - Unix Makefiles"),
        "make" => Some("Unix Makefiles"),
        "nmake" => Some("NMake Makefiles"),
        "mingw" => Some("MinGW Makefiles"),
        "xcode" => Some("Xcode"),
        "ninja" => Some("Ninja"),
        _ => None,
    }
}

fn has_arg(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn has_arg_prefix(args: &[String], prefix: &str) -> bool {
    args.iter().any(|arg| arg.starts_with(prefix))
}

fn has_generator_arg(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let trimmed = arg.trim_start();
        trimmed == "-G"
            || trimmed.starts_with("-G")
            || trimmed == "--generator"
            || trimmed.starts_with("--generator=")
    })
}

fn dedupe_generator_args(args: &mut Vec<String>) {
    let mut seen = false;
    let mut idx = 0;
    while idx < args.len() {
        let trimmed = args[idx].trim_start().to_string();
        let is_generator = trimmed == "-G"
            || trimmed.starts_with("-G")
            || trimmed == "--generator"
            || trimmed.starts_with("--generator=");
        if is_generator {
            if seen {
                args.remove(idx);
                if trimmed == "-G" || trimmed == "--generator" {
                    if idx < args.len() {
                        args.remove(idx);
                    }
                }
                continue;
            }
            seen = true;
        }
        idx += 1;
    }
}

fn env_or_ctx(ctx: &BuildContext<'_>, key: &str) -> Option<String> {
    ctx.env
        .get(key)
        .cloned()
        .or_else(|| std::env::var(key).ok())
}

fn cmake_module_path() -> Option<String> {
    let root = py::find_python_root().ok()?;
    let path = root
        .join("rezplugins")
        .join("build_system")
        .join("cmake_files");
    if path.is_dir() {
        Some(to_cmake_path(&path))
    } else {
        None
    }
}

fn to_cmake_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn warn_if_missing_windows_sdk(ctx: &BuildContext<'_>, generator: &str) {
    if !cfg!(windows) {
        return;
    }

    let needs_msvc = generator.contains("Visual Studio") || generator.contains("NMake");
    if !needs_msvc {
        return;
    }

    let lib = ctx.env.get("LIB").cloned().or_else(|| std::env::var("LIB").ok());
    if lib.is_none() {
        eprintln!(
            "Warning: Windows SDK libs not found in LIB. Run from a Visual Studio Developer Command Prompt or set WindowsSdkDir/VCINSTALLDIR."
        );
    }
}
