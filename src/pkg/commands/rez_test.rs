//! Rez test command.
//!
//! Runs package tests: resolves env, executes pre_test_commands (rex-style),
//! then runs each test entry (shell or exec) and prints a summary.

use crate::cli::TestArgs;
use pkg_lib::dep::DepSpec;
use pkg_lib::rex::{apply_package_commands, package_root_from_source};
use pkg_lib::{config, Env, Package, Storage};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::process::{Command, ExitCode};

#[derive(Clone, Debug)]
struct TestDef {
    name: String,
    command: TestCommand,
    requires: Vec<String>,
    run_on: Vec<String>,
    #[allow(dead_code)]
    on_variants: Option<JsonValue>,
}

#[derive(Clone, Debug)]
enum TestCommand {
    Shell(String),
    Exec(Vec<String>),
}

pub fn cmd_rez_test(storage: &Storage, args: &TestArgs) -> ExitCode {
    if args.inplace && (!args.extra_packages.is_empty() || args.paths.is_some() || args.no_local) {
        eprintln!("rez test: cannot use --inplace with --extra-packages/--paths/--no-local");
        return ExitCode::FAILURE;
    }

    if !args.extra_args.is_empty() && args.tests.len() > 1 {
        eprintln!("rez test: extra args are only allowed for a single test");
        return ExitCode::FAILURE;
    }

    let storage = match build_storage_for_tests(storage, args) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("rez test: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let package = match storage.resolve(&args.pkg) {
        Some(pkg) => pkg,
        None => {
            eprintln!("rez test: package not found: {}", args.pkg);
            return ExitCode::FAILURE;
        }
    };

    let tests = match collect_tests(&package) {
        Ok(tests) => tests,
        Err(err) => {
            eprintln!("rez test: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if tests.is_empty() {
        eprintln!("No tests found in {}", package.name);
        return ExitCode::SUCCESS;
    }

    if args.list {
        for name in tests.iter().map(|t| t.name.clone()).collect::<Vec<_>>() {
            println!("{}", name);
        }
        return ExitCode::SUCCESS;
    }

    let requested = if args.tests.is_empty() {
        filter_tests_by_run_on(&tests, &["default".to_string()])
    } else {
        match filter_tests_by_patterns(&tests, &args.tests) {
            Ok(list) => list,
            Err(err) => {
                eprintln!("rez test: {}", err);
                return ExitCode::FAILURE;
            }
        }
    };

    if requested.is_empty() {
        eprintln!("No matching tests found in {}", package.name);
        return ExitCode::SUCCESS;
    }

    let rxt = if args.inplace { load_current_context() } else { None };
    let mut summary = TestSummary::default();

    for test in requested {
        if summary.stopped {
            break;
        }
        let result = run_test(&storage, &package, &test, args, rxt.as_ref());
        let failed = matches!(result, TestOutcome::Failed(_));
        summary.record(&test.name, result);
        if args.stop_on_fail && failed {
            summary.stopped = true;
        }
    }

    summary.print();
    if summary.failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn build_storage_for_tests(storage: &Storage, args: &TestArgs) -> Result<Storage, String> {
    if let Some(paths) = &args.paths {
        let paths = std::env::split_paths(paths).map(|p| p.to_string_lossy().to_string()).collect();
        return Storage::scan_paths(paths).map_err(|e| e.to_string());
    }
    if args.no_local {
        let cfg = config::get().map_err(|e| e.to_string())?;
        let mut paths = config::packages_path(cfg);
        if let Some(local) = config::local_packages_path(cfg) {
            paths.retain(|p| p != &local);
        }
        let paths = paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
        return Storage::scan_paths(paths).map_err(|e| e.to_string());
    }
    Ok(storage.clone())
}

fn collect_tests(pkg: &Package) -> Result<Vec<TestDef>, String> {
    let Some(tests_value) = pkg.tests.clone() else {
        return Ok(Vec::new());
    };
    let json = toml_to_json(&tests_value);
    let obj = json.as_object().ok_or_else(|| "tests must be a mapping".to_string())?;
    let mut out = Vec::new();

    for (name, value) in obj {
        let (command, requires, run_on, on_variants) = parse_test_value(value)?;
        out.push(TestDef {
            name: name.to_string(),
            command,
            requires,
            run_on,
            on_variants,
        });
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn parse_test_value(value: &JsonValue) -> Result<(TestCommand, Vec<String>, Vec<String>, Option<JsonValue>), String> {
    match value {
        JsonValue::String(cmd) => Ok((TestCommand::Shell(cmd.to_string()), Vec::new(), vec!["default".to_string()], None)),
        JsonValue::Array(items) => {
            let mut cmd = Vec::new();
            for item in items {
                if let Some(s) = item.as_str() {
                    cmd.push(s.to_string());
                }
            }
            Ok((TestCommand::Exec(cmd), Vec::new(), vec!["default".to_string()], None))
        }
        JsonValue::Object(map) => {
            let command_val = map.get("command").ok_or_else(|| "test entry missing command".to_string())?;
            let command = match command_val {
                JsonValue::String(s) => TestCommand::Shell(s.to_string()),
                JsonValue::Array(items) => TestCommand::Exec(items.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                _ => return Err("test command must be string or list".to_string()),
            };
            let requires = map.get("requires")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let run_on = match map.get("run_on") {
                Some(JsonValue::String(s)) => vec![s.to_string()],
                Some(JsonValue::Array(items)) => items.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => vec!["default".to_string()],
            };
            let on_variants = map.get("on_variants").cloned();
            Ok((command, requires, run_on, on_variants))
        }
        _ => Err("invalid test entry".to_string()),
    }
}

fn filter_tests_by_run_on(tests: &[TestDef], run_on: &[String]) -> Vec<TestDef> {
    tests
        .iter()
        .filter(|t| t.run_on.iter().any(|r| run_on.contains(r)))
        .cloned()
        .collect()
}

fn filter_tests_by_patterns(tests: &[TestDef], patterns: &[String]) -> Result<Vec<TestDef>, String> {
    let mut out = Vec::new();
    for pat in patterns {
        let re = glob_to_regex(pat)?;
        for test in tests {
            if re.is_match(&test.name) && !out.iter().any(|t: &TestDef| t.name == test.name) {
                out.push(test.clone());
            }
        }
    }
    Ok(out)
}

fn glob_to_regex(pattern: &str) -> Result<regex::Regex, String> {
    let mut s = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => s.push_str(".*"),
            '?' => s.push('.'),
            '.' => s.push_str("\\."),
            other => s.push(other),
        }
    }
    s.push('$');
    regex::Regex::new(&s).map_err(|e| e.to_string())
}

fn run_test(
    storage: &Storage,
    pkg: &Package,
    test: &TestDef,
    args: &TestArgs,
    rxt: Option<&ContextInfo>,
) -> TestOutcome {
    let base_env = if args.inplace {
        build_inplace_env(rxt, pkg, test, args)
    } else {
        build_test_env(storage, pkg, test, args)
    };

    let mut env = match base_env {
        Ok(env) => env,
        Err(reason) => return TestOutcome::Skipped(reason),
    };

    if args.dry_run {
        println!("[dry-run] {}", test.name);
        print_command(&test.command, pkg, &args.extra_args);
        return TestOutcome::Skipped("dry-run".to_string());
    }

    // Run pre_test_commands rex-style and merge env mutations, then re-solve
    if let Some(pre) = pkg.pre_test_commands.as_ref() {
        let root_path = package_root_from_source(pkg.package_source.as_ref());
        if let Err(err) = apply_package_commands(
            &mut env,
            pkg,
            pre,
            root_path.as_deref(),
            Some("pre_test_commands"),
        ) {
            return TestOutcome::Failed(format!("pre_test_commands failed: {}", err));
        }
        env = match env.solve_impl(10, true) {
            Ok(solved) => solved,
            Err(e) => return TestOutcome::Failed(format!("env solve after pre_test_commands: {}", e)),
        };
    }

    let extra_args = if args.extra_args.is_empty() { Vec::new() } else { args.extra_args.clone() };
    match run_command(&test.command, pkg, &env, &extra_args) {
        Ok(()) => TestOutcome::Passed,
        Err(err) => TestOutcome::Failed(err),
    }
}

fn build_inplace_env(
    rxt: Option<&ContextInfo>,
    _pkg: &Package,
    test: &TestDef,
    args: &TestArgs,
) -> Result<Env, String> {
    let Some(rxt) = rxt else {
        return Err("not in a resolved environment".to_string());
    };

    if !rxt.satisfies_request(&args.pkg) {
        return Err("current environment does not satisfy request".to_string());
    }

    if !rxt.satisfies_requires(&test.requires, &args.extra_packages) {
        return Err("current environment does not meet test requirements".to_string());
    }

    let mut env = Env::new("inplace".to_string());
    for (k, v) in std::env::vars() {
        env.add(pkg_lib::Evar::set(&k, &v));
    }
    Ok(env)
}

fn build_test_env(
    storage: &Storage,
    pkg: &Package,
    test: &TestDef,
    args: &TestArgs,
) -> Result<Env, String> {
    let mut pkg = pkg.clone();
    for extra in &args.extra_packages {
        pkg.add_req(extra.clone());
    }
    for req in &test.requires {
        pkg.add_req(req.clone());
    }

    if let Err(err) = pkg.solve(storage.packages()) {
        return Err(format!("resolve failed: {}", err));
    }

    let env = pkg._env("default", true).or_else(|| pkg.default_env());
    let Some(env) = env else {
        return Err("default env not found".to_string());
    };

    env.solve_impl(10, true).map_err(|e| e.to_string())
}

fn run_command(cmd: &TestCommand, pkg: &Package, env: &Env, extra_args: &[String]) -> Result<(), String> {
    match cmd {
        TestCommand::Shell(text) => run_shell(text, pkg, env, extra_args),
        TestCommand::Exec(list) => run_exec(list, pkg, env, extra_args),
    }
}

fn run_shell(command: &str, pkg: &Package, env: &Env, extra_args: &[String]) -> Result<(), String> {
    let cmd = expand_command(command, pkg);
    let cmd = if extra_args.is_empty() {
        cmd
    } else {
        format!("{} {}", cmd, extra_args.join(" "))
    };

    let mut command = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", &cmd]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &cmd]);
        c
    };
    command.envs(env_to_hashmap(env));
    let status = command.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed ({:?})", status.code()))
    }
}

fn run_exec(list: &[String], _pkg: &Package, env: &Env, extra_args: &[String]) -> Result<(), String> {
    if list.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new(&list[0]);
    cmd.args(&list[1..]);
    cmd.args(extra_args);
    cmd.envs(env_to_hashmap(env));
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed ({:?})", status.code()))
    }
}

fn print_command(cmd: &TestCommand, pkg: &Package, extra_args: &[String]) {
    match cmd {
        TestCommand::Shell(text) => {
            let cmd = expand_command(text, pkg);
            if extra_args.is_empty() {
                println!("{}", cmd);
            } else {
                println!("{} {}", cmd, extra_args.join(" "));
            }
        }
        TestCommand::Exec(list) => {
            println!("{}", list.join(" "));
        }
    }
}

fn expand_command(command: &str, pkg: &Package) -> String {
    let root = pkg_root(pkg).unwrap_or_else(|| "".to_string());
    command
        .replace("{root}", &root)
        .replace("{name}", &pkg.name)
        .replace("{base}", &pkg.base)
        .replace("{version}", &pkg.version)
}

fn pkg_root(pkg: &Package) -> Option<String> {
    let source = pkg.package_source.as_ref()?;
    let path = Path::new(source);
    let root = if path.is_dir() { path } else { path.parent()? };
    Some(root.to_string_lossy().to_string())
}

#[derive(Default)]
struct TestSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    stopped: bool,
    details: Vec<(String, TestOutcome)>,
}

