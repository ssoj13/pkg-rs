//! Rex-style package command execution (pre_commands, commands, post_commands).
//!
//! Runs package command source in a minimal Python env that records env mutations
//! (set/append/insert), then applies them to the given Env. Used by `pkg env` for Rez parity.

use crate::{Env, Evar, Package};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::Path;

const REX_BOOTSTRAP: &str = r#"
class _EnvVarProxy:
    def __init__(self, env, name):
        self._env = env
        self._name = name

    def append(self, value):
        self._env._evars.append(("append", self._name, value))

    def prepend(self, value):
        self._env._evars.append(("insert", self._name, value))

    def set(self, value):
        self._env._evars.append(("set", self._name, value))

class _RexEnv:
    def __init__(self):
        self._evars = []

    def __getattr__(self, name):
        return _EnvVarProxy(self, name)

    def __setattr__(self, name, value):
        if name.startswith("_"):
            object.__setattr__(self, name, value)
        else:
            self._evars.append(("set", name, value))

env = _RexEnv()
"#;

/// Run package command source (rex-like) and merge recorded env mutations into `env`.
/// `root_path`: package root (e.g. parent of package.py); used as `ROOT` in the script.
pub fn apply_package_commands(
    env: &mut Env,
    package: &Package,
    command_source: &str,
    root_path: Option<&Path>,
) -> Result<(), String> {
    let root_str = root_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| String::new());

    pyo3::Python::attach(|py| {
        let globals = PyDict::new(py);
        if let Ok(os_mod) = py.import("os") {
            globals.set_item("os", os_mod).ok();
        }
        if let Ok(sys_mod) = py.import("sys") {
            globals.set_item("sys", sys_mod).ok();
        }

        let bootstrap = std::ffi::CString::new(REX_BOOTSTRAP)
            .map_err(|e| format!("rex bootstrap: {e}"))?;
        py.run(bootstrap.as_c_str(), Some(&globals), None)
            .map_err(|e| format!("rex bootstrap exec: {e}"))?;

        globals.set_item("ROOT", &root_str).ok();
        let this_dict = PyDict::new(py);
        this_dict.set_item("name", format!("{}-{}", package.base, package.version)).ok();
        this_dict.set_item("root", &root_str).ok();
        globals.set_item("this", this_dict).ok();

        let source = std::ffi::CString::new(command_source)
            .map_err(|e| format!("rex source: {e}"))?;
        py.run(source.as_c_str(), Some(&globals), None)
            .map_err(|e| format!("rex exec: {e}"))?;

        let env_obj = globals
            .get_item("env")
            .map_err(|e| format!("rex env get: {e}"))?
            .ok_or("rex env missing")?;
        let evars: Vec<(String, String, String)> = env_obj
            .getattr("_evars")
            .map_err(|e| format!("rex _evars: {e}"))?
            .extract()
            .map_err(|e| format!("rex _evars extract: {e}"))?;

        for (action, name, value) in evars {
            let evar = match action.as_str() {
                "append" => Evar::append(name, value),
                "insert" => Evar::insert(name, value),
                "set" => Evar::set(name, value),
                other => return Err(format!("rex unknown action: {}", other)),
            };
            env.add(evar);
        }

        Ok(())
    })
}

/// Dependency order: deps first (as-is), then root package. Used for rex command execution order.
pub fn packages_in_rex_order<'a>(pkg: &'a Package) -> Vec<&'a Package> {
    let mut out = Vec::with_capacity(pkg.deps.len() + 1);
    for dep in &pkg.deps {
        out.push(dep);
    }
    out.push(pkg);
    out
}

/// Package root directory from package_source (parent of package.py).
pub fn package_root_from_source(package_source: Option<&String>) -> Option<std::path::PathBuf> {
    let s = package_source.as_deref()?;
    let p = Path::new(s);
    p.parent().map(|x| x.to_path_buf())
}
