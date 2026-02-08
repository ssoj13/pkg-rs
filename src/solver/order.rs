//! Package ordering rules (Rez-style).

use crate::config;
use crate::dep::DepSpec;
use crate::error::SolverError;
use crate::rez_version::Version;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct PackageEntry {
    pub(crate) version: Version,
    pub(crate) deps: Vec<DepSpec>,
    pub(crate) timestamp: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PackageOrderList {
    orderers: Vec<PackageOrder>,
    by_package: HashMap<String, usize>,
}

impl PackageOrderList {
    pub fn from_config(cfg: Option<&config::Config>) -> Result<Option<Self>, SolverError> {
        let value = cfg.and_then(|c| config::get_json(c, "package_orderers"));
        let Some(value) = value else {
            return Ok(None);
        };

        if value.is_null() {
            return Ok(None);
        }

        let mut orderers = Vec::new();
        match value {
            JsonValue::Array(items) => {
                for item in items {
                    orderers.push(parse_orderer(&item)?);
                }
            }
            JsonValue::Object(_) => {
                orderers.push(parse_orderer(&value)?);
            }
            _ => {
                return Err(SolverError::InvalidDepSpec {
                    spec: "package_orderers".to_string(),
                    reason: "package_orderers must be dict or list".to_string(),
                });
            }
        }

        if orderers.is_empty() {
            return Ok(None);
        }

        let mut by_package = HashMap::new();
        for (idx, orderer) in orderers.iter().enumerate() {
            for package in orderer.packages() {
                if !by_package.contains_key(package) {
                    by_package.insert(package.clone(), idx);
                }
            }
        }

        Ok(Some(Self { orderers, by_package }))
    }

    pub(crate) fn reorder(&self, base: &str, entries: &[PackageEntry]) -> Vec<PackageEntry> {
        let orderer = self
            .by_package
            .get(base)
            .or_else(|| self.by_package.get("*"))
            .and_then(|idx| self.orderers.get(*idx));

        if let Some(orderer) = orderer {
            orderer.reorder(base, entries)
        } else {
            PackageOrder::sorted(true, Vec::new()).reorder(base, entries)
        }
    }
}

#[derive(Debug, Clone)]
enum PackageOrder {
    NoOrder { packages: Vec<String> },
    Sorted { descending: bool, packages: Vec<String> },
    VersionSplit { first_version: Version, packages: Vec<String> },
    SoftTimestamp { timestamp: i64, rank: u32, packages: Vec<String> },
    PerFamily {
        orderers: Vec<PackageOrder>,
        default_order: Option<Box<PackageOrder>>,
        packages: Vec<String>,
    },
}

impl PackageOrder {
    fn sorted(descending: bool, packages: Vec<String>) -> Self {
        Self::Sorted {
            descending,
            packages,
        }
    }

    fn packages(&self) -> &[String] {
        match self {
            PackageOrder::NoOrder { packages } => packages,
            PackageOrder::Sorted { packages, .. } => packages,
            PackageOrder::VersionSplit { packages, .. } => packages,
            PackageOrder::SoftTimestamp { packages, .. } => packages,
            PackageOrder::PerFamily { packages, .. } => packages,
        }
    }

    fn reorder(&self, base: &str, entries: &[PackageEntry]) -> Vec<PackageEntry> {
        match self {
            PackageOrder::NoOrder { .. } => entries.to_vec(),
            PackageOrder::Sorted { descending, .. } => {
                sort_versions(entries, *descending)
            }
            PackageOrder::VersionSplit { first_version, .. } => {
                order_version_split(entries, first_version)
            }
            PackageOrder::SoftTimestamp {
                timestamp,
                rank,
                ..
            } => order_soft_timestamp(entries, *timestamp, *rank),
            PackageOrder::PerFamily {
                orderers,
                default_order,
                ..
            } => {
                let mut by_package = HashMap::new();
                for (idx, orderer) in orderers.iter().enumerate() {
                    for package in orderer.packages() {
                        if !by_package.contains_key(package) {
                            by_package.insert(package.clone(), idx);
                        }
                    }
                }
                if let Some(idx) = by_package.get(base) {
                    if let Some(orderer) = orderers.get(*idx) {
                        return orderer.reorder(base, entries);
                    }
                }
                if let Some(orderer) = default_order.as_ref() {
                    return orderer.reorder(base, entries);
                }
                entries.to_vec()
            }
        }
    }
}

fn parse_orderer(value: &JsonValue) -> Result<PackageOrder, SolverError> {
    let obj = value.as_object().ok_or_else(|| SolverError::InvalidDepSpec {
        spec: "package_orderers".to_string(),
        reason: "orderer must be a dict".to_string(),
    })?;

    let order_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SolverError::InvalidDepSpec {
            spec: "package_orderers".to_string(),
            reason: "orderer missing type".to_string(),
        })?
        .to_string();

    let packages_value = obj.get("packages");
    let packages = parse_packages(packages_value);

    match order_type.as_str() {
        "no_order" => Ok(PackageOrder::NoOrder { packages }),
        "sorted" => {
            let descending = obj
                .get("descending")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Ok(PackageOrder::Sorted { descending, packages })
        }
        "version_split" => {
            let first_version = obj
                .get("first_version")
                .and_then(|v| v.as_str())
                .ok_or_else(|| SolverError::InvalidDepSpec {
                    spec: "package_orderers".to_string(),
                    reason: "version_split requires first_version".to_string(),
                })?;
            let first_version = Version::parse(first_version).map_err(|e| SolverError::InvalidDepSpec {
                spec: first_version.to_string(),
                reason: e.to_string(),
            })?;
            Ok(PackageOrder::VersionSplit {
                first_version,
                packages,
            })
        }
        "soft_timestamp" => {
            let timestamp = obj
                .get("timestamp")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| SolverError::InvalidDepSpec {
                    spec: "package_orderers".to_string(),
                    reason: "soft_timestamp requires timestamp".to_string(),
                })?;
            let rank = obj
                .get("rank")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            Ok(PackageOrder::SoftTimestamp {
                timestamp,
                rank,
                packages,
            })
        }
        "per_family" => {
            let mut orderers = Vec::new();
            let items = obj
                .get("orderers")
                .and_then(|v| v.as_array())
                .ok_or_else(|| SolverError::InvalidDepSpec {
                    spec: "package_orderers".to_string(),
                    reason: "per_family requires orderers list".to_string(),
                })?;
            for item in items {
                orderers.push(parse_orderer(item)?);
            }
            let default_order = if let Some(val) = obj.get("default_order") {
                Some(Box::new(parse_orderer(val)?))
            } else {
                None
            };

            let mut families = Vec::new();
            for orderer in &orderers {
                families.extend(orderer.packages().iter().cloned());
            }

            let packages = if packages_value.is_some() {
                parse_packages(packages_value)
            } else {
                families
            };

            Ok(PackageOrder::PerFamily {
                orderers,
                default_order,
                packages,
            })
        }
        _ => Err(SolverError::InvalidDepSpec {
            spec: order_type,
            reason: "unknown orderer type".to_string(),
        }),
    }
}

