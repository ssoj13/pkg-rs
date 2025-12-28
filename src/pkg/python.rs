//! Python REPL and script execution.

use pkg_lib::{Action, App, Env, Evar, Package, Solver, Storage};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::CString;
use std::path::PathBuf;
use std::process::ExitCode;

/// Run Python REPL or script.
pub fn cmd_python(script: Option<PathBuf>, args: Vec<String>, verbose: bool) -> ExitCode {
    // Initialize Python
    let _ = pyo3::Python::initialize();

    Python::attach(|py| {
        // Create globals with injected pkg classes
        let globals = PyDict::new(py);

        // Create pkg module and register in sys.modules
        let pkg_module = PyModule::new(py, "pkg").unwrap();
        pkg_module.add_class::<Package>().ok();
        pkg_module.add_class::<Env>().ok();
        pkg_module.add_class::<Evar>().ok();
        pkg_module.add_class::<App>().ok();
        pkg_module.add_class::<Action>().ok();
        pkg_module.add_class::<Storage>().ok();
        pkg_module.add_class::<Solver>().ok();
        pkg_module.add("__all__", vec!["Package", "Env", "Evar", "App", "Action", "Storage", "Solver"]).ok();

        // Register in sys.modules
        if let Ok(sys) = py.import("sys") {
            if let Ok(modules) = sys.getattr("modules") {
                modules.set_item("pkg", &pkg_module).ok();
            }
        }

        // Add pkg module to globals
        globals.set_item("pkg", &pkg_module).ok();

        // Also inject classes directly for convenience
        globals.set_item("Package", py.get_type::<Package>()).ok();
        globals.set_item("Env", py.get_type::<Env>()).ok();
        globals.set_item("Evar", py.get_type::<Evar>()).ok();
        globals.set_item("App", py.get_type::<App>()).ok();
        globals.set_item("Action", py.get_type::<Action>()).ok();
        globals.set_item("Storage", py.get_type::<Storage>()).ok();
        globals.set_item("Solver", py.get_type::<Solver>()).ok();

        // Setup builtins
        let setup_code = CString::new(
            r#"
import sys
from pathlib import Path
print("pkg: Package, Env, Evar, App, Action, Storage, Solver")
"#,
        )
        .unwrap();

        if let Err(e) = py.run(&setup_code, Some(&globals), None) {
            eprintln!("Setup error: {}", e);
        }

        match script {
            Some(path) => {
                // Run script
                if verbose {
                    println!("Running: {:?}", path);
                }

                // Read script
                let code = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {:?}: {}", path, e);
                        return ExitCode::FAILURE;
                    }
                };

                // Set sys.argv
                let argv_list: Vec<_> = std::iter::once(path.to_string_lossy().to_string())
                    .chain(args.iter().cloned())
                    .collect();
                let argv_code =
                    CString::new(format!("import sys; sys.argv = {:?}", argv_list)).unwrap();
                let _ = py.run(&argv_code, Some(&globals), None);

                // Run the script
                let code_cstr = CString::new(code).unwrap();
                if let Err(e) = py.run(&code_cstr, Some(&globals), None) {
                    eprintln!("Error: {}", e);
                    return ExitCode::FAILURE;
                }

                ExitCode::SUCCESS
            }
            None => {
                // Interactive REPL
                println!("pkg Python REPL");

                let repl_code = CString::new(
                    r#"
import code
console = code.InteractiveConsole(locals=globals())
console.interact(banner='', exitmsg='')
"#,
                )
                .unwrap();

                if let Err(e) = py.run(&repl_code, Some(&globals), None) {
                    eprintln!("REPL error: {}", e);
                    return ExitCode::FAILURE;
                }

                ExitCode::SUCCESS
            }
        }
    })
}
