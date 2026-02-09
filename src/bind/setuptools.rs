//! Bind module: setuptools (Python setuptools).
//!
//! Как в Rez: копируем установленный модуль в пакет (root/python/setuptools) и
//! прописываем PYTHONPATH, чтобы после `rez env setuptools` можно было делать
//! `import setuptools`.

use super::{common, BindHandler};
use std::path::Path;

pub struct SetuptoolsHandler;

impl BindHandler for SetuptoolsHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let py_exe = common::which("python").or_else(|_| common::which("python3"))?;
        let version = common::run_cmd(&[
            py_exe.to_str().unwrap(),
            "-c",
            "import setuptools; print(setuptools.__version__)",
        ])
        .map_err(|e| format!("setuptools не найден или ошибка: {}", e))?;
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        let pkg_dir = path.join(name).join(&version);
        let python_dir = pkg_dir.join("python");
        std::fs::create_dir_all(&python_dir).map_err(common::io_err)?;
        let src = common::python_module_path(&py_exe, "setuptools")?;
        let dst = python_dir.join("setuptools");
        common::copy_dir_all(&src, &dst)?;
        let package_py = pkg_dir.join("package.py");
        let content = format!(
            r#"from pkg import Package, Env, Evar, Action

def get_package():
    pkg = Package("{}", "{}")
    pkg.add_env(Env("default", [Evar("PYTHONPATH", "{{root}}/python", Action.Prepend)]))
    return pkg
"#,
            name, version
        );
        std::fs::write(package_py, content).map_err(common::io_err)?;
        Ok(())
    }
}