fn parse_packages(value: Option<&JsonValue>) -> Vec<String> {
    match value {
        None => vec!["*".to_string()],
        Some(JsonValue::String(text)) => vec![text.trim().to_string()],
        Some(JsonValue::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => vec!["*".to_string()],
    }
}

fn sort_versions(entries: &[PackageEntry], descending: bool) -> Vec<PackageEntry> {
    let mut out = entries.to_vec();
    if descending {
        out.sort_by(|a, b| b.version.cmp(&a.version));
    } else {
        out.sort_by(|a, b| a.version.cmp(&b.version));
    }
    out
}

fn order_version_split(entries: &[PackageEntry], first_version: &Version) -> Vec<PackageEntry> {
    let mut before = Vec::new();
    let mut after = Vec::new();

    for entry in entries {
        if entry.version <= *first_version {
            before.push(entry.clone());
        } else {
            after.push(entry.clone());
        }
    }

    before.sort_by(|a, b| b.version.cmp(&a.version));
    after.sort_by(|a, b| b.version.cmp(&a.version));

    before.extend(after);
    before
}

fn order_soft_timestamp(entries: &[PackageEntry], timestamp: i64, rank: u32) -> Vec<PackageEntry> {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| b.version.cmp(&a.version));

    let first_after = calc_first_after(&sorted, timestamp, rank);

    let mut before = Vec::new();
    let mut after = Vec::new();

    for entry in sorted {
        let is_before = match &first_after {
            None => true,
            Some(first) => entry.version < *first,
        };
        if is_before {
            before.push(entry);
        } else {
            after.push(entry);
        }
    }

    before.sort_by(|a, b| b.version.cmp(&a.version));

    if rank == 0 {
        after.sort_by(|a, b| a.version.cmp(&b.version));
    } else {
        let trim_len = (rank.saturating_sub(1)) as usize;
        after.sort_by(|a, b| {
            let ta = a.version.trim(trim_len);
            let tb = b.version.trim(trim_len);
            if ta == tb {
                b.version.cmp(&a.version)
            } else {
                ta.cmp(&tb)
            }
        });
    }

    before.extend(after);
    before
}

fn calc_first_after(entries_desc: &[PackageEntry], timestamp: i64, rank: u32) -> Option<Version> {
    let mut first_after: Option<Version> = None;
    let mut break_idx: Option<usize> = None;
    let mut last_checked: Option<Version> = None;

    for (i, entry) in entries_desc.iter().enumerate() {
        if let Some(ts) = entry.timestamp {
            last_checked = Some(entry.version.clone());
            if ts > timestamp {
                first_after = Some(entry.version.clone());
            } else {
                break_idx = Some(i);
                break;
            }
        }
    }

    if rank == 0 {
        return first_after;
    }

    let Some(last_version) = last_checked else {
        return first_after;
    };

    let trim_len = (rank.saturating_sub(1)) as usize;
    let trimmed = last_version.trim(trim_len);
    let limit = break_idx.unwrap_or(entries_desc.len());

    for entry in entries_desc[..limit].iter().rev() {
        if entry.version.trim(trim_len) != trimmed {
            return Some(entry.version.clone());
        }
    }

    None
}
