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

        let (name, range, original) = if let Some((idx, sep)) = find_req_sep(s) {
            let name = s[..idx].to_string();
            let mut req_str = &s[idx..];
            if matches!(sep, '-' | '@' | '#') {
                req_str = &req_str[1..];
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

fn find_req_sep(s: &str) -> Option<(usize, char)> {
    let mut iter = s.char_indices().peekable();
    while let Some((i, ch)) = iter.next() {
        match ch {
            '@' | '#' | '<' | '>' | '=' => return Some((i, ch)),
            '-' => {
                if let Some((_, next)) = iter.peek() {
                    if next.is_ascii_digit() {
                        return Some((i, ch));
                    }
                }
            }
            _ => {}
        }
    }
    None
}
