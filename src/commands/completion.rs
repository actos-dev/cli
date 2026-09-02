use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::cli::Cli;
use crate::error::CliError;

pub fn handle_completion(shell_name: &str) -> Result<(), CliError> {
    let shell = match shell_name.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            return Err(CliError::Usage(format!(
                "Unsupported shell '{shell_name}'. Supported: bash, zsh, fish, powershell, elvish"
            )));
        }
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "actos", &mut std::io::stdout());

    Ok(())
}
