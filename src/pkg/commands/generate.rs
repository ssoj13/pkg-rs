//! Generate test repository command.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Convert snake_case to Title Case (maya -> Maya, houdini_engine -> Houdini Engine)
fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate test repository with random packages for stress-testing.
pub fn cmd_generate_repo(
    output: PathBuf,
    num_packages: usize,
    versions_per_pkg: usize,
    max_depth: usize,
    dep_rate: f64,
    seed: Option<u64>,
) -> ExitCode {
    // Calculate total and ask for confirmation
    let total = num_packages * versions_per_pkg;
    println!(
        "Will generate {} packages x {} versions = {} package versions",
        num_packages, versions_per_pkg, total
    );
    println!("Output: {}", output.display());
    print!("Continue? [Y/n] ");
    let _ = std::io::stdout().flush();
    
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        eprintln!("Failed to read input");
        return ExitCode::FAILURE;
    }
    let input = input.trim().to_lowercase();
    // Y is default - only abort on explicit "n" or "no"
    if input == "n" || input == "no" {
        println!("Aborted.");
        return ExitCode::SUCCESS;
    }

    // Check if directory exists
    if output.exists() {
        eprintln!("Error: Directory already exists: {}", output.display());
        eprintln!("Hint: Use an empty directory or remove existing one");
        return ExitCode::FAILURE;
    }
    
    // Create output directory
    if let Err(e) = std::fs::create_dir_all(&output) {
        eprintln!("Failed to create directory: {}", e);
        return ExitCode::FAILURE;
    }
    
    // Simple LCG random number generator
    struct Rng(u64);
    impl Rng {
        fn new(seed: u64) -> Self { Self(seed) }
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.0
        }
        fn next_f64(&mut self) -> f64 {
            (self.next() as f64) / (u64::MAX as f64)
        }
        fn range(&mut self, min: usize, max: usize) -> usize {
            min + (self.next() as usize % (max - min + 1))
        }
    }
    
    let mut rng = Rng::new(seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42)
    }));
    
    // Realistic VFX software names - DCCs, renderers, plugins
    let vfx_software = [
        // === DCCs ===
        "maya", "houdini", "blender", "cinema4d", "max", "nuke", "fusion",
        "flame", "resolve", "aftereffects", "modo", "katana", "clarisse",
        
        // === Renderers ===
        "arnold", "vray", "redshift", "octane", "renderman", "karma", "mantra",
        "corona", "cycles", "maxwell", "guerilla", "iray",
        
        // === Maya Plugins ===
        "mtoa", "rfm", "golaem", "ornatrix", "yeti", "phoenix_fd", "fumefx",
        "soup", "advanced_skeleton", "mgear", "ziva", "qualoth", "mash", "xgen",
        "bifrost", "boss", "rapid_rig", "animbot", "tween_machine", "studio_library",
        "ngskintool",
        
        // === Houdini Plugins ===
        "htoa", "mops", "mops_plus", "od_tools", "modeler", "qlib", "sidefx_labs",
        "kinefx", "apex", "vex_snippets", "aelib", "eglib",
        
        // === Nuke Plugins ===
        "ocula", "furnace", "neat_video", "twixtor", "facebuilder", "geotracker",
        "mocha_pro", "silhouette", "splinewarp", "smartvector", "nuke_survival",
        "nukepedia_tools", "w_hotbox", "flow_warp",
        
        // === Cinema 4D Plugins ===
        "x_particles", "turbulence_fd", "forester", "greyscalegorilla", "hb_modelling",
        "drop_to_floor", "nitroblast", "cv_vrcam", "magic_solo", "rocket_lasso",
        "signal", "transform",
        
        // === Simulation ===
        "houdini_engine", "realflow", "embergen", "tyflow", "thinking_particles",
        
        // === Texturing ===
        "substance_painter", "substance_designer", "mari", "mudbox", "zbrush",
        "quixel_mixer", "armorpaint",
        
        // === Pipeline/Core ===
        "usd", "alembic", "openvdb", "openexr", "ocio", "oiio", "ptex", "materialx",
        "aces", "openimageio",
        
        // === Tracking/Matchmove ===
        "pftrack", "syntheyes", "equalizer", "boujou",
        
        // === Review/Dailies ===
        "rv", "djv", "mrviewer", "pdplayer", "cinesync",
        
        // === Asset Management ===
        "shotgrid", "ftrack", "prism", "kitsu", "anchorpoint", "ayon",
        
        // === Farm/Render Management ===
        "deadline", "tractor", "qube", "royalrender", "opencue", "afanasy",
        
        // === Scripting/Dev ===
        "python", "pyqt", "pyside", "numpy", "scipy", "opencv", "pillow",
        
        // === Utilities ===
        "ffmpeg", "imagemagick", "oiiotool", "txmake",
    ];
    
    // Required packages - always first
    let required = [
        "maya", "houdini", "nuke", "aftereffects", "resolve",
        "arnold", "vray", "redshift", "usd", "python",
    ];
    
    // Build name pool: required + rest + synthetic overflow
    let mut pool: Vec<String> = required.iter().map(|s| s.to_string()).collect();
    for &name in &vfx_software {
        if !required.contains(&name) {
            pool.push(name.to_string());
        }
    }
    // Extend with synthetic if needed
    let mut ext = 1;
    while pool.len() < num_packages {
        let base = vfx_software[ext % vfx_software.len()];
        pool.push(format!("{}_ext{}", base, ext));
        ext += 1;
    }
    
    let pkg_names: Vec<String> = pool.into_iter().take(num_packages).collect();
    
    println!("Generating {} packages with {} versions each...", num_packages, versions_per_pkg);
    
    let mut total_versions = 0;
    let mut total_deps = 0;
    
    for (pkg_idx, pkg_name) in pkg_names.iter().enumerate() {
        let pkg_dir = output.join(pkg_name);
        
        for v in 0..versions_per_pkg {
            // Generate realistic versions based on software type
            let version = match pkg_name.as_str() {
                "maya" | "max" | "flame" | "smoke" => {
                    let year = 2023 + v;
                    let patch = rng.range(0, 3);
                    format!("{}.{}.0", year, patch)
                }
                "houdini" | "houdini_engine" | "karma" | "mantra" => {
                    let major = 19 + (v / 2);
                    let minor = v % 2;
                    let build = rng.range(100, 999);
                    format!("{}.{}.{}", major, minor, build)
                }
                "nuke" | "nuke_tracker" => {
                    let major = 13 + v;
                    let minor = rng.range(0, 3);
                    let patch = rng.range(1, 9);
                    format!("{}.{}.{}", major, minor, patch)
                }
                _ => {
                    let major = 1 + (v / 3);
                    let minor = v % 3;
                    let patch = rng.range(0, 15);
                    format!("{}.{}.{}", major, minor, patch)
                }
            };
            
            let version_dir = pkg_dir.join(&version);
            if let Err(e) = std::fs::create_dir_all(&version_dir) {
                eprintln!("Failed to create {}: {}", version_dir.display(), e);
                continue;
            }
            
            // Generate dependencies (only from earlier packages to avoid cycles)
            let mut deps = Vec::new();
            let dep_count = if pkg_idx > 0 && rng.next_f64() < dep_rate {
                rng.range(1, max_depth.min(pkg_idx))
            } else {
                0
            };
            
            for _ in 0..dep_count {
                let dep_idx = rng.range(0, pkg_idx - 1);
                let dep_name = &pkg_names[dep_idx];
                if !deps.contains(dep_name) {
                    deps.push(dep_name.clone());
                }
            }
            total_deps += deps.len();
            
            // Generate package.py with cross-platform paths
            let mut content = String::new();
            content.push_str("# Auto-generated package for stress testing\n");
            content.push_str(&format!("# Package: {} v{}\n\n", pkg_name, version));
            content.push_str("import sys\n");
            content.push_str("from pathlib import Path\n\n");
            
            // Generate platform-aware root path
            let pkg_title = to_title_case(pkg_name);
            content.push_str(&format!("# Platform-specific install paths\n"));
            content.push_str("if sys.platform == 'win32':\n");
            content.push_str(&format!("    ROOT = Path(r'C:/Program Files/{}/{}')\n", pkg_title, version));
            content.push_str("elif sys.platform == 'darwin':\n");
            content.push_str(&format!("    ROOT = Path('/Applications/{}/{}')\n", pkg_title, version));
            content.push_str("else:\n");
            content.push_str(&format!("    ROOT = Path('/opt/{}/{}')\n\n", pkg_name, version));
            
            content.push_str("def get_package():\n");
            content.push_str(&format!("    p = Package(\"{}\", \"{}\")\n", pkg_name, version));
            
            for dep in &deps {
                content.push_str(&format!("    p.add_req(\"{}\")\n", dep));
            }
            
            content.push_str("\n    env = Env(\"default\")\n");
            content.push_str(&format!(
                "    env.add(Evar(\"{}_ROOT\", str(ROOT), \"set\"))\n",
                pkg_name.to_uppercase()
            ));
            content.push_str(
                "    env.add(Evar(\"PATH\", str(ROOT / 'bin'), \"insert\"))\n"
            );
            
            // Add DCC-specific env vars randomly
            let dcc_vars: &[(&str, &[&str])] = &[
                // Maya vars
                ("maya", &["MAYA_PLUG_IN_PATH", "MAYA_SCRIPT_PATH", "MAYA_ICON_PATH", 
                          "MAYA_MODULE_PATH", "MAYA_PRESET_PATH", "XBMLANGPATH"]),
                ("mtoa", &["MAYA_PLUG_IN_PATH", "MAYA_SCRIPT_PATH", "MTOA_TEMPLATES_PATH"]),
                ("arnold", &["ARNOLD_PLUGIN_PATH", "ARNOLD_PROCEDURAL_PATH"]),
                ("yeti", &["MAYA_PLUG_IN_PATH", "YETI_HOME", "YETI_TMP"]),
                ("golaem", &["MAYA_PLUG_IN_PATH", "GOLAEM_LICENSE", "GOLAEM_TMP"]),
                ("bifrost", &["BIFROST_LIB_CONFIG_FILES", "MAYA_PLUG_IN_PATH"]),
                ("xgen", &["XGEN_LOCATION", "MAYA_PLUG_IN_PATH"]),
                // Houdini vars
                ("houdini", &["HOUDINI_PATH", "HOUDINI_OTLSCAN_PATH", "HOUDINI_DSO_PATH",
                             "HOUDINI_GALLERY_PATH", "HOUDINI_MENU_PATH", "HSITE"]),
                ("htoa", &["HOUDINI_PATH", "HOUDINI_OTLSCAN_PATH"]),
                ("sidefx_labs", &["HOUDINI_PATH", "HOUDINI_OTLSCAN_PATH"]),
                ("mops", &["HOUDINI_PATH", "HOUDINI_OTLSCAN_PATH"]),
                ("qlib", &["HOUDINI_PATH", "QLIB", "QLIB_HOUDINI_PATH"]),
                ("karma", &["HOUDINI_PATH", "KARMA_PROCEDURALS_PATH"]),
                // Nuke vars
                ("nuke", &["NUKE_PATH", "NUKE_GIZMO_PATH", "NUKE_PLUGIN_PATH",
                          "NUKE_TEMP_DIR", "FOUNDRY_LICENSE_FILE"]),
                ("mocha_pro", &["NUKE_PATH", "MOCHA_LICENSE"]),
                ("furnace", &["NUKE_PATH", "FURNACE_LICENSE"]),
                // Common
                ("python", &["PYTHONPATH", "PYTHONHOME"]),
                ("usd", &["PXR_PLUGINPATH_NAME", "USD_ASSET_RESOLVER"]),
                ("ocio", &["OCIO", "OCIO_ACTIVE_DISPLAYS"]),
                ("redshift", &["REDSHIFT_COREDATAPATH", "REDSHIFT_PLUG_IN_PATH"]),
                ("vray", &["VRAY_PATH", "VRAY_AUTH_CLIENT_FILE_PATH"]),
            ];
            
            // Find matching DCC vars and add some randomly
            for (pattern, vars) in dcc_vars {
                if pkg_name.contains(pattern) {
                    for var in *vars {
                        if rng.next_f64() < 0.5 {
                            let subdir = match *var {
                                v if v.contains("SCRIPT") || v.contains("PYTHON") => "scripts",
                                v if v.contains("PLUG") || v.contains("DSO") => "plug-ins",
                                v if v.contains("ICON") || v.contains("XBML") => "icons",
                                v if v.contains("OTL") || v.contains("GIZMO") => "otls",
                                v if v.contains("MENU") => "menus",
                                v if v.contains("MODULE") || v.contains("PATH") => "modules",
                                _ => "lib",
                            };
                            content.push_str(&format!(
                                "    env.add(Evar(\"{}\", str(ROOT / '{}'), \"append\"))\n",
                                var, subdir
                            ));
                        }
                    }
                    break;
                }
            }
            
            content.push_str("    p.add_env(env)\n");
            
            if rng.next_f64() < 0.3 {
                content.push_str(&format!("\n    app = App(\"{}\")\n", pkg_name));
                content.push_str("    exe = '.exe' if sys.platform == 'win32' else ''\n");
                content.push_str(&format!(
                    "    app.path = str(ROOT / 'bin' / f'{}{{exe}}')\n",
                    pkg_name
                ));
                content.push_str("    p.add_app(app)\n");
            }
            
            content.push_str("\n    return p\n");
            
            let py_path = version_dir.join("package.py");
            match std::fs::File::create(&py_path) {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(content.as_bytes()) {
                        eprintln!("Failed to write {}: {}", py_path.display(), e);
                    }
                }
                Err(e) => eprintln!("Failed to create {}: {}", py_path.display(), e),
            }
            
            total_versions += 1;
        }
    }
    
    // Generate .toolsets/*.toml
    generate_toolsets(&output, &pkg_names);
    
    println!("Generated {} package versions in {}", total_versions, output.display());
    println!("Total dependencies: {} (avg {:.2}/pkg)", total_deps, total_deps as f64 / total_versions as f64);
    
    ExitCode::SUCCESS
}

