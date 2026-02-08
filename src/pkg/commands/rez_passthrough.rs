//! Pass-through to embedded Rez CLI for unported commands.

use pkg_lib::py::{ensure_python_executable, ensure_rez_on_sys_path};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::process::ExitCode;

pub fn cmd_rez_passthrough(command: &str, args: &[String]) -> ExitCode {
    let _ = Python::initialize();

    let result: Result<i32, String> = Python::attach(|py| {
        ensure_rez_on_sys_path(py).map_err(|e| e.to_string())?;
        ensure_python_executable(py).map_err(|e| e.to_string())?;

        let sys = py.import("sys").map_err(|e| e.to_string())?;
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(format!("rez-{}", command));
        argv.extend(args.iter().cloned());
        let argv_py = PyList::new(py, &argv).map_err(|e| e.to_string())?;
        sys.setattr("argv", argv_py)
            .map_err(|e| e.to_string())?;

        let (stdout_buf, stderr_buf) = setup_capture(py).map_err(|e| e.to_string())?;

        let main = py.import("rez.cli._main").map_err(|e| e.to_string())?;
        let run = main.getattr("run").map_err(|e| e.to_string())?;

        let run_result = run.call1((command,));

        let (stdout_text, stderr_text) =
            take_capture(&stdout_buf, &stderr_buf).map_err(|e| e.to_string())?;
        if !stdout_text.is_empty() {
            print!("{}", stdout_text);
        }
        if !stderr_text.is_empty() {
            eprint!("{}", stderr_text);
        }

        match run_result {
            Ok(_) => Ok(0),
            Err(err) => {
                if err.is_instance_of::<pyo3::exceptions::PySystemExit>(py) {
                    let code_obj = err.value(py).getattr("code").ok();
                    if let Some(code_obj) = code_obj {
                        if code_obj.is_none() {
                            return Ok(0);
                        }
                        if let Ok(code) = code_obj.extract::<i32>() {
                            return Ok(code);
                        }
                    }
                    Ok(1)
                } else {
                    Err(err.to_string())
                }
            }
        }
    });

    match result {
        Ok(code) => ExitCode::from(clamp_exit_code(code)),
        Err(err) => {
            eprintln!("Rez CLI error: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn clamp_exit_code(code: i32) -> u8 {
    if code <= 0 {
        0
    } else if code > 255 {
        255
    } else {
        code as u8
    }
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
