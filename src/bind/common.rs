//! Общие хелперы для нативных bind-модулей.

use std::path::Path;
use std::process::Command;

pub(crate) fn io_err(e: std::io::Error) -> String {
    format!("ошибка ввода-вывода: {}", e)
}

pub fn write_basic_package(repo: &Path, name: &str, version: &str) -> Result<(), String> {
    let pkg_dir = repo.join(name).join(version);
    std::fs::create_dir_all(&pkg_dir).map_err(io_err)?;
    let package_py = pkg_dir.join("package.py");
    let content = format!(
        "from pkg import Package\n\n\ndef get_package():\n    pkg = Package(\"{}\", \"{}\")\n    return pkg\n",
        name, version
    );
    std::fs::write(package_py, content).map_err(io_err)?;
    Ok(())
}

pub fn package_version_installed(repo: &Path, name: &str, version: &str) -> bool {
    repo.join(name).join(version).join("package.py").is_file()
}

pub fn which(exe: &str) -> Result<std::path::PathBuf, String> {
    let path = std::env::var_os("PATH").ok_or_else(|| "PATH не задан".to_string())?;
    let exe_name = if cfg!(windows) && !exe.ends_with(".exe") {
        format!("{}.exe", exe)
    } else {
        exe.to_string()
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(&exe_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("исполняемый файл не найден: {}", exe))
}

/// Рекурсивно копирует каталог src в dst (dst создаётся).
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(io_err)?;
    for entry in std::fs::read_dir(src).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        let ty = entry.file_type().map_err(io_err)?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path).map_err(io_err)?;
        }
    }
    Ok(())
}

/// Путь к установленному Python-модулю (каталог пакета или каталог с одним файлом).
pub fn python_module_path(py_exe: &Path, module: &str) -> Result<std::path::PathBuf, String> {
    let code = format!(
        "import {}, os; p = getattr({}, '__path__', None); print(p[0] if p else os.path.dirname({}.__file__))",
        module, module, module
    );
    let out = run_cmd(&[py_exe.to_str().unwrap(), "-c", &code])?;
    Ok(std::path::PathBuf::from(out))
}

pub fn run_cmd(args: &[&str]) -> Result<String, String> {
    let (exe, rest) = args
        .split_first()
        .ok_or_else(|| "пустая команда".to_string())?;
    let out = Command::new(exe)
        .args(rest)
        .output()
        .map_err(io_err)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("команда не удалась: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn detect_arch() -> String {
    if cfg!(windows) {
        std::env::var("PROCESSOR_ARCHITECTURE")
            .unwrap_or_else(|_| std::env::consts::ARCH.to_string())
    } else {
        std::env::consts::ARCH.to_string()
    }
}

pub fn detect_os() -> String {
    if cfg!(windows) {
        detect_windows_version()
            .map(|v| format!("windows-{}", v))
            .unwrap_or_else(|| "windows".to_string())
    } else if cfg!(target_os = "macos") {
        detect_macos_version()
            .map(|v| format!("osx-{}", v))
            .unwrap_or_else(|| "osx".to_string())
    } else if cfg!(target_os = "linux") {
        detect_linux_version().unwrap_or_else(|| "linux".to_string())
    } else {
        std::env::consts::OS.to_string()
    }
}

#[cfg(windows)]
pub fn detect_windows_version() -> Option<String> {
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
pub fn detect_windows_version() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub fn detect_macos_version() -> Option<String> {
    let out = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn detect_macos_version() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
pub fn detect_linux_version() -> Option<String> {
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
pub fn detect_linux_version() -> Option<String> {
    None
}
