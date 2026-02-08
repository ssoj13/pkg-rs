//! MSVC environment bootstrap (vcv-rs port).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MsvcEnvReport {
    pub vs_version: String,
    pub tools_version: String,
    pub sdk_version: Option<String>,
    pub ucrt_version: Option<String>,
    pub host: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub enum MsvcEnvState {
    Applied(MsvcEnvReport),
    Skipped,
    Failed(String),
}

#[cfg(windows)]
mod win {
    use super::{MsvcEnvReport, MsvcEnvState};
    use crate::config;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::process::Command;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Arch {
        X64,
        X86,
        Arm64,
    }

    impl Arch {
        fn as_str(self) -> &'static str {
            match self {
                Arch::X64 => "x64",
                Arch::X86 => "x86",
                Arch::Arm64 => "arm64",
            }
        }

        fn from_str(value: &str) -> Option<Self> {
            match value.to_ascii_lowercase().as_str() {
                "x64" | "amd64" => Some(Arch::X64),
                "x86" | "win32" => Some(Arch::X86),
                "arm64" => Some(Arch::Arm64),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    struct VsInfo {
        install: PathBuf,
        version: String,
        vc: PathBuf,
        tools_ver: String,
        tools: PathBuf,
    }

    #[derive(Debug)]
    struct SdkInfo {
        path: PathBuf,
        version: String,
    }

    #[derive(Deserialize)]
    struct VsWhereEntry {
        #[serde(rename = "installationPath")]
        installation_path: String,
        #[serde(rename = "installationVersion", default)]
        installation_version: String,
    }

    pub fn ensure(env: &mut HashMap<String, String>) -> MsvcEnvState {
        if !should_apply(env) {
            return MsvcEnvState::Skipped;
        }

        let vs_year = read_vs_year(env);
        let host = read_arch(env, "PKG_MSVC_HOST", "VSCMD_ARG_HOST_ARCH").unwrap_or(Arch::X64);
        let target = read_arch(env, "PKG_MSVC_TARGET", "VSCMD_ARG_TGT_ARCH").unwrap_or(Arch::X64);

        let vs = match detect_vs(vs_year) {
            Some(vs) => vs,
            None => {
                let installed = list_vs_versions();
                let hint = if installed.is_empty() {
                    "vswhere.exe not found".to_string()
                } else {
                    format!("installed: {:?}", installed)
                };
                return MsvcEnvState::Failed(format!(
                    "Visual Studio not found ({})",
                    hint
                ));
            }
        };

        let sdk = detect_sdk();
        if sdk.is_none() {
            return MsvcEnvState::Failed(
                "Windows SDK not found (install Windows 10/11 SDK via Visual Studio Installer)"
                    .to_string(),
            );
        }
        let ucrt = detect_ucrt();
        if ucrt.is_none() {
            return MsvcEnvState::Failed(
                "UCRT not found (install Windows 10/11 SDK via Visual Studio Installer)"
                    .to_string(),
            );
        }

        let built = build_env(&vs, sdk.as_ref(), ucrt.as_ref(), host, target);
        apply_env(env, &built);

        let report = MsvcEnvReport {
            vs_version: vs.version,
            tools_version: vs.tools_ver,
            sdk_version: sdk.map(|s| s.version),
            ucrt_version: ucrt.map(|s| s.version),
            host: host.as_str().to_string(),
            target: target.as_str().to_string(),
        };

        MsvcEnvState::Applied(report)
    }

    fn should_apply(env: &HashMap<String, String>) -> bool {
        if is_disabled(env, "PKG_MSVC_AUTO") {
            return false;
        }
        if let Ok(cfg) = config::get() {
            if let Some(flag) = config::get_bool(cfg, "plugins.pkg_rs.msvc_auto") {
                if !flag {
                    return false;
                }
            }
        }

        let has_lib = env
            .get("LIB")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        let has_include = env
            .get("INCLUDE")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        let has_vc = env
            .get("VCINSTALLDIR")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);

        !(has_lib && has_include && has_vc)
    }

    fn is_disabled(env: &HashMap<String, String>, key: &str) -> bool {
        let value = env
            .get(key)
            .cloned()
            .or_else(|| std::env::var(key).ok());
        value
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
            .unwrap_or(false)
    }

    fn read_vs_year(env: &HashMap<String, String>) -> Option<u16> {
        let raw = env
            .get("PKG_MSVC_VS_YEAR")
            .cloned()
            .or_else(|| std::env::var("PKG_MSVC_VS_YEAR").ok())
            .or_else(|| {
                config::get()
                    .ok()
                    .and_then(|cfg| config::get_str(cfg, "plugins.pkg_rs.msvc_vs_year"))
            });
        raw.and_then(|v| v.parse::<u16>().ok())
    }

    fn read_arch(env: &HashMap<String, String>, key: &str, fallback_key: &str) -> Option<Arch> {
        let raw = env
            .get(key)
            .cloned()
            .or_else(|| std::env::var(key).ok())
            .or_else(|| env.get(fallback_key).cloned())
            .or_else(|| std::env::var(fallback_key).ok());
        raw.and_then(|v| Arch::from_str(&v))
    }

    fn read_txt(path: &PathBuf) -> Option<String> {
        std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn build_vs_info(vs: VsWhereEntry) -> Option<VsInfo> {
        let install = PathBuf::from(&vs.installation_path);
        let vc = install.join("VC");
        let aux = vc.join("Auxiliary").join("Build");

        let tools_ver = read_txt(&aux.join("Microsoft.VCToolsVersion.v143.default.txt"))
            .or_else(|| read_txt(&aux.join("Microsoft.VCToolsVersion.default.txt")))?;

        let tools = vc.join("Tools").join("MSVC").join(&tools_ver);
        if !tools.exists() {
            return None;
        }

        Some(VsInfo {
            install,
            version: vs.installation_version,
            vc,
            tools_ver,
            tools,
        })
    }

    fn detect_vs(vs_year: Option<u16>) -> Option<VsInfo> {
        let vswhere = PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
        if !vswhere.exists() {
            return None;
        }

        let output = Command::new(&vswhere)
            .args(["-all", "-format", "json", "-utf8"])
            .output()
            .ok()?;

        let entries: Vec<VsWhereEntry> = serde_json::from_slice(&output.stdout).ok()?;

        let filtered: Vec<_> = if let Some(year) = vs_year {
            let major = match year {
                2017 => "15.",
                2019 => "16.",
                2022 => "17.",
                _ => return None,
            };
            entries
                .into_iter()
                .filter(|e| e.installation_version.starts_with(major))
                .collect()
        } else {
            entries
        };

        let mut sorted = filtered;
        sorted.sort_by(|a, b| b.installation_version.cmp(&a.installation_version));

        sorted.into_iter().find_map(build_vs_info)
    }

    fn list_vs_versions() -> Vec<(u16, String)> {
        let vswhere = PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
        if !vswhere.exists() {
            return vec![];
        }

        let output = match Command::new(&vswhere)
            .args(["-all", "-format", "json", "-utf8"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        let entries: Vec<VsWhereEntry> = match serde_json::from_slice(&output.stdout) {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        entries
            .into_iter()
            .filter_map(|e| {
                let year = if e.installation_version.starts_with("17.") {
                    2022
                } else if e.installation_version.starts_with("16.") {
                    2019
                } else if e.installation_version.starts_with("15.") {
                    2017
                } else {
                    return None;
                };
                Some((year, e.installation_version))
            })
            .collect()
    }

    fn reg_val(root: &RegKey, path: &str, name: &str) -> Option<String> {
        root.open_subkey(path)
            .ok()
            .and_then(|k| k.get_value::<String, _>(name).ok())
    }

    fn reg_find(path: &str, name: &str) -> Option<String> {
        let roots = [
            RegKey::predef(HKEY_LOCAL_MACHINE),
            RegKey::predef(HKEY_CURRENT_USER),
        ];
        let prefixes = [r"SOFTWARE\Wow6432Node", r"SOFTWARE"];

        for root in &roots {
            for prefix in &prefixes {
                let full_path = format!(r"{}\{}", prefix, path);
                if let Some(val) = reg_val(root, &full_path, name) {
                    return Some(val);
                }
            }
        }
        None
    }

    fn detect_sdk() -> Option<SdkInfo> {
        let sdk_path = reg_find(r"Microsoft\Microsoft SDKs\Windows\v10.0", "InstallationFolder")?;
        let root = PathBuf::from(sdk_path);
        let inc = root.join("include");
        if !inc.exists() {
            return None;
        }

        let mut versions: Vec<_> = std::fs::read_dir(&inc)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("10.") && e.path().join("um").join("winsdkver.h").exists()
            })
            .collect();

        versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        let version = versions.first()?.file_name().to_string_lossy().to_string();

        Some(SdkInfo { path: root, version })
    }

    fn detect_ucrt() -> Option<SdkInfo> {
        let ucrt_path = reg_find(r"Microsoft\Windows Kits\Installed Roots", "KitsRoot10")?;
        let root = PathBuf::from(ucrt_path);
        let lib = root.join("Lib");
        if !lib.exists() {
            return None;
        }

        let mut versions: Vec<_> = std::fs::read_dir(&lib)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("10.")
                    && e.path().join("ucrt").join("x64").join("ucrt.lib").exists()
            })
            .collect();

        versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        let version = versions.first()?.file_name().to_string_lossy().to_string();

        Some(SdkInfo { path: root, version })
    }

    #[derive(Debug, Default)]
    struct Env {
        path: Vec<PathBuf>,
        include: Vec<PathBuf>,
        lib: Vec<PathBuf>,
        libpath: Vec<PathBuf>,
        vars: BTreeMap<String, String>,
    }

    impl Env {
        fn add_if_exists(lst: &mut Vec<PathBuf>, paths: &[PathBuf]) {
            for p in paths {
                if p.exists() {
                    lst.push(p.clone());
                }
            }
        }
    }

    fn build_env(vs: &VsInfo, sdk: Option<&SdkInfo>, ucrt: Option<&SdkInfo>, host: Arch, target: Arch) -> Env {
        let mut env = Env::default();
        let tp = &vs.tools;

        let hd = match host {
            Arch::X64 => "Hostx64",
            Arch::X86 => "Hostx86",
            Arch::Arm64 => "Hostarm64",
        };
        let tgt = target.as_str();

        Env::add_if_exists(&mut env.path, &[tp.join("bin").join(hd).join(tgt)]);
        if host != target {
            let host_str = host.as_str();
            Env::add_if_exists(&mut env.path, &[tp.join("bin").join(hd).join(host_str)]);
        }

        Env::add_if_exists(&mut env.include, &[
            tp.join("include"),
            tp.join("ATLMFC").join("include"),
        ]);
        Env::add_if_exists(&mut env.lib, &[
            tp.join("lib").join(tgt),
            tp.join("ATLMFC").join("lib").join(tgt),
        ]);
        Env::add_if_exists(&mut env.libpath, &[
            tp.join("lib").join(tgt),
            tp.join("ATLMFC").join("lib").join(tgt),
        ]);

        if let Some(sdk) = sdk {
            let sp = &sdk.path;
            let sv = &sdk.version;
            let host_str = host.as_str();

            Env::add_if_exists(&mut env.path, &[sp.join("bin").join(sv).join(host_str)]);
            Env::add_if_exists(&mut env.include, &[
                sp.join("include").join(sv).join("um"),
                sp.join("include").join(sv).join("shared"),
                sp.join("include").join(sv).join("winrt"),
                sp.join("include").join(sv).join("cppwinrt"),
            ]);
            Env::add_if_exists(&mut env.lib, &[sp.join("lib").join(sv).join("um").join(tgt)]);
            Env::add_if_exists(&mut env.libpath, &[
                sp.join("UnionMetadata").join(sv),
                sp.join("References").join(sv),
            ]);
        }

        if let Some(ucrt) = ucrt {
            let up = &ucrt.path;
            let uv = &ucrt.version;

            Env::add_if_exists(&mut env.include, &[up.join("include").join(uv).join("ucrt")]);
            Env::add_if_exists(&mut env.lib, &[up.join("lib").join(uv).join("ucrt").join(tgt)]);
        }

        env.vars.insert("VSINSTALLDIR".into(), format!("{}\\", vs.install.display()));
        env.vars.insert("VCINSTALLDIR".into(), format!("{}\\", vs.vc.display()));
        env.vars.insert("VCToolsInstallDir".into(), format!("{}\\", tp.display()));
        env.vars.insert("VCToolsVersion".into(), vs.tools_ver.clone());
        env.vars.insert("VisualStudioVersion".into(), "17.0".into());
        env.vars.insert("Platform".into(), tgt.into());
        env.vars.insert("VSCMD_ARG_HOST_ARCH".into(), host.as_str().into());
        env.vars.insert("VSCMD_ARG_TGT_ARCH".into(), target.as_str().into());

        if let Some(sdk) = sdk {
            env.vars.insert("WindowsSdkDir".into(), format!("{}\\", sdk.path.display()));
            env.vars.insert("WindowsSDKVersion".into(), format!("{}\\", sdk.version));
        }

        if let Some(ucrt) = ucrt {
            env.vars.insert("UniversalCRTSdkDir".into(), format!("{}\\", ucrt.path.display()));
            env.vars.insert("UCRTVersion".into(), ucrt.version.clone());
        }

        env
    }

    fn apply_env(env: &mut HashMap<String, String>, built: &Env) {
        let sep = ";";
        prepend_list(env, "PATH", &built.path, sep);
        prepend_list(env, "INCLUDE", &built.include, sep);
        prepend_list(env, "LIB", &built.lib, sep);
        prepend_list(env, "LIBPATH", &built.libpath, sep);

        for (k, v) in &built.vars {
            env.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    fn prepend_list(env: &mut HashMap<String, String>, key: &str, values: &[PathBuf], sep: &str) {
        if values.is_empty() {
            return;
        }
        let joined = values
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(sep);
        match env.get(key) {
            Some(existing) if !existing.trim().is_empty() => {
                env.insert(key.to_string(), format!("{}{}{}", joined, sep, existing));
            }
            _ => {
                env.insert(key.to_string(), joined);
            }
        }
    }
}

#[cfg(windows)]
pub fn ensure_msvc_env(env: &mut HashMap<String, String>) -> MsvcEnvState {
    win::ensure(env)
}

#[cfg(not(windows))]
pub fn ensure_msvc_env(_env: &mut HashMap<String, String>) -> MsvcEnvState {
    MsvcEnvState::Skipped
}
