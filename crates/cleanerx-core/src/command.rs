use crate::models::ScanLog;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("failed to run {program}: {message}")]
    Spawn { program: String, message: String },
    #[error("{program} exited with status {status}: {stderr}")]
    Failed {
        program: String,
        status: String,
        stderr: String,
    },
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
}

pub fn run(program: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
    let output =
        Command::new(program)
            .args(args)
            .output()
            .map_err(|error| CommandError::Spawn {
                program: program.to_string(),
                message: error.to_string(),
            })?;

    if !output.status.success() {
        return Err(CommandError::Failed {
            program: program.to_string(),
            status: output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}

pub fn log_command_error(context: &str, error: &CommandError) -> ScanLog {
    ScanLog::warning(format!("{context}: {error}"))
}
