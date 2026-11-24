use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oxgraft_core::{Available, Profile, Toolkit};

/// Oxgraft: the best of every coding agent, grafted into one.
#[derive(Debug, Parser)]
#[command(name = "oxgraft", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List every strategy and where it was learned from.
    Strategies {
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Validate a profile and print the session preamble it would produce.
    Profile {
        /// Path to a profile, e.g. profiles/default.toml.
        path: PathBuf,
        /// Print the JSONL event stream instead of a summary.
        #[arg(long)]
        json: bool,
    },
}

fn available() -> Available {
    Available {
        edit_formats: oxgraft_strategies::edit_formats(),
        contexts: oxgraft_strategies::contexts(),
        approvals: oxgraft_strategies::approvals(),
        sandboxes: oxgraft_strategies::sandboxes(),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("oxgraft: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Strategies { json } => {
            let catalog = oxgraft_strategies::catalog();
            if json {
                println!("{}", serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?);
            } else {
                println!("{:<12} {:<22} {:<11} {:<10} SUMMARY", "SEAM", "ID", "ORIGIN", "DONOR");
                for s in catalog {
                    let origin = serde_json::to_value(s.provenance.origin).map_err(|e| e.to_string())?;
                    println!(
                        "{:<12} {:<22} {:<11} {:<10} {}",
                        s.seam.as_str(),
                        s.id,
                        origin.as_str().unwrap_or("?"),
                        s.provenance.harness.unwrap_or("-"),
                        s.summary
                    );
                }
            }
        }
        Command::Profile { path, json } => {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile = Profile::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let toolkit = Toolkit::assemble(profile, available()).map_err(|e| e.to_string())?;
            if json {
                for event in toolkit.session_preamble() {
                    println!("{}", event.to_jsonl());
                }
            } else {
                println!("profile ok: {toolkit:?}");
            }
        }
    }
    Ok(())
}