/// Generate toolset TOML files.
fn generate_toolsets(output: &PathBuf, pkg_names: &[String]) {
    let toolsets_dir = output.join(".toolsets");
    if let Err(e) = std::fs::create_dir_all(&toolsets_dir) {
        eprintln!("Failed to create .toolsets dir: {}", e);
        return;
    }
    
    let toolset_defs = [
        ("maya.toml", vec![
            ("maya-anim", "Maya animation", vec![
                "maya", "python", "numpy", "pyside",
                "advanced_skeleton", "mgear", "animbot", "tween_machine", 
                "studio_library", "ngskintool", "rapid_rig",
                "arnold", "mtoa", "ocio"
            ]),
            ("maya-fx", "Maya FX/simulation", vec![
                "maya", "python", "numpy", "pyside",
                "bifrost", "phoenix_fd", "fumefx", "mash", "boss",
                "redshift", "arnold", "mtoa",
                "usd", "alembic", "ocio"
            ]),
            ("maya-lookdev", "Maya lookdev/lighting", vec![
                "maya", "python", "numpy", "pyside",
                "redshift", "arnold", "mtoa", "vray",
                "substance_painter", "mari",
                "usd", "materialx", "ocio", "aces"
            ]),
            ("maya-groom", "Maya grooming", vec![
                "maya", "python", "pyside",
                "xgen", "yeti", "ornatrix",
                "arnold", "mtoa", "redshift"
            ]),
            ("maya-crowd", "Maya crowd", vec![
                "maya", "python", "pyside",
                "golaem", "mash",
                "arnold", "mtoa", "alembic"
            ]),
        ]),
        ("houdini.toml", vec![
            ("houdini-fx", "Houdini FX", vec![
                "houdini", "python", "numpy", "pyside",
                "karma", "mantra", "sidefx_labs", "mops", "mops_plus",
                "qlib", "od_tools", "aelib", "kinefx", "apex",
                "usd", "alembic", "openvdb", "ocio"
            ]),
            ("houdini-redshift", "Houdini with Redshift", vec![
                "houdini", "python", "numpy",
                "redshift", "sidefx_labs", "mops",
                "usd", "alembic", "ocio"
            ]),
            ("houdini-arnold", "Houdini with Arnold", vec![
                "houdini", "python", "numpy",
                "arnold", "htoa", "sidefx_labs", "qlib",
                "usd", "alembic", "ocio"
            ]),
        ]),
        ("nuke.toml", vec![
            ("nuke-comp", "Nuke compositing", vec![
                "nuke", "python", "numpy", "pyside",
                "mocha_pro", "neat_video", "furnace", "nuke_survival",
                "smartvector", "splinewarp", "flow_warp",
                "ocio", "aces", "openexr"
            ]),
            ("nuke-3d", "Nuke 3D/tracking", vec![
                "nuke", "python", "numpy",
                "facebuilder", "geotracker", "ocula", "pftrack",
                "alembic", "usd", "ocio"
            ]),
            ("nuke-roto", "Nuke roto/paint", vec![
                "nuke", "python",
                "silhouette", "mocha_pro", "nuke_survival",
                "ocio"
            ]),
        ]),
        ("c4d.toml", vec![
            ("c4d-mograph", "C4D motion graphics", vec![
                "cinema4d", "python",
                "redshift", "octane",
                "x_particles", "greyscalegorilla", "turbulence_fd",
                "ocio"
            ]),
            ("c4d-arch", "C4D archviz", vec![
                "cinema4d", "python",
                "corona", "vray", "forester",
                "quixel_mixer", "ocio"
            ]),
        ]),
        ("pipeline.toml", vec![
            ("usd-core", "USD pipeline core", vec![
                "usd", "alembic", "openvdb", "openexr", "oiio",
                "materialx", "ocio", "aces", "ptex",
                "python", "numpy"
            ]),
            ("farm-deadline", "Deadline render farm", vec![
                "deadline", "python", "ffmpeg", "oiiotool", "imagemagick"
            ]),
            ("review-daily", "Review/dailies", vec![
                "rv", "djv", "mrviewer",
                "shotgrid", "ftrack", "kitsu",
                "ffmpeg", "ocio", "openexr"
            ]),
            ("track-matchmove", "Tracking/matchmove", vec![
                "pftrack", "syntheyes", "equalizer",
                "python", "opencv", "numpy"
            ]),
        ]),
    ];
    
    let mut toolset_count = 0;
    for (filename, toolsets) in &toolset_defs {
        let toml_path = toolsets_dir.join(filename);
        let mut content = String::new();
        content.push_str("# Auto-generated toolsets for testing\n\n");
        
        for (name, desc, reqs) in toolsets {
            let valid_reqs: Vec<&str> = reqs.iter()
                .filter(|r| pkg_names.iter().any(|p| p == *r))
                .copied()
                .collect();
            
            if valid_reqs.is_empty() {
                continue;
            }
            
            content.push_str(&format!("[{}]\n", name));
            content.push_str(&format!("description = \"{}\"\n", desc));
            content.push_str("requires = [\n");
            for req in &valid_reqs {
                content.push_str(&format!("    \"{}\",\n", req));
            }
            content.push_str("]\n\n");
            toolset_count += 1;
        }
        
        if !content.trim().is_empty() && content.contains('[') {
            if let Err(e) = std::fs::write(&toml_path, &content) {
                eprintln!("Failed to write {}: {}", toml_path.display(), e);
            }
        }
    }
    
    println!("Generated {} toolsets in .toolsets/", toolset_count);
}
