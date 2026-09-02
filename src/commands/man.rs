use clap::CommandFactory;
use std::fs::File;
use std::path::Path;

use crate::cli::Cli;
use crate::error::CliError;

pub fn handle_man(dir: Option<&str>) -> Result<(), CliError> {
    let cmd = Cli::command();

    if let Some(target_dir) = dir {
        let path = Path::new(target_dir);
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| {
                CliError::Io(format!(
                    "Failed to create man directory '{target_dir}': {e}"
                ))
            })?;
        }

        let out_file_path = path.join("actos.1");
        let mut out_file = File::create(&out_file_path)
            .map_err(|e| CliError::Io(format!("Failed to create man file: {e}")))?;

        clap_mangen::Man::new(cmd)
            .render(&mut out_file)
            .map_err(|e| CliError::Io(format!("Failed to render man page: {e}")))?;

        println!("Man page generated at {}", out_file_path.display());
    } else {
        clap_mangen::Man::new(cmd)
            .render(&mut std::io::stdout())
            .map_err(|e| CliError::Io(format!("Failed to render man page: {e}")))?;
    }

    Ok(())
}
