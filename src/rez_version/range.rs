use std::cmp::Ordering;
use std::fmt;

use pubgrub::Ranges;

use super::version::{Version, VersionError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LowerBound {
    pub version: Version,
    pub inclusive: bool,
}

impl LowerBound {
    pub fn min() -> Self {
        Self {
            version: Version::empty(),
            inclusive: true,
        }
    }

    pub fn contains_version(&self, version: &Version) -> bool {
        version > &self.version || (self.inclusive && version == &self.version)
    }
}

impl Ord for LowerBound {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_version = self.version.cmp(&other.version);
        if by_version != Ordering::Equal {
            return by_version;
        }
        match (self.inclusive, other.inclusive) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl PartialOrd for LowerBound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for LowerBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.version.is_empty() {
            if self.inclusive {
                return write!(f, "");
            }
            return write!(f, ">");
        }
        if self.inclusive {
            write!(f, "{}+", self.version)
        } else {
            write!(f, ">{}", self.version)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpperBound {
    pub version: Version,
    pub inclusive: bool,
}

impl UpperBound {
    pub fn inf() -> Self {
        Self {
            version: Version::inf(),
            inclusive: true,
        }
    }

    pub fn contains_version(&self, version: &Version) -> bool {
        version < &self.version || (self.inclusive && version == &self.version)
    }
}

impl Ord for UpperBound {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_version = self.version.cmp(&other.version);
        if by_version != Ordering::Equal {
            return by_version;
        }
        match (self.inclusive, other.inclusive) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl PartialOrd for UpperBound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for UpperBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inclusive {
            write!(f, "<={}", self.version)
        } else {
            write!(f, "<{}", self.version)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bound {
    pub lower: LowerBound,
    pub upper: UpperBound,
}

impl Bound {
    pub fn new(
        lower: Option<LowerBound>,
        upper: Option<UpperBound>,
        invalid_bound_error: bool,
    ) -> Result<Self, VersionError> {
        let lower = lower.unwrap_or_else(LowerBound::min);
        let upper = upper.unwrap_or_else(UpperBound::inf);

        if invalid_bound_error {
            if lower.version > upper.version
                || (lower.version == upper.version
                    && !(lower.inclusive && upper.inclusive))
            {
                return Err(VersionError::new("Invalid bound"));
            }
        }

        Ok(Self { lower, upper })
    }

    pub fn any() -> Self {
        Self {
            lower: LowerBound::min(),
            upper: UpperBound::inf(),
        }
    }

    pub fn contains_version(&self, version: &Version) -> bool {
        self.lower.contains_version(version) && self.upper.contains_version(version)
    }

    pub fn lower_bounded(&self) -> bool {
        self.lower != LowerBound::min()
    }

    pub fn upper_bounded(&self) -> bool {
        self.upper != UpperBound::inf()
    }

    pub fn intersects(&self, other: &Self) -> bool {
        let lower = std::cmp::max(self.lower.clone(), other.lower.clone());
        let upper = std::cmp::min(self.upper.clone(), other.upper.clone());
        lower.version < upper.version
            || (lower.version == upper.version && lower.inclusive && upper.inclusive)
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let lower = std::cmp::max(self.lower.clone(), other.lower.clone());
        let upper = std::cmp::min(self.upper.clone(), other.upper.clone());
        if lower.version < upper.version
            || (lower.version == upper.version && lower.inclusive && upper.inclusive)
        {
            Some(Self { lower, upper })
        } else {
            None
        }
    }

    pub fn to_pubgrub(&self) -> Ranges<Version> {
        let lower_is_min = self.lower == LowerBound::min();
        let upper_is_inf = self.upper == UpperBound::inf();

        if lower_is_min && upper_is_inf {
            return Ranges::full();
        }

        let lower_range = if lower_is_min {
            Ranges::full()
        } else if self.lower.inclusive {
            Ranges::higher_than(self.lower.version.clone())
        } else {
            Ranges::strictly_higher_than(self.lower.version.clone())
        };

        let upper_range = if upper_is_inf {
            Ranges::full()
        } else if self.upper.inclusive {
            Ranges::lower_than(self.upper.version.clone())
        } else {
            Ranges::strictly_lower_than(self.upper.version.clone())
        };

        lower_range.intersection(&upper_range)
    }
}

impl Ord for Bound {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_lower = self.lower.cmp(&other.lower);
        if by_lower != Ordering::Equal {
            return by_lower;
        }
        self.upper.cmp(&other.upper)
    }
}

impl PartialOrd for Bound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.upper.version.is_inf() {
            return write!(f, "{}", self.lower);
        }
        if self.lower.version == self.upper.version {
            return write!(f, "=={}", self.lower.version);
        }
        if self.lower.inclusive && self.upper.inclusive {
            if !self.lower.version.is_empty() {
                return write!(f, "{}..{}", self.lower.version, self.upper.version);
            }
            return write!(f, "{}", self.upper);
        }
        if self.lower.inclusive
            && !self.upper.inclusive
            && self.lower.version.next() == self.upper.version
        {
            return write!(f, "{}", self.lower.version);
        }
        write!(f, "{}{}", self.lower, self.upper)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionRange {
    pub bounds: Vec<Bound>,
}

impl VersionRange {
    pub fn parse(range_str: &str) -> Result<Self, VersionError> {
        Self::parse_with_opts(range_str, true)
    }

    pub fn parse_with_opts(range_str: &str, invalid_bound_error: bool) -> Result<Self, VersionError> {
        if range_str.is_empty() {
            return Ok(Self { bounds: vec![Bound::any()] });
        }

        let mut bounds = Vec::new();
        let mut any = false;

        for part in range_str.split('|') {
            if part.is_empty() {
                any = true;
                continue;
            }
            let bound = parse_clause(part, invalid_bound_error)?;
            bounds.push(bound);
        }

        if any {
            return Ok(Self { bounds: vec![Bound::any()] });
        }

        if bounds.is_empty() {
            return Ok(Self { bounds: vec![Bound::any()] });
        }

        let bounds = union_bounds(bounds);
        Ok(Self { bounds })
    }

    pub fn is_any(&self) -> bool {
        self.bounds.len() == 1 && self.bounds[0] == Bound::any()
    }

    pub fn lower_bounded(&self) -> bool {
        self.bounds.first().map(|b| b.lower_bounded()).unwrap_or(false)
    }

    pub fn upper_bounded(&self) -> bool {
        self.bounds.last().map(|b| b.upper_bounded()).unwrap_or(false)
    }

    pub fn bounded(&self) -> bool {
        self.lower_bounded() && self.upper_bounded()
    }

    pub fn contains_version(&self, version: &Version) -> bool {
        self.bounds.iter().any(|b| b.contains_version(version))
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut bounds = self.bounds.clone();
        bounds.extend(other.bounds.iter().cloned());
        let bounds = union_bounds(bounds);
        Self { bounds }
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let mut bounds = Vec::new();
        for b1 in &self.bounds {
            for b2 in &other.bounds {
                if let Some(b) = b1.intersection(b2) {
                    bounds.push(b);
                }
            }
        }
        if bounds.is_empty() {
            None
        } else {
            Some(Self { bounds })
        }
    }

    pub fn inverse(&self) -> Option<Self> {
        if self.is_any() {
            return None;
        }

        let mut lbounds: Vec<Option<LowerBound>> = vec![None];
        let mut ubounds: Vec<Option<UpperBound>> = Vec::new();

        for bound in &self.bounds {
            if bound.lower.version.is_empty() && bound.lower.inclusive {
                ubounds.push(None);
            } else {
                ubounds.push(Some(UpperBound {
                    version: bound.lower.version.clone(),
                    inclusive: !bound.lower.inclusive,
                }));
            }

            if bound.upper.version.is_inf() {
                lbounds.push(None);
            } else {
                lbounds.push(Some(LowerBound {
                    version: bound.upper.version.clone(),
                    inclusive: !bound.upper.inclusive,
                }));
            }
        }

        ubounds.push(None);

        let mut bounds = Vec::new();
        for (lower, upper) in lbounds.into_iter().zip(ubounds.into_iter()) {
            if lower.is_none() && upper.is_none() {
                continue;
            }
            let bound = Bound::new(lower, upper, false).ok()?;
            bounds.push(bound);
        }

        Some(Self { bounds })
    }

    pub fn to_pubgrub(&self) -> Ranges<Version> {
        if self.is_any() {
            return Ranges::full();
        }
        let mut out = Ranges::empty();
        for bound in &self.bounds {
            let range = bound.to_pubgrub();
            out = out.union(&range);
        }
        out
    }

    pub fn from_version(version: &Version, op: Option<&str>) -> Result<Self, VersionError> {
        let mut lower: Option<LowerBound> = None;
        let mut upper: Option<UpperBound> = None;

        match op {
            None => {
                lower = Some(LowerBound {
                    version: version.clone(),
                    inclusive: true,
                });
                if !version.is_empty() {
                    upper = Some(UpperBound {
                        version: version.next(),
                        inclusive: false,
                    });
                }
            }
            Some("eq") | Some("==") => {
                lower = Some(LowerBound {
                    version: version.clone(),
                    inclusive: true,
                });
                upper = Some(UpperBound {
                    version: version.clone(),
                    inclusive: true,
                });
            }
            Some("gt") | Some(">") => {
                lower = Some(LowerBound {
                    version: version.clone(),
                    inclusive: false,
                });
            }
            Some("gte") | Some(">=") => {
                lower = Some(LowerBound {
                    version: version.clone(),
                    inclusive: true,
                });
            }
            Some("lt") | Some("<") => {
                upper = Some(UpperBound {
                    version: version.clone(),
                    inclusive: false,
                });
            }
            Some("lte") | Some("<=") => {
                upper = Some(UpperBound {
                    version: version.clone(),
                    inclusive: true,
                });
            }
            Some(op) => {
                return Err(VersionError::new(format!(
                    "Unknown bound operation '{}'",
                    op
                )))
            }
        }

        let bound = Bound::new(lower, upper, true)?;
        Ok(Self { bounds: vec![bound] })
    }
}

impl fmt::Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.bounds.iter().map(|b| b.to_string()).collect();
        write!(f, "{}", parts.join("|"))
    }
}

fn parse_clause(input: &str, invalid_bound_error: bool) -> Result<Bound, VersionError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(VersionError::new("Empty version range"));
    }

    if input.contains("..") {
        return parse_inclusive_bound(input, invalid_bound_error);
    }

    let mut parts: Vec<&str> = Vec::new();
    if input.contains(',') {
        for part in input.split(',') {
            if !part.trim().is_empty() {
                parts.push(part.trim());
            }
        }
    } else if let Some(idx) = find_upper_start(input) {
        let left = &input[..idx];
        let right = &input[idx..];
        if !left.trim().is_empty() && !right.trim().is_empty() {
            if parse_constraint(left, invalid_bound_error).is_ok()
                && parse_constraint(right, invalid_bound_error).is_ok()
            {
                parts.push(left.trim());
                parts.push(right.trim());
            } else {
                parts.push(input);
            }
        } else {
            parts.push(input);
        }
    } else {
        parts.push(input);
    }

    let mut bound: Option<Bound> = None;
    for part in parts {
        let b = parse_constraint(part, invalid_bound_error)?;
        bound = match bound {
            None => Some(b),
            Some(existing) => match existing.intersection(&b) {
                Some(next) => Some(next),
                None => {
                    return Err(VersionError::new("Invalid bound"));
                }
            },
        };
    }

    bound.ok_or_else(|| VersionError::new("Invalid version range"))
}

