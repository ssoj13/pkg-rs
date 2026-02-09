//! Bind module: rez (Rez CLI).
//!
//! У нас rez — один бинарник; копируем его в пакет и выставляем PATH/PYTHONPATH на этот root.

use super::{common, BindHandler};
use std::path::Path;

pub struct RezHandler;

impl BindHandler for RezHandler {
    fn bind(&self, name: &str, path: &Path, _no_deps: bool) -> Result<(), String> {
        let rez_exe = common::which("rez")?;
        let out = common::run_cmd(&[rez_exe.to_str().unwrap(), "--version"])?;
        let version = out
            .split_whitespace()
            .nth(1)
            .unwrap_or("0.0.0")
            .to_string();
        if common::package_version_installed(path, name, &version) {
            return Ok(());
        }
        let pkg_dir = path.join(name).join(&version);
        std::fs::create_dir_all(&pkg_dir).map_err(common::io_err)?;
        let root = pkg_dir.join("rez_root");
        std::fs::create_dir_all(&root).map_err(common::io_err)?;
        let bin_dir = root.join("bin");
        std::fs::create_dir_all(&bin_dir).map_err(common::io_err)?;
        let dest = bin_dir.join(if cfg!(windows) { "rez.exe" } else { "rez" });
        std::fs::copy(&rez_exe, &dest).map_err(common::io_err)?;
        let package_py = pkg_dir.join("package.py");
        let content = format!(
            r#"from pkg import Package, Env, Evar, Action

def get_package():
    pkg = Package("{}", "{}")
    pkg.add_env(Env("default", [
        Evar("PATH", "{{root}}/rez_root/bin", Action.Prepend),
        Evar("PYTHONPATH", "{{root}}/rez_root", Action.Prepend),
    ]))
    return pkg
"#,
            name, version
        );
        std::fs::write(package_py, content).map_err(common::io_err)?;
        Ok(())
    }
}
