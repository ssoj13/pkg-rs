//! Rez bind command (quickstart override + passthrough).

use crate::cli::RezStubArgs;
use pkg_lib::config;
use pkg_lib::py::{ensure_python_executable, ensure_rez_on_sys_path};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn cmd_rez_bind(args: &RezStubArgs) -> ExitCode {
    let parsed = parse_bind_args(&args.args);
    if !parsed.quickstart {
        return super::cmd_rez_passthrough("bind", &args.args);
    }

    if parsed.list || parsed.search {
        return super::cmd_rez_passthrough("bind", &args.args);
    }

    cmd_quickstart(parsed)
}

#[derive(Debug, Default)]
struct BindQuickstartArgs {
    quickstart: bool,
    list: bool,
    search: bool,
    release: bool,
    no_deps: bool,
    install_path: Option<PathBuf>,
    unknown: Vec<String>,
}

fn parse_bind_args(args: &[String]) -> BindQuickstartArgs {
    let mut parsed = BindQuickstartArgs::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--quickstart" => {
                parsed.quickstart = true;
                i += 1;
            }
            "--release" | "-r" => {
                parsed.release = true;
                i += 1;
            }
            "--no-deps" => {
                parsed.no_deps = true;
                i += 1;
            }
            "--list" | "-l" => {
                parsed.list = true;
                i += 1;
            }
            "--search" | "-s" => {
                parsed.search = true;
                i += 1;
            }
            "--install-path" | "-i" => {
                if i + 1 < args.len() {
                    parsed.install_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    parsed.unknown.push(args[i].clone());
                    i += 1;
                }
            }
            _ => {
                parsed.unknown.push(args[i].clone());
                i += 1;
            }
        }
    }

    parsed
}

fn cmd_quickstart(args: BindQuickstartArgs) -> ExitCode {
    let cfg = match config::get() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("Config error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let install_path = if let Some(path) = args.install_path {
        path
    } else if args.release {
        match config::release_packages_path(cfg) {
            Some(path) => path,
            None => {
                eprintln!("rez bind --release: release_packages_path is not set");
                return ExitCode::FAILURE;
            }
        }
    } else {
        match config::local_packages_path(cfg) {
            Some(path) => path,
            None => {
                eprintln!("rez bind: local_packages_path is not set");
                return ExitCode::FAILURE;
            }
        }
    };

    if let Err(err) = std::fs::create_dir_all(&install_path) {
        eprintln!(
            "rez bind: failed to create install path {}: {}",
            install_path.display(),
            err
        );
        return ExitCode::FAILURE;
    }

    if !args.unknown.is_empty() {
        eprintln!(
            "rez bind --quickstart: ignoring extra args: {:?}",
            args.unknown
        );
    }

    let names = [
        "platform",
        "arch",
        "os",
        "python",
        "rez",
        "rezgui",
        "setuptools",
        "pip",
    ];

    let _ = Python::initialize();

    let result: Result<(), String> = Python::attach(|py| {
        ensure_rez_on_sys_path(py).map_err(|e| e.to_string())?;
        ensure_python_executable(py).map_err(|e| e.to_string())?;

        let package_bind = py.import("rez.package_bind").map_err(|e| e.to_string())?;
        let bind_package = package_bind
            .getattr("bind_package")
            .map_err(|e| e.to_string())?;
        let print_list = package_bind
            .getattr("_print_package_list")
            .map_err(|e| e.to_string())?;

        let (stdout_buf, stderr_buf) = setup_capture(py).map_err(|e| e.to_string())?;

        let installed_variants = PyList::empty(py);
        let install_path_str = install_path.to_string_lossy().to_string();
        let no_deps = true;

        for name in names {
            if package_already_installed(&install_path, name) {
                println!("Skipping {} (already installed)", name);
                continue;
            }

            println!("Binding {} into {}...", name, install_path.display());
            let kwargs = PyDict::new(py);
            kwargs
                .set_item("path", install_path_str.clone())
                .map_err(|e| e.to_string())?;
            kwargs.set_item("no_deps", no_deps).map_err(|e| e.to_string())?;
            kwargs.set_item("quiet", true).map_err(|e| e.to_string())?;

            match bind_package.call((name,), Some(&kwargs)) {
                Ok(variants) => {
                    let variants_list = variants
                        .cast::<pyo3::types::PyList>()
                        .map_err(|e| e.to_string())?;
                    for item in variants_list.iter() {
                        installed_variants
                            .append(item)
                            .map_err(|e| e.to_string())?;
                    }
                }
                Err(err) => {
                    if err.is_instance_of::<pyo3::exceptions::PyFileExistsError>(py) {
                        eprintln!("Skipping {} (already exists)", name);
                        continue;
                    }
                    return Err(err.to_string());
                }
            }
        }

        if installed_variants.len() > 0 {
            println!(
                "\nSuccessfully converted the following software found on the current system into Rez packages:\n"
            );
            print_list
                .call1((installed_variants,))
                .map_err(|e| e.to_string())?;
            println!(
                "\nTo bind other software, see what's available using the command 'rez-bind --list', then run 'rez-bind <name>'.\n"
            );
        }

        let (stdout_text, stderr_text) =
            take_capture(&stdout_buf, &stderr_buf).map_err(|e| e.to_string())?;
        if !stdout_text.is_empty() {
            print!("{}", stdout_text);
        }
        if !stderr_text.is_empty() {
            eprint!("{}", stderr_text);
        }

        Ok(())
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Rez bind quickstart error: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn package_already_installed(repo: &Path, name: &str) -> bool {
    let base = repo.join(name);
    if !base.is_dir() {
        return false;
    }
    if base.join("package.py").is_file() {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("package.py").is_file() {
                return true;
            }
            if path.is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        if sub.path().join("package.py").is_file() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    true
}

fn setup_capture(py: Python<'_>) -> PyResult<(pyo3::Bound<'_, PyAny>, pyo3::Bound<'_, PyAny>)> {
    let sys = py.import("sys")?;
    let io = py.import("io")?;
    let stdout_buf = io.call_method0("StringIO")?;
    let stderr_buf = io.call_method0("StringIO")?;
    sys.setattr("stdout", &stdout_buf)?;
    sys.setattr("stderr", &stderr_buf)?;
    sys.setattr("__stdout__", &stdout_buf)?;
    sys.setattr("__stderr__", &stderr_buf)?;
    Ok((stdout_buf, stderr_buf))
}

fn take_capture(
    stdout_buf: &pyo3::Bound<'_, PyAny>,
    stderr_buf: &pyo3::Bound<'_, PyAny>,
) -> PyResult<(String, String)> {
    let stdout_text: String = stdout_buf.call_method0("getvalue")?.extract()?;
    let stderr_text: String = stderr_buf.call_method0("getvalue")?.extract()?;
    Ok((stdout_text, stderr_text))
}
