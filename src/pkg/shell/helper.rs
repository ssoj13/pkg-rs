//! Shell helper for completion and hints.

use super::SHELL_COMMANDS;
use pkg_lib::Storage;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use std::borrow::Cow;

/// Shell helper for completion and hints.
pub struct ShellHelper {
    packages: Vec<String>,
    bases: Vec<String>,
}

impl ShellHelper {
    pub fn new(storage: &Storage) -> Self {
        let packages = storage.packages().iter().map(|p| p.name.clone()).collect();
        let bases = storage.bases();
        Self { packages, bases }
    }

    pub fn update(&mut self, storage: &Storage) {
        self.packages = storage.packages().iter().map(|p| p.name.clone()).collect();
        self.bases = storage.bases();
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line = &line[..pos];
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Complete command
        if parts.is_empty() || (parts.len() == 1 && !line.ends_with(' ')) {
            let prefix = parts.first().copied().unwrap_or("");
            let matches: Vec<Pair> = SHELL_COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(prefix))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            return Ok((0, matches));
        }

        // Complete package name for commands that need it
        let cmd = parts[0];
        if matches!(cmd, "info" | "run" | "env" | "solve") {
            let prefix = if line.ends_with(' ') {
                ""
            } else {
                parts.get(1).copied().unwrap_or("")
            };
            let start = if line.ends_with(' ') {
                pos
            } else {
                line.rfind(' ').map(|i| i + 1).unwrap_or(0)
            };

            let mut matches: Vec<Pair> = self
                .packages
                .iter()
                .filter(|p| p.starts_with(prefix))
                .map(|p| Pair {
                    display: p.clone(),
                    replacement: p.clone(),
                })
                .collect();

            // Also add base names
            for base in &self.bases {
                if base.starts_with(prefix) && !matches.iter().any(|m| m.display == *base) {
                    matches.push(Pair {
                        display: base.clone(),
                        replacement: base.clone(),
                    });
                }
            }

            return Ok((start, matches));
        }

        Ok((pos, vec![]))
    }
}

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        if parts.len() == 1 && !line.ends_with(' ') {
            let prefix = parts[0];
            for cmd in SHELL_COMMANDS {
                if cmd.starts_with(prefix) && *cmd != prefix {
                    return Some(cmd[prefix.len()..].to_string());
                }
            }
        }

        None
    }
}

impl Highlighter for ShellHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }
}

impl Validator for ShellHelper {}
impl Helper for ShellHelper {}
