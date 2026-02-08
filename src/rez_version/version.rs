use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionError {
    pub message: String,
}

impl VersionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubToken {
    s: String,
    n: Option<u64>,
}

impl SubToken {
    fn new(s: String) -> Self {
        let n = if s.chars().all(|c| c.is_ascii_digit()) {
            s.parse::<u64>().ok()
        } else {
            None
        };
        Self { s, n }
    }
}

impl Ord for SubToken {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.n, other.n) {
            (None, None) => self.s.cmp(&other.s),
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a), Some(b)) => {
                let by_num = a.cmp(&b);
                if by_num == Ordering::Equal {
                    self.s.cmp(&other.s)
                } else {
                    by_num
                }
            }
        }
    }
}

impl PartialOrd for SubToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SubToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    subtokens: Vec<SubToken>,
}

impl Token {
    pub fn parse(token: &str) -> Result<Self, VersionError> {
        if token.is_empty() {
            return Err(VersionError::new("Empty version token"));
        }
        if !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(VersionError::new(format!(
                "Invalid version token: '{}'",
                token
            )));
        }
        let subtokens = parse_subtokens(token);
        Ok(Self { subtokens })
    }

    pub fn next(&self) -> Self {
        if self.subtokens.is_empty() {
            return Self {
                subtokens: vec![SubToken::new("_".to_string())],
            };
        }

        let mut subtokens = self.subtokens.clone();
        if let Some(last) = subtokens.last_mut() {
            if last.n.is_none() {
                last.s.push('_');
                last.n = None;
            } else {
                subtokens.push(SubToken::new("_".to_string()));
            }
        }
        Self { subtokens }
    }

    pub fn as_string(&self) -> String {
        self.subtokens.iter().map(|s| s.to_string()).collect()
    }
}

impl Ord for Token {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.subtokens.len().min(other.subtokens.len());
        for i in 0..len {
            let cmp = self.subtokens[i].cmp(&other.subtokens[i]);
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
        self.subtokens.len().cmp(&other.subtokens.len())
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    tokens: Option<Vec<Token>>, // None == infinity
    seps: Vec<char>,
}

impl Version {
    pub fn parse(ver_str: &str) -> Result<Self, VersionError> {
        if ver_str.is_empty() {
            return Ok(Self::empty());
        }

        let mut tokens = Vec::new();
        let mut seps = Vec::new();
        let mut current = String::new();

        for ch in ver_str.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if current.is_empty() {
                    return Err(VersionError::new(format!(
                        "Invalid version syntax: '{}'",
                        ver_str
                    )));
                }
                let token = Token::parse(&current).map_err(|e| {
                    VersionError::new(format!("Invalid version '{}': {}", ver_str, e))
                })?;
                tokens.push(token);
                current.clear();
                seps.push(ch);
            }
        }

        if current.is_empty() {
            return Err(VersionError::new(format!(
                "Invalid version syntax: '{}'",
                ver_str
            )));
        }
        let token = Token::parse(&current).map_err(|e| {
            VersionError::new(format!("Invalid version '{}': {}", ver_str, e))
        })?;
        tokens.push(token);

        if seps.len() + 1 != tokens.len() {
            return Err(VersionError::new(format!(
                "Invalid version syntax: '{}'",
                ver_str
            )));
        }

        Ok(Self {
            tokens: Some(tokens),
            seps,
        })
    }

    pub fn empty() -> Self {
        Self {
            tokens: Some(Vec::new()),
            seps: Vec::new(),
        }
    }

    pub fn inf() -> Self {
        Self {
            tokens: None,
            seps: Vec::new(),
        }
    }

    pub fn is_inf(&self) -> bool {
        self.tokens.is_none()
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.tokens.as_ref(), Some(tokens) if tokens.is_empty())
    }

    pub fn tokens(&self) -> Option<&[Token]> {
        self.tokens.as_deref()
    }

    pub fn trim(&self, len: usize) -> Self {
        let Some(tokens) = self.tokens.as_ref() else {
            return Self::inf();
        };
        if len >= tokens.len() {
            return self.clone();
        }
        let tokens = tokens[..len].to_vec();
        let seps = if len == 0 { Vec::new() } else { self.seps[..len.saturating_sub(1)].to_vec() };
        Self {
            tokens: Some(tokens),
            seps,
        }
    }

    pub fn next(&self) -> Self {
        let Some(tokens) = self.tokens.as_ref() else {
            return Self::inf();
        };
        if tokens.is_empty() {
            return Self::inf();
        }
        let mut tokens = tokens.clone();
        if let Some(last) = tokens.pop() {
            tokens.push(last.next());
        }
        Self {
            tokens: Some(tokens),
            seps: self.seps.clone(),
        }
    }

    pub fn as_tuple(&self) -> Vec<String> {
        match self.tokens.as_ref() {
            None => Vec::new(),
            Some(tokens) => tokens.iter().map(|t| t.as_string()).collect(),
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.tokens, &other.tokens) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => {
                let len = a.len().min(b.len());
                for i in 0..len {
                    let cmp = a[i].cmp(&b[i]);
                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
                a.len().cmp(&b.len())
            }
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tokens.is_none() {
            return write!(f, "[INF]");
        }
        let tokens = self.tokens.as_ref().unwrap();
        let mut out = String::new();
        for (idx, token) in tokens.iter().enumerate() {
            out.push_str(&token.as_string());
            if idx < self.seps.len() {
                out.push(self.seps[idx]);
            }
        }
        write!(f, "{}", out)
    }
}

fn parse_subtokens(token: &str) -> Vec<SubToken> {
    let mut subtokens = Vec::new();
    let mut current = String::new();
    let mut is_numeric: Option<bool> = None;

    for ch in token.chars() {
        let digit = ch.is_ascii_digit();
        match is_numeric {
            None => {
                current.push(ch);
                is_numeric = Some(digit);
            }
            Some(state) if state == digit => {
                current.push(ch);
            }
            Some(_) => {
                subtokens.push(SubToken::new(current));
                current = String::new();
                current.push(ch);
                is_numeric = Some(digit);
            }
        }
    }

    if !current.is_empty() {
        subtokens.push(SubToken::new(current));
    }

    subtokens
}
