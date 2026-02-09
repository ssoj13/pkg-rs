//! Bind module: pip (Python pip).
//!
//! Как в Rez: копируем модуль pip в root/python/pip, копируем исполняемый файл pip
//! в root/bin, прописываем PYTHONPATH и PATH. После `rez env pip` (вместе с python)
//! доступны команда pip и `import pip`.

use super::{common, BindHandler};
use std::path::Path;

pub struct PipHandler;

impl BindHandler for PipHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let py_exe = common::which("python").or_else(|_| common::which("python3"))?;
        let out = common::run_cmd(&[
            py_exe.to_str().unwrap(),
            "-m",
            "pip",
            "--version",
        ])
        .map_err(|e| format!("pip не найден или ошибка: {}", e))?;
        let version = out
            .split_whitespace()
            .nth(1)
            .unwrap_or("0.0.0")
            .to_string();
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        let pkg_dir = path.join(name).join(&version);
        let python_dir = pkg_dir.join("python");
        let bin_dir = pkg_dir.join("bin");
        std::fs::create_dir_all(&python_dir).map_err(common::io_err)?;
        std::fs::create_dir_all(&bin_dir).map_err(common::io_err)?;
        // Копируем модуль pip (как в Rez)
        let src = common::python_module_path(&py_exe, "pip")?;
        let dst = python_dir.join("pip");
        common::copy_dir_all(&src, &dst)?;
        // Копируем исполняемый файл pip в bin
        if let Ok(pip_exe) = common::which("pip") {
            let dest = bin_dir.join(if cfg!(windows) { "pip.exe" } else { "pip" });
            std::fs::copy(&pip_exe, &dest).map_err(common::io_err)?;
        }
        let package_py = pkg_dir.join("package.py");
        let content = format!(
            r#"from pkg import Package, Env, Evar, Action

def get_package():
    pkg = Package("{}", "{}")
    pkg.add_env(Env("default", [
        Evar("PYTHONPATH", "{{root}}/python", Action.Prepend),
        Evar("PATH", "{{root}}/bin", Action.Prepend),
    ]))
    return pkg
"#,
            name, version
        );
        std::fs::write(package_py, content).map_err(common::io_err)?;
        Ok(())
    }
}
