use super::{VersionError, VersionRange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub range: Option<VersionRange>,
    pub conflict: bool,
    pub weak: bool,
    pub original: String,
}

impl Requirement {
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let mut s = input.trim();
        if s.is_empty() {
            return Err(VersionError::new("Empty requirement"));
        }

        let mut conflict = false;
        let mut weak = false;

        if let Some(rest) = s.strip_prefix('!') {
            conflict = true;
            s = rest;
        } else if let Some(rest) = s.strip_prefix('~') {
            weak = true;
            conflict = true;
            s = rest;
        }

        let (name, range, original) = if let Some(idx) = s
            .find(|c: char| matches!(c, '-' | '@' | '#' | '=' | '<' | '>'))
        {
            let name = s[..idx].to_string();
            let mut req_str = &s[idx..];
            if let Some(first) = req_str.chars().next() {
                if matches!(first, '-' | '@' | '#') {
                    req_str = &req_str[1..];
                }
            }

            let mut range = if req_str.is_empty() {
                VersionRange::parse("")?
            } else {
                VersionRange::parse(req_str)?
            };

            if weak {
                range = match range.inverse() {
                    Some(r) => r,
                    None => VersionRange::parse("")?,
                };
            }

            (name, Some(range), input.to_string())
        } else if weak {
            (s.to_string(), None, input.to_string())
        } else {
            (s.to_string(), Some(VersionRange::parse("")?), input.to_string())
        };

        Ok(Self {
            name,
            range,
            conflict,
            weak,
            original,
        })
    }
}
