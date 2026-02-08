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
        if parsed.list || parsed.search {
            return super::cmd_rez_passthrough("bind", &args.args);
        }

        if parsed.unknown.is_empty() {
            if let Some(pkg) = parsed.pkg.as_deref() {
                if let Some(exit) = try_native_bind(pkg, &parsed) {
                    return exit;
                }
            }
        }

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
    pkg: Option<String>,
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
                if parsed.pkg.is_none() && !args[i].starts_with('-') {
                    parsed.pkg = Some(args[i].clone());
                } else {
                    parsed.unknown.push(args[i].clone());
                }
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

fn try_native_bind(pkg: &str, args: &BindQuickstartArgs) -> Option<ExitCode> {
    let pkg_name = match pkg {
        "platform" | "arch" | "os" => pkg,
        _ => return None,
    };

    if pkg.contains(['!', '+', '<', '>', '=']) || pkg.contains('-') {
        return None;
    }

    let install_path = match resolve_install_path(args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err);
            return Some(ExitCode::FAILURE);
        }
    };

    let version = match pkg_name {
        "platform" => detect_platform(),
        "arch" => detect_arch(),
        "os" => detect_os(),
        _ => "unknown".to_string(),
    };

    if package_version_installed(&install_path, pkg_name, &version) {
        println!("Skipping {} (already installed)", pkg_name);
        return Some(ExitCode::SUCCESS);
    }

    if let Err(err) = write_basic_package(&install_path, pkg_name, &version) {
        eprintln!("Failed to bind {}: {}", pkg_name, err);
        return Some(ExitCode::FAILURE);
    }

    println!(
        "Installed {} {} into {}",
        pkg_name,
        version,
        install_path.display()
    );
    Some(ExitCode::SUCCESS)
}

fn resolve_install_path(args: &BindQuickstartArgs) -> Result<PathBuf, String> {
    let cfg = config::get().map_err(|e| e.to_string())?;
    if let Some(path) = &args.install_path {
        return Ok(path.clone());
    }
    if args.release {
        return config::release_packages_path(cfg)
            .ok_or_else(|| "rez bind --release: release_packages_path is not set".to_string());
    }
    config::local_packages_path(cfg)
        .ok_or_else(|| "rez bind: local_packages_path is not set".to_string())
}

fn write_basic_package(repo: &Path, name: &str, version: &str) -> Result<(), String> {
    let pkg_dir = repo.join(name).join(version);
    std::fs::create_dir_all(&pkg_dir).map_err(|e| e.to_string())?;
    let package_py = pkg_dir.join("package.py");
    let content = format!(
        "from pkg import Package\n\n\ndef get_package():\n    pkg = Package(\"{}\", \"{}\")\n    return pkg\n",
        name, version
    );
    std::fs::write(package_py, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn detect_platform() -> String {
    std::env::consts::OS.to_string()
}

fn detect_arch() -> String {
    if cfg!(windows) {
        std::env::var("PROCESSOR_ARCHITECTURE")
            .unwrap_or_else(|_| std::env::consts::ARCH.to_string())
    } else {
        std::env::consts::ARCH.to_string()
    }
}

fn detect_os() -> String {
    if cfg!(windows) {
        detect_windows_version()
            .map(|ver| format!("windows-{}", ver))
            .unwrap_or_else(|| "windows".to_string())
    } else if cfg!(target_os = "macos") {
        detect_macos_version()
            .map(|ver| format!("osx-{}", ver))
            .unwrap_or_else(|| "osx".to_string())
    } else if cfg!(target_os = "linux") {
        detect_linux_version().unwrap_or_else(|| "linux".to_string())
    } else {
        std::env::consts::OS.to_string()
    }
}

#[cfg(windows)]
fn detect_windows_version() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()?;
    let major: u32 = key.get_value("CurrentMajorVersionNumber").ok().unwrap_or(10);
    let minor: u32 = key.get_value("CurrentMinorVersionNumber").ok().unwrap_or(0);
    let build: String = key
        .get_value("CurrentBuildNumber")
        .ok()
        .unwrap_or_else(|| "0".to_string());
    Some(format!("{}.{}.{}", major, minor, build))
}

#[cfg(not(windows))]
fn detect_windows_version() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn detect_macos_version() -> Option<String> {
    let output = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ver.is_empty() {
        None
    } else {
        Some(ver)
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_macos_version() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn detect_linux_version() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    let mut id = None;
    let mut version = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("ID=") {
            id = Some(rest.trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
            version = Some(rest.trim_matches('"').to_string());
        }
    }
    if let (Some(id), Some(version)) = (id, version) {
        Some(format!("{}-{}", id, version))
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn detect_linux_version() -> Option<String> {
    None
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
    false
}

fn package_version_installed(repo: &Path, name: &str, version: &str) -> bool {
    let path = repo.join(name).join(version).join("package.py");
    path.is_file()
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