impl TestSummary {
    fn record(&mut self, name: &str, outcome: TestOutcome) {
        match &outcome {
            TestOutcome::Passed => self.passed += 1,
            TestOutcome::Failed(_) => self.failed += 1,
            TestOutcome::Skipped(_) => self.skipped += 1,
        }
        self.details.push((name.to_string(), outcome));
    }

    fn print(&self) {
        for (name, outcome) in &self.details {
            match outcome {
                TestOutcome::Passed => println!("PASS {}", name),
                TestOutcome::Failed(err) => println!("FAIL {}: {}", name, err),
                TestOutcome::Skipped(reason) => println!("SKIP {}: {}", name, reason),
            }
        }
        println!("\nSummary: {} passed, {} failed, {} skipped", self.passed, self.failed, self.skipped);
    }
}

enum TestOutcome {
    Passed,
    Failed(String),
    Skipped(String),
}

struct ContextInfo {
    resolved: HashMap<String, String>,
}

impl ContextInfo {
    fn satisfies_request(&self, request: &str) -> bool {
        if let Ok(spec) = DepSpec::parse_impl(request) {
            if let Some(version) = self.resolved.get(&spec.base) {
                return spec.matches_impl(version).unwrap_or(false);
            }
        }
        false
    }

    fn satisfies_requires(&self, requires: &[String], extra: &[String]) -> bool {
        for req in requires.iter().chain(extra.iter()) {
            if let Ok(spec) = DepSpec::parse_impl(req) {
                if let Some(version) = self.resolved.get(&spec.base) {
                    if !spec.matches_impl(version).unwrap_or(false) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }
}

fn load_current_context() -> Option<ContextInfo> {
    let rxt = std::env::var("REZ_RXT_FILE").ok()?;
    let content = fs::read_to_string(&rxt).ok()?;
    let json: JsonValue = serde_json::from_str(&content).ok()?;
    let mut resolved = HashMap::new();
    let list = json.get("resolved_packages")?.as_array()?;
    for item in list {
        let obj = item.as_object()?;
        let (base, version) = if let (Some(b), Some(v)) = (obj.get("base").and_then(|x| x.as_str()), obj.get("version").and_then(|x| x.as_str())) {
            (b.to_string(), v.to_string())
        } else if let Some(vars) = obj.get("variables").and_then(|x| x.as_object()) {
            let name = vars.get("name").and_then(|x| x.as_str())?;
            let version = vars.get("version").and_then(|x| x.as_str())?;
            (name.to_string(), version.to_string())
        } else {
            continue;
        };
        resolved.insert(base, version);
    }
    Some(ContextInfo { resolved })
}

fn env_to_hashmap(env: &Env) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for evar in env.evars_sorted() {
        map.insert(evar.name.clone(), evar.value.clone());
    }
    map
}

fn toml_to_json(value: &toml::Value) -> JsonValue {
    match value {
        toml::Value::String(s) => JsonValue::String(s.clone()),
        toml::Value::Integer(v) => JsonValue::Number((*v).into()),
        toml::Value::Float(v) => serde_json::Number::from_f64(*v)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        toml::Value::Boolean(v) => JsonValue::Bool(*v),
        toml::Value::Datetime(v) => JsonValue::String(v.to_string()),
        toml::Value::Array(values) => {
            JsonValue::Array(values.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(values) => {
            let mut map = serde_json::Map::new();
            for (key, item) in values {
                map.insert(key.clone(), toml_to_json(item));
            }
            JsonValue::Object(map)
        }
    }
}
