#![forbid(unsafe_code)]

use actos::cli::{Cli, Commands};
use actos::commands::config::handle_config;
use actos::error::{CliError, ExitCode};
use clap::Parser;

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Config(args) => handle_config(args.action, cli.profile.as_deref(), cli.json),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_json = args.iter().any(|arg| arg == "--json");

    match Cli::try_parse() {
        Ok(cli) => {
            let json_mode = cli.json;
            if let Err(err) = run(cli) {
                err.render(json_mode);
                std::process::exit(err.exit_code().as_i32());
            }
        }
        Err(err) => {
            use clap::error::ErrorKind;
            match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    let _ = err.print();
                    std::process::exit(ExitCode::Success.as_i32());
                }
                _ => {
                    if is_json {
                        let cli_err = CliError::Usage(err.to_string());
                        cli_err.render(true);
                    } else {
                        let _ = err.print();
                    }
                    std::process::exit(ExitCode::UsageError.as_i32());
                }
            }
        }
    }
}
