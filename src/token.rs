//! Token expansion for environment variables.
//!
//! Provides unified `{TOKEN}` expansion logic used by both [`Evar`](crate::evar::Evar) and [`Env`](crate::env::Env).
//! Supports recursive expansion with cycle detection and depth limiting.
//!
//! # Example
//!
//! ```ignore
//! use pkg::token::{expand, TokenLookup};
//! use std::collections::HashMap;
//!
//! let vars: HashMap<String, String> = [
//!     ("ROOT".into(), "/opt/maya".into()),
//!     ("BIN".into(), "{ROOT}/bin".into()),
//! ].into();
//!
//! let result = expand("{BIN}/maya", &vars, 10)?;
//! assert_eq!(result, "/opt/maya/bin/maya");
//! ```

use log::trace;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors during token expansion.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Circular reference detected (A -> B -> A).
    #[error("Circular reference detected for token '{name}'")]
    CircularReference { name: String },

    /// Maximum recursion depth exceeded.
    #[error("Max depth {max_depth} exceeded expanding '{name}'")]
    DepthExceeded { name: String, max_depth: usize },
}

/// Trait for token value lookup.
///
/// Implement this for custom lookup sources (HashMap, Env, closure, etc).
pub trait TokenLookup {
    /// Look up token value by name. Returns None if not found.
    fn lookup(&self, name: &str) -> Option<&str>;
}

// Impl for HashMap<String, String> - case-insensitive lookup
impl TokenLookup for HashMap<String, String> {
    fn lookup(&self, name: &str) -> Option<&str> {
        // Try exact match first, then lowercase
        self.get(name)
            .or_else(|| self.get(&name.to_lowercase()))
            .map(|s| s.as_str())
    }
}

/// Extract all `{TOKEN}` names from a string.
///
/// Returns set of token names (without braces).
/// Only valid identifiers are extracted (alphanumeric + underscore).
pub fn extract(value: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let token: String = chars[start..end].iter().collect();
                if is_valid_identifier(&token) {
                    tokens.insert(token);
                }
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }

    tokens
}

/// Check if string contains any `{TOKEN}` patterns.
#[inline]
pub fn has_tokens(value: &str) -> bool {
    value.contains('{') && value.contains('}')
}

/// Expand `{TOKEN}` patterns in value using a lookup closure.
///
/// Single-pass expansion without recursion. Use [`expand_recursive`] for
/// full recursive expansion with cycle detection.
///
/// Tokens not found in lookup are left as-is.
///
/// # Example
/// ```ignore
/// let result = expand_tokens("{ROOT}/bin", |name| {
///     match name {
///         "ROOT" => Some("/opt/maya".into()),
///         _ => None
///     }
/// });
/// assert_eq!(result, "/opt/maya/bin");
/// ```
pub fn expand_tokens<F>(value: &str, lookup: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut result = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let token: String = chars[start..end].iter().collect();
                if is_valid_identifier(&token) {
                    if let Some(replacement) = lookup(&token) {
                        result.push_str(&replacement);
                        i = end + 1;
                        continue;
                    }
                }
            }
            // Token not found or invalid - keep original
            result.push('{');
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Expand `{TOKEN}` patterns recursively with cycle detection.
///
/// # Arguments
/// * `value` - String with `{TOKEN}` patterns
/// * `lookup` - Token value provider
/// * `max_depth` - Maximum recursion depth (10 is typical)
///
/// # Errors
/// - [`TokenError::CircularReference`] if A references B which references A
/// - [`TokenError::DepthExceeded`] if recursion goes too deep
pub fn expand_recursive(
    value: &str,
    lookup: &HashMap<String, String>,
    max_depth: usize,
) -> Result<String, TokenError> {
    let mut visiting: HashSet<String> = HashSet::new();
    expand_impl(value, lookup, &mut visiting, 0, max_depth)
}

/// Expand with OS environment fallback.
///
/// If token not found in lookup, tries `std::env::var()`.
pub fn expand_with_fallback(
    value: &str,
    lookup: &HashMap<String, String>,
    max_depth: usize,
) -> Result<String, TokenError> {
    let mut visiting: HashSet<String> = HashSet::new();
    expand_impl_with_fallback(value, lookup, &mut visiting, 0, max_depth, true)
}

