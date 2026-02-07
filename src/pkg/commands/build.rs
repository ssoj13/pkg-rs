//! Build command.

use pkg_lib::build::{build_package, BuildOptions, BuildType};
use pkg_lib::{Loader, Storage};
use std::path::PathBuf;
use std::process::ExitCode;

/// Run the build pipeline in the current package directory.
pub fn cmd_build(
    storage: &Storage,
    build_system: Option<String>,
    process: String,
    build_args: Option<String>,
    child_build_args: Option<String>,
    variants: Vec<usize>,
    clean: bool,
    install: bool,
    prefix: Option<PathBuf>,
    scripts: bool,
    view_pre: bool,
    extra_args: Vec<String>,
) -> ExitCode {
    let package_path = PathBuf::from("package.py");
    if !package_path.exists() {
        eprintln!("package.py not found in current directory");
        return ExitCode::FAILURE;
    }

    let mut loader = Loader::new(Some(false));
    let mut pkg = match loader.load_path(&package_path) {
        Ok(pkg) => pkg,
        Err(e) => {
            eprintln!("Failed to load package.py: {}", e);
            return ExitCode::FAILURE;
        }
    };
    pkg.package_source = Some(package_path.to_string_lossy().to_string());

    if view_pre {
        match pkg.to_json_pretty() {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Failed to serialize package: {}", e),
        }
        return ExitCode::SUCCESS;
    }

    let mut merged_build_args = pkg.build_args.clone();
    merged_build_args.extend(parse_args(build_args));

    let merged_child_args = parse_args(child_build_args);
    let build_type = match process.as_str() {
        "central" => BuildType::Central,
        _ => BuildType::Local,
    };

    let options = BuildOptions {
        build_system,
        build_args: merged_build_args,
        child_build_args: merged_child_args,
        variants,
        clean,
        install,
        prefix,
        scripts,
        build_type,
        extra_args,
    };

    match build_package(&pkg, &package_path, storage, &options) {
        Ok(report) => {
            println!("Build complete: {} variant(s)", report.built_variants);
            if let Some(path) = report.install_path {
                println!("Installed to: {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Build failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn parse_args(args: Option<String>) -> Vec<String> {
    let Some(args) = args else { return Vec::new() };
    match shell_words::split(&args) {
        Ok(split) => split,
        Err(_) => args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
    }
}
