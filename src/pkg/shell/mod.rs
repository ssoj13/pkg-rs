//! Interactive shell (REPL) for pkg.

mod commands;
mod helper;

use commands::{shell_env, shell_help, shell_info, shell_list, shell_run, shell_solve};
use helper::ShellHelper;
use pkg_lib::Storage;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor};
use std::path::PathBuf;
use std::process::ExitCode;

/// Shell command names for completion.
pub const SHELL_COMMANDS: &[&str] = &[
    "list", "ls", "info", "run", "env", "solve", "scan", "help", "exit", "quit", "q",
];

/// History file name.
const SHELL_HISTORY_FILE: &str = ".pkg_history";

/// Interactive shell command.
pub fn cmd_shell(mut storage: Storage) -> ExitCode {
    println!("pkg shell v{}", pkg_lib::VERSION);
    println!("Type 'help' for commands, 'exit' to quit.\n");

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();

    let helper = ShellHelper::new(&storage);
    let mut rl: Editor<ShellHelper, DefaultHistory> = match Editor::with_config(config) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to init shell: {}", e);
            return ExitCode::FAILURE;
        }
    };
    rl.set_helper(Some(helper));

    // Load history
    let history_path = dirs::home_dir()
        .map(|p| p.join(SHELL_HISTORY_FILE))
        .unwrap_or_else(|| PathBuf::from(SHELL_HISTORY_FILE));
    let _ = rl.load_history(&history_path);

    // REPL loop
    loop {
        let prompt = format!("pkg ({})> ", storage.packages().len());

        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);
                let parts: Vec<&str> = line.split_whitespace().collect();
                let cmd = parts[0];
                let args = &parts[1..];

                match cmd {
                    "help" | "?" => shell_help(),
                    "exit" | "quit" | "q" => break,
                    "list" | "ls" => shell_list(&storage, args),
                    "info" => shell_info(&storage, args),
                    "run" => shell_run(&storage, args),
                    "env" => shell_env(&storage, args),
                    "solve" => shell_solve(&storage, args),
                    "scan" => {
                        match Storage::scan() {
                            Ok(s) => {
                                storage = s;
                                if let Some(helper) = rl.helper_mut() {
                                    helper.update(&storage);
                                }
                                println!("Rescanned: {} packages", storage.packages().len());
                            }
                            Err(e) => eprintln!("Scan failed: {}", e),
                        }
                    }
                    _ => eprintln!("Unknown command: {}. Type 'help' for commands.", cmd),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    ExitCode::SUCCESS
}