/// Internal recursive expansion.
fn expand_impl(
    value: &str,
    lookup: &HashMap<String, String>,
    visiting: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
) -> Result<String, TokenError> {
    expand_impl_with_fallback(value, lookup, visiting, depth, max_depth, false)
}

/// Internal recursive expansion with optional OS fallback.
fn expand_impl_with_fallback(
    value: &str,
    lookup: &HashMap<String, String>,
    visiting: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
    use_os_fallback: bool,
) -> Result<String, TokenError> {
    trace!("token::expand depth={} value={}", depth, value);
    
    if depth > max_depth {
        return Err(TokenError::DepthExceeded {
            name: String::new(),
            max_depth,
        });
    }

    // Quick exit if no tokens
    if !has_tokens(value) {
        return Ok(value.to_string());
    }

    let mut result = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }

            if end < chars.len() {
                let token: String = chars[start..end].iter().collect();

                if is_valid_identifier(&token) {
                    let token_lower = token.to_lowercase();

                    // Cycle detection
                    if visiting.contains(&token_lower) {
                        return Err(TokenError::CircularReference { name: token });
                    }

                    // Try lookup
                    let replacement = if let Some(val) = lookup.get(&token_lower) {
                        // Recursively expand the value
                        visiting.insert(token_lower.clone());
                        let expanded = expand_impl_with_fallback(
                            val,
                            lookup,
                            visiting,
                            depth + 1,
                            max_depth,
                            use_os_fallback,
                        )?;
                        visiting.remove(&token_lower);
                        Some(expanded)
                    } else if use_os_fallback {
                        // Try OS environment
                        std::env::var(&token).ok()
                    } else {
                        None
                    };

                    if let Some(ref rep) = replacement {
                        trace!("token::expand {{{}}} -> {}", token, rep);
                        result.push_str(rep);
                        i = end + 1;
                        continue;
                    }
                }
            }
            // Token not found - keep original
            result.push('{');
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Ok(result)
}

/// Check if string is valid identifier (alphanumeric + underscore).
#[inline]
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tokens_basic() {
        let tokens = extract("{ROOT}/bin/{LIB}");
        assert!(tokens.contains("ROOT"));
        assert!(tokens.contains("LIB"));
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn extract_empty_braces() {
        let tokens = extract("{}/bin");
        assert!(tokens.is_empty());
    }

    #[test]
    fn expand_tokens_basic() {
        let lookup: HashMap<String, String> =
            [("ROOT".into(), "/opt/maya".into())].into_iter().collect();

        let result = expand_tokens("{ROOT}/bin", |n| lookup.get(n).cloned());
        assert_eq!(result, "/opt/maya/bin");
    }

    #[test]
    fn expand_tokens_missing() {
        let result = expand_tokens("{UNKNOWN}/bin", |_| None);
        assert_eq!(result, "{UNKNOWN}/bin");
    }

    #[test]
    fn expand_recursive_chain() {
        let lookup: HashMap<String, String> = [
            ("a".into(), "base".into()),
            ("b".into(), "{A}/level1".into()),
            ("c".into(), "{B}/level2".into()),
        ]
        .into_iter()
        .collect();

        let result = expand_recursive("{C}", &lookup, 10).unwrap();
        assert_eq!(result, "base/level1/level2");
    }

    #[test]
    fn expand_recursive_cycle() {
        let lookup: HashMap<String, String> = [
            ("a".into(), "{B}".into()),
            ("b".into(), "{A}".into()),
        ]
        .into_iter()
        .collect();

        let result = expand_recursive("{A}", &lookup, 10);
        assert!(matches!(result, Err(TokenError::CircularReference { .. })));
    }

    #[test]
    fn expand_recursive_depth() {
        // Deep chain
        let mut lookup: HashMap<String, String> = HashMap::new();
        lookup.insert("v0".into(), "base".into());
        for i in 1..=15 {
            lookup.insert(format!("v{}", i), format!("{{V{}}}", i - 1));
        }

        let result = expand_recursive("{V15}", &lookup, 5);
        assert!(matches!(result, Err(TokenError::DepthExceeded { .. })));
    }

    #[test]
    fn has_tokens_check() {
        assert!(has_tokens("{ROOT}/bin"));
        assert!(has_tokens("prefix{X}suffix"));
        assert!(!has_tokens("no tokens here"));
        assert!(!has_tokens("just { brace"));
        assert!(!has_tokens("just } brace"));
    }
}
