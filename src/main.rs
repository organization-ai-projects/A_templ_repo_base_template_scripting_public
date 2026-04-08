//! CLI binary providing automation primitives for repo_base_template scripts.
//! Called from GitHub Actions workflows and shell scripts.
mod cli;
mod commands;
mod regex;

use crate::cli::AutomationCli;
use crate::commands::CommandArgs;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("");

    match command {
        "" | "help" | "--help" | "-h" => {
            println!("Usage: <command> [options]");
            println!("Available commands: automation");
            process::exit(0);
        }
        "automation" => {
            let command_args = CommandArgs::new(args.clone());
            let cli = AutomationCli::new(command_args);
            let subcommand = args.get(2).map(String::as_str).unwrap_or("");
            cli.handle_subcommand(subcommand);
        }
        unknown => {
            eprintln!("Unknown command: {unknown}");
            eprintln!("Run 'help' for available commands.");
            process::exit(1);
        }
    }
}
