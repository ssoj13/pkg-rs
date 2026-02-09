//! Bind module: python (system Python interpreter).

use super::{common, BindHandler};
use std::path::Path;

pub struct PythonHandler;

impl BindHandler for PythonHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let py_exe = common::which("python").or_else(|_| common::which("python3"))?;
        let version = common::run_cmd(&[
            py_exe.to_str().unwrap(),
            "-c",
            "import sys; print('.'.join(map(str, sys.version_info[:3])))",
        ])?;
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        let pkg_dir = path.join(name).join(&version);
        let bin_dir = pkg_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).map_err(common::io_err)?;
        let dest = bin_dir.join(if cfg!(windows) { "python.exe" } else { "python" });
        std::fs::copy(&py_exe, &dest).map_err(common::io_err)?;
        let package_py = pkg_dir.join("package.py");
        let content = format!(
            r#"from pkg import Package, Env, Evar, Action

def get_package():
    pkg = Package("{}", "{}")
    pkg.add_env(Env("default", [Evar("PATH", "{{root}}/bin", Action.Prepend)]))
    return pkg
"#,
            name, version
        );
        std::fs::write(package_py, content).map_err(common::io_err)?;
        Ok(())
    }
}
