//! Extensible bind backend: один файл на пакет (platform, arch, os, python, rez, …).
//!
//! Чтобы добавить модуль: создать `src/bind/<name>.rs` с `impl BindHandler` и
//! зарегистрировать в [`builtin_handlers`].
//!
//! Config: `plugins.pkg_rs.bind_modules_extra` добавляет имена (Python fallback);
//! `plugins.pkg_rs.bind_modules_remove` исключает имена.

mod arch;
mod common;
mod os;
mod pip;
mod platform;
mod python;
mod rez;
mod setuptools;

use std::path::Path;

/// Handler для одного bind-модуля. Добавление модуля = новый файл + одна строка в builtin_handlers.
pub trait BindHandler: Send + Sync {
    fn bind(&self, name: &str, path: &Path, no_deps: bool) -> Result<(), String>;

    fn strategy_label(&self) -> &'static str {
        "native"
    }
}

/// Встроенные обработчики. Порядок и имена задают регистр; конфиг может добавить (Python) или убрать.
pub fn builtin_handlers() -> Vec<(&'static str, Box<dyn BindHandler>)> {
    vec![
        ("platform", Box::new(platform::PlatformHandler)),
        ("arch", Box::new(arch::ArchHandler)),
        ("os", Box::new(os::OsHandler)),
        ("python", Box::new(python::PythonHandler)),
        ("rez", Box::new(rez::RezHandler)),
        ("setuptools", Box::new(setuptools::SetuptoolsHandler)),
        ("pip", Box::new(pip::PipHandler)),
    ]
}
