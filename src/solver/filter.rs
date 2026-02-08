//! Package filter rules (Rez-style).

use crate::config;
use crate::dep::DepSpec;
use crate::error::SolverError;
use crate::Package;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PackageFilterList {
    filters: Vec<PackageFilter>,
}

impl PackageFilterList {
    pub fn from_config(cfg: Option<&config::Config>) -> Result<Option<Self>, SolverError> {
        let value = cfg.and_then(|c| config::get_json(c, "package_filter"));
        let Some(value) = value else {
            return Ok(None);
        };

        if value.is_null() {
            return Ok(None);
        }

        let mut filters = Vec::new();
        match value {
            JsonValue::Array(items) => {
                for item in items {
                    filters.push(PackageFilter::from_pod(&item)?);
                }
            }
            JsonValue::Object(_) => {
                filters.push(PackageFilter::from_pod(&value)?);
            }
            _ => {
                return Err(SolverError::InvalidDepSpec {
                    spec: "package_filter".to_string(),
                    reason: "package_filter must be dict or list of dicts".to_string(),
                });
            }
        }

        if filters.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Self { filters }))
        }
    }

    pub fn excludes(&self, pkg: &Package) -> bool {
        for filter in &self.filters {
            if filter.excludes(pkg) {
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

#[derive(Debug, Clone)]
struct PackageFilter {
    excludes: HashMap<Option<String>, Vec<Rule>>,
    includes: HashMap<Option<String>, Vec<Rule>>,
}

impl PackageFilter {
    fn from_pod(value: &JsonValue) -> Result<Self, SolverError> {
        let obj = value.as_object().ok_or_else(|| SolverError::InvalidDepSpec {
            spec: "package_filter".to_string(),
            reason: "filter entry must be a dict".to_string(),
        })?;

        let mut filter = PackageFilter {
            excludes: HashMap::new(),
            includes: HashMap::new(),
        };

        if let Some(raw) = obj.get("excludes") {
            let rules = parse_rule_list(raw)?;
            for rule in rules {
                filter.add_exclusion(rule);
            }
        }

        if let Some(raw) = obj.get("includes") {
            let rules = parse_rule_list(raw)?;
            for rule in rules {
                filter.add_inclusion(rule);
            }
        }

        Ok(filter)
    }

    fn add_exclusion(&mut self, rule: Rule) {
        let key = rule.family().map(|s| s.to_string());
        self.excludes.entry(key).or_default().push(rule);
    }

    fn add_inclusion(&mut self, rule: Rule) {
        let key = rule.family().map(|s| s.to_string());
        self.includes.entry(key).or_default().push(rule);
    }

    fn excludes(&self, pkg: &Package) -> bool {
        if self.excludes.is_empty() {
            return false;
        }

        let mut matched = false;

        if let Some(rules) = self.excludes.get(&Some(pkg.base.clone())) {
            matched = rules.iter().any(|r| r.matches(pkg));
        }

        if !matched {
            if let Some(rules) = self.excludes.get(&None) {
                matched = rules.iter().any(|r| r.matches(pkg));
            }
        }

        if !matched {
            return false;
        }

        if let Some(rules) = self.includes.get(&Some(pkg.base.clone())) {
            if rules.iter().any(|r| r.matches(pkg)) {
                return false;
            }
        }

        if let Some(rules) = self.includes.get(&None) {
            if rules.iter().any(|r| r.matches(pkg)) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
enum Rule {
    Glob { _pattern: String, regex: Regex, family: Option<String> },
    Regex { _pattern: String, regex: Regex, family: Option<String> },
    Range { spec: DepSpec },
    Timestamp {
        timestamp: i64,
        reverse: bool,
        match_untimestamped: bool,
        family: Option<String>,
    },
}

impl Rule {
    fn family(&self) -> Option<&str> {
        match self {
            Rule::Glob { family, .. } => family.as_deref(),
            Rule::Regex { family, .. } => family.as_deref(),
            Rule::Range { spec } => Some(spec.base.as_str()),
            Rule::Timestamp { family, .. } => family.as_deref(),
        }
    }

    fn matches(&self, pkg: &Package) -> bool {
        match self {
            Rule::Glob { regex, family, .. } => {
                if let Some(fam) = family {
                    if pkg.base != *fam {
                        return false;
                    }
                }
                regex.is_match(&pkg.name)
            }
            Rule::Regex { regex, family, .. } => {
                if let Some(fam) = family {
                    if pkg.base != *fam {
                        return false;
                    }
                }
                regex.is_match(&pkg.name)
            }
            Rule::Range { spec } => {
                if pkg.base != spec.base {
                    return false;
                }
                spec.matches_impl(&pkg.version).unwrap_or(false)
            }
            Rule::Timestamp {
                timestamp,
                reverse,
                match_untimestamped,
                family,
            } => {
                if let Some(fam) = family {
                    if pkg.base != *fam {
                        return false;
                    }
                }
                match pkg.timestamp {
                    None => *match_untimestamped,
                    Some(ts) => {
                        if *reverse {
                            ts > *timestamp
                        } else {
                            ts <= *timestamp
                        }
                    }
                }
            }
        }
    }
}

fn parse_rule_list(value: &JsonValue) -> Result<Vec<Rule>, SolverError> {
    let mut out = Vec::new();
    match value {
        JsonValue::String(text) => {
            out.push(parse_rule(text)?);
        }
        JsonValue::Array(items) => {
            for item in items {
                if let Some(text) = item.as_str() {
                    out.push(parse_rule(text)?);
                } else {
                    return Err(SolverError::InvalidDepSpec {
                        spec: "package_filter".to_string(),
                        reason: "rule must be a string".to_string(),
                    });
                }
            }
        }
        _ => {
            return Err(SolverError::InvalidDepSpec {
                spec: "package_filter".to_string(),
                reason: "rule must be string or list".to_string(),
            });
        }
    }
    Ok(out)
}

fn parse_rule(raw: &str) -> Result<Rule, SolverError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(SolverError::InvalidDepSpec {
            spec: "package_filter".to_string(),
            reason: "empty rule".to_string(),
        });
    }

    let mut label = None;
    let mut body = raw;
    if let Some(start) = raw.find('(') {
        if raw.ends_with(')') && start + 1 < raw.len() {
            label = Some(raw[..start].trim().to_string());
            body = &raw[start + 1..raw.len() - 1];
        }
    }

    let label = label.unwrap_or_else(|| {
        if raw.contains('*') || raw.contains('?') {
            "glob".to_string()
        } else {
            "range".to_string()
        }
    });

    match label.as_str() {
        "glob" => {
            let pattern = body.trim().to_string();
            let family = extract_family(&pattern);
            let regex = glob_to_regex(&pattern)?;
            Ok(Rule::Glob {
                _pattern: pattern,
                regex,
                family,
            })
        }
        "regex" => {
            let pattern = body.trim().to_string();
            let family = extract_family(&pattern);
            let regex = Regex::new(&pattern).map_err(|e| SolverError::InvalidDepSpec {
                spec: raw.to_string(),
                reason: e.to_string(),
            })?;
            Ok(Rule::Regex {
                _pattern: pattern,
                regex,
                family,
            })
        }
        "range" => {
            let spec = parse_requirement(body)?;
            Ok(Rule::Range { spec })
        }
        "before" | "after" => {
            let mut family = None;
            let mut ts_text = body.trim();
            if let Some(idx) = ts_text.find(':') {
                let (fam, rest) = ts_text.split_at(idx);
                if !fam.trim().is_empty() {
                    family = Some(fam.trim().to_string());
                }
                ts_text = rest.trim_start_matches(':').trim();
            }
            let timestamp = ts_text.parse::<i64>().map_err(|e| SolverError::InvalidDepSpec {
                spec: raw.to_string(),
                reason: e.to_string(),
            })?;
            let reverse = label == "after";
            Ok(Rule::Timestamp {
                timestamp,
                reverse,
                match_untimestamped: false,
                family,
            })
        }
        _ => Err(SolverError::InvalidDepSpec {
            spec: raw.to_string(),
            reason: "unknown rule type".to_string(),
        }),
    }
}

fn parse_requirement(raw: &str) -> Result<DepSpec, SolverError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(SolverError::InvalidDepSpec {
            spec: "package_filter".to_string(),
            reason: "empty range rule".to_string(),
        });
    }

    if raw.contains('@') {
        return DepSpec::parse_impl(raw).map_err(|e| SolverError::InvalidDepSpec {
            spec: raw.to_string(),
            reason: e.to_string(),
        });
    }

    if let Some(pos) = raw.find(|c| c == '<' || c == '>' || c == '=') {
        let (base, rest) = raw.split_at(pos);
        let base = base.trim();
        if base.is_empty() {
            return Err(SolverError::InvalidDepSpec {
                spec: raw.to_string(),
                reason: "missing package name".to_string(),
            });
        }
        let constraint = rest.trim();
        let spec = DepSpec::new(base.to_string(), Some(constraint.to_string()));
        return Ok(spec);
    }

    if raw.ends_with('+') {
        let trimmed = raw.trim_end_matches('+').trim();
        if let Some((base, version)) = split_name_version(trimmed) {
            let spec = DepSpec::new(base, Some(format!(">={}", version)));
            return Ok(spec);
        }
    }

    if let Some((base, version)) = split_name_version(raw) {
        let spec = DepSpec::new(base, Some(version));
        return Ok(spec);
    }

    Ok(DepSpec::new(raw.to_string(), None))
}

fn split_name_version(raw: &str) -> Option<(String, String)> {
    let (base, version) = crate::Package::parse_name(raw).ok()?;
    Some((base, version))
}

fn extract_family(text: &str) -> Option<String> {
    static FAMILY_RE: OnceLock<Regex> = OnceLock::new();
    let re = FAMILY_RE.get_or_init(|| Regex::new(r"^([A-Za-z0-9_]+)[-@#]").unwrap());
    re.captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

fn glob_to_regex(pattern: &str) -> Result<Regex, SolverError> {
    let mut out = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out.push('$');
    Regex::new(&out).map_err(|e| SolverError::InvalidDepSpec {
        spec: pattern.to_string(),
        reason: e.to_string(),
    })
}