fn parse_inclusive_bound(input: &str, invalid_bound_error: bool) -> Result<Bound, VersionError> {
    let parts: Vec<&str> = input.split("..").collect();
    if parts.len() != 2 {
        return Err(VersionError::new(format!("Invalid version range '{}'", input)));
    }
    let lower_version = parse_version_opt(parts[0].trim())?;
    let upper_version = parse_version_opt(parts[1].trim())?;

    let lower = Some(LowerBound {
        version: lower_version,
        inclusive: true,
    });
    let upper = Some(UpperBound {
        version: upper_version,
        inclusive: true,
    });

    Bound::new(lower, upper, invalid_bound_error)
}

fn parse_constraint(input: &str, invalid_bound_error: bool) -> Result<Bound, VersionError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(VersionError::new("Empty version constraint"));
    }

    if input == "*" {
        return Ok(Bound::any());
    }

    if let Some(rest) = input.strip_prefix("==") {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            Some(LowerBound {
                version: version.clone(),
                inclusive: true,
            }),
            Some(UpperBound {
                version,
                inclusive: true,
            }),
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_prefix('=') {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            Some(LowerBound {
                version: version.clone(),
                inclusive: true,
            }),
            Some(UpperBound {
                version,
                inclusive: true,
            }),
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_prefix(">=") {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            Some(LowerBound {
                version,
                inclusive: true,
            }),
            None,
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_prefix('>') {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            Some(LowerBound {
                version,
                inclusive: false,
            }),
            None,
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_prefix("<=") {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            None,
            Some(UpperBound {
                version,
                inclusive: true,
            }),
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_prefix('<') {
        let version = parse_version_opt(rest.trim())?;
        if version.is_empty() {
            return Err(VersionError::new("Invalid upper bound"));
        }
        return Bound::new(
            None,
            Some(UpperBound {
                version,
                inclusive: false,
            }),
            invalid_bound_error,
        );
    }

    if let Some(rest) = input.strip_suffix('+') {
        let version = parse_version_opt(rest.trim())?;
        return Bound::new(
            Some(LowerBound {
                version,
                inclusive: true,
            }),
            None,
            invalid_bound_error,
        );
    }

    let version = parse_version_opt(input)?;
    VersionRange::from_version(&version, None).map(|r| r.bounds[0].clone())
}

fn parse_version_opt(input: &str) -> Result<Version, VersionError> {
    if input.is_empty() {
        return Ok(Version::empty());
    }
    Version::parse(input)
}

fn find_upper_start(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b'<' {
            return Some(i);
        }
    }
    None
}

fn union_bounds(bounds: Vec<Bound>) -> Vec<Bound> {
    if bounds.len() < 2 {
        return bounds;
    }

    let mut sorted = bounds;
    sorted.sort();

    let mut new_bounds = Vec::new();
    let mut start = 0usize;
    let mut prev_bound: Option<Bound> = None;
    let mut upper: Option<UpperBound> = None;

    for (i, bound) in sorted.iter().enumerate() {
        if i == 0 {
            prev_bound = Some(bound.clone());
            upper = Some(bound.upper.clone());
            continue;
        }

        let prev = prev_bound.as_ref().unwrap();
        let upper_bound = upper.as_ref().unwrap();
        let gap = bound.lower.version > upper_bound.version
            || (bound.lower.version == upper_bound.version
                && !bound.lower.inclusive
                && !prev.upper.inclusive);

        if gap {
            let merged = Bound::new(
                Some(sorted[start].lower.clone()),
                upper.clone(),
                false,
            )
            .unwrap_or_else(|_| Bound::any());
            new_bounds.push(merged);
            start = i;
        }

        if let Some(current_upper) = &upper {
            if bound.upper > *current_upper {
                upper = Some(bound.upper.clone());
            }
        } else {
            upper = Some(bound.upper.clone());
        }

        prev_bound = Some(bound.clone());
    }

    if let Some(upper) = upper {
        let merged = Bound::new(
            Some(sorted[start].lower.clone()),
            Some(upper),
            false,
        )
        .unwrap_or_else(|_| Bound::any());
        new_bounds.push(merged);
    }

    new_bounds
}
