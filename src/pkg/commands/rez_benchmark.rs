//! Rez benchmark command.

use crate::cli::BenchmarkArgs;
use pkg_lib::{archive, Solver, Storage};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

pub fn cmd_rez_benchmark(args: &BenchmarkArgs) -> ExitCode {
    let out_dir = args.out.canonicalize().unwrap_or_else(|_| args.out.clone());
    let repo_dir = out_dir.join("packages");

    if args.histogram {
        return match print_histogram(&out_dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("rez benchmark: {}", err);
                ExitCode::FAILURE
            }
        };
    }

    if let Some(compare_dir) = &args.compare {
        return match compare_results(&out_dir, compare_dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("rez benchmark: {}", err);
                ExitCode::FAILURE
            }
        };
    }

    if out_dir.exists() {
        eprintln!("rez benchmark: dir specified by --out must not exist");
        return ExitCode::FAILURE;
    }

    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("rez benchmark: {}", err);
        return ExitCode::FAILURE;
    }

    if let Err(err) = extract_benchmark_repo(&out_dir) {
        eprintln!("rez benchmark: {}", err);
        return ExitCode::FAILURE;
    }

    let requests = match load_requests() {
        Ok(reqs) => reqs,
        Err(err) => {
            eprintln!("rez benchmark: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let storage = match Storage::scan_paths(vec![repo_dir.to_string_lossy().to_string()]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rez benchmark: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let results = run_resolves(&storage, &requests, args.iterations);
    if let Err(err) = write_results(&out_dir, &results) {
        eprintln!("rez benchmark: {}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn load_requests() -> Result<Vec<Vec<String>>, String> {
    let path = benchmark_data_dir().join("requests.json");
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let json: JsonValue = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let list = json.as_array().ok_or_else(|| "invalid requests.json".to_string())?;
    let mut out = Vec::new();
    for reqs in list {
        let arr = reqs
            .as_array()
            .ok_or_else(|| "invalid request list".to_string())?;
        out.push(arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
    }
    Ok(out)
}

fn run_resolves(storage: &Storage, requests: &[Vec<String>], iterations: usize) -> Vec<JsonValue> {
    let mut summaries = Vec::new();
    let packages = storage.packages();
    let solver = Solver::new(packages).ok();

    let mut total = 0usize;
    let mut total_ok = 0usize;
    let mut total_fail = 0usize;
    let mut total_err = 0usize;

    for reqs in requests {
        total += 1;
        let mut summary = serde_json::json!({
            "request": reqs,
        });

        let mut secs = 0.0;
        let mut ok = false;
        let mut resolved: Vec<String> = Vec::new();

        if let Some(solver) = solver.as_ref() {
            let mut error_msg = None;
            for _ in 0..iterations.max(1) {
                let t = Instant::now();
                match solver.solve_reqs(reqs.clone()) {
                    Ok(solution) => {
                        secs += t.elapsed().as_secs_f64();
                        resolved = solution;
                        ok = true;
                    }
                    Err(err) => {
                        secs += t.elapsed().as_secs_f64();
                        error_msg = Some(err.to_string());
                        ok = false;
                        break;
                    }
                }
            }

            let avg = secs / iterations.max(1) as f64;
            if ok {
                total_ok += 1;
                summary["status"] = JsonValue::String("success".to_string());
                summary["resolve_time"] = JsonValue::Number(serde_json::Number::from_f64(avg).unwrap());
                summary["resolved_packages"] = JsonValue::Array(
                    resolved.iter().map(|name| JsonValue::String(name.clone())).collect(),
                );
            } else if let Some(err) = error_msg {
                total_err += 1;
                summary["status"] = JsonValue::String("error".to_string());
                summary["error"] = JsonValue::String(err);
            } else {
                total_fail += 1;
                summary["status"] = JsonValue::String("failed".to_string());
                summary["resolve_time"] = JsonValue::Number(serde_json::Number::from_f64(avg).unwrap());
            }
        } else {
            total_err += 1;
            summary["status"] = JsonValue::String("error".to_string());
            summary["error"] = JsonValue::String("solver init failed".to_string());
        }

        summaries.push(summary);
    }

    let _ = (total, total_ok, total_fail, total_err);
    summaries
}

fn write_results(out_dir: &Path, summaries: &[JsonValue]) -> Result<(), String> {
    let resolves = serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("resolves.json"), resolves).map_err(|e| e.to_string())?;

    let mut resolve_times = Vec::new();
    let mut errors = 0usize;
    let mut fails = 0usize;
    for summary in summaries {
        match summary.get("status").and_then(|v| v.as_str()) {
            Some("success") | Some("failed") => {
                if let Some(t) = summary.get("resolve_time").and_then(|v| v.as_f64()) {
                    resolve_times.push(t);
                }
            }
            Some("error") => errors += 1,
            _ => {}
        }
        if summary.get("status").and_then(|v| v.as_str()) == Some("failed") {
            fails += 1;
        }
    }

    let total_run_time = resolve_times.iter().sum::<f64>();
    let stats = build_stats(resolve_times, errors, fails, total_run_time);
    let stats_str = serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?;
    println!("{}", stats_str);
    fs::write(out_dir.join("summary.json"), stats_str).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_stats(resolve_times: Vec<f64>, errors: usize, fails: usize, total_run_time: f64) -> JsonValue {
    let n = resolve_times.len();
    if n == 0 {
        return serde_json::json!({
            "total_run_time": total_run_time,
            "num_success_resolves": 0,
            "num_error_resolves": errors,
            "num_failed_resolves": fails,
        });
    }

    let mut times = resolve_times.clone();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[n / 2];
    let mean = times.iter().sum::<f64>() / n as f64;
    let min = *times.first().unwrap_or(&0.0);
    let max = *times.last().unwrap_or(&0.0);
    let stddev = (times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / n as f64).sqrt();

    serde_json::json!({
        "total_run_time": total_run_time,
        "num_success_resolves": n,
        "num_error_resolves": errors,
        "num_failed_resolves": fails,
        "median": median,
        "mean": mean,
        "min": min,
        "max": max,
        "stddev": stddev,
    })
}

fn print_histogram(out_dir: &Path) -> Result<(), String> {
    let content = fs::read_to_string(out_dir.join("resolves.json")).map_err(|e| e.to_string())?;
    let summaries: Vec<JsonValue> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let mut resolve_times = Vec::new();
    for summary in summaries {
        if let Some(t) = summary.get("resolve_time").and_then(|v| v.as_f64()) {
            resolve_times.push(t);
        }
    }
    if resolve_times.is_empty() {
        return Err("no resolve times found".to_string());
    }

    let rows = 40usize;
    let cols = 40usize;
    let min = resolve_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = resolve_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let bucket = (max - min) / rows as f64;
    let mut buckets = vec![0usize; rows];
    for t in resolve_times {
        let mut idx = ((t - min) / bucket) as usize;
        if idx >= rows {
            idx = rows - 1;
        }
        buckets[idx] += 1;
    }

    let max_bucket = *buckets.iter().max().unwrap_or(&1);
    let mult = cols as f64 / max_bucket as f64;
    for i in 0..rows {
        let start = min + bucket * i as f64;
        let end = start + bucket;
        let left = format!("[{:.2}-{:.2}]", start, end);
        let bar_len = (buckets[i] as f64 * mult).round() as usize;
        println!("{:>18} |{}", left, "#".repeat(bar_len));
    }

    Ok(())
}

fn compare_results(out_dir: &Path, other_dir: &Path) -> Result<(), String> {
    let summary1: JsonValue = serde_json::from_str(&fs::read_to_string(out_dir.join("summary.json")).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let summary2: JsonValue = serde_json::from_str(&fs::read_to_string(other_dir.join("summary.json")).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let mut delta = serde_json::Map::new();
    for field in ["max", "min", "mean", "median", "stddev"] {
        let v1 = summary1.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let v2 = summary2.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
        if v1 == 0.0 {
            continue;
        }
        let diff = v2 - v1;
        let pct = 100.0 * (diff / v1);
        delta.insert(
            format!("{}_delta", field),
            JsonValue::Array(vec![JsonValue::from(diff), JsonValue::String(format!("{:+.2}%", pct))]),
        );
    }

    let out = JsonValue::Object(delta);
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    Ok(())
}

fn extract_benchmark_repo(out_dir: &Path) -> Result<(), String> {
    let zip_path = benchmark_data_dir().join("packages.zip");
    archive::extract_zip(&zip_path, out_dir)
}

fn benchmark_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("rez")
        .join("data")
        .join("benchmarking")
}
