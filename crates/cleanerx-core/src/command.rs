use crate::models::ScanLog;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
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
        stdout: String,
    },
    #[error("{program} timed out after {seconds}s")]
    Timeout { program: String, seconds: u64 },
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub canceled: bool,
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

    command_output(program, output)
}

pub fn run_partial_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, CommandError> {
    run_partial_with_timeout_and_cancel(program, args, timeout, None)
}

pub fn run_partial_with_timeout_and_cancel(
    program: &str,
    args: &[&str],
    timeout: Duration,
    cancel: Option<&AtomicBool>,
) -> Result<CommandOutput, CommandError> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CommandError::Spawn {
            program: program.to_string(),
            message: error.to_string(),
        })?;

    let stdout = child.stdout.take().ok_or_else(|| CommandError::Spawn {
        program: program.to_string(),
        message: "failed to capture stdout".to_string(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| CommandError::Spawn {
        program: program.to_string(),
        message: "failed to capture stderr".to_string(),
    })?;
    let stdout_reader = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut buffer = String::new();
        let _ = reader.read_to_string(&mut buffer);
        buffer
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut buffer = String::new();
        let _ = reader.read_to_string(&mut buffer);
        buffer
    });

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|error| CommandError::Spawn {
            program: program.to_string(),
            message: error.to_string(),
        })? {
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default().trim().to_string();
            return Ok(CommandOutput {
                stdout,
                stderr,
                status: status.code(),
                success: status.success(),
                timed_out: false,
                canceled: false,
            });
        }

        if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            let _ = child.kill();
            let _ = child.wait();
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default().trim().to_string();
            return Ok(CommandOutput {
                stdout,
                stderr,
                status: None,
                success: false,
                timed_out: false,
                canceled: true,
            });
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default().trim().to_string();
            return Ok(CommandOutput {
                stdout,
                stderr,
                status: None,
                success: false,
                timed_out: true,
                canceled: false,
            });
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn command_output(
    program: &str,
    output: std::process::Output,
) -> Result<CommandOutput, CommandError> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let status = output.status.code();
    let success = output.status.success();

    if !success {
        return Err(CommandError::Failed {
            program: program.to_string(),
            status: status
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            stderr,
            stdout,
        });
    }

    Ok(CommandOutput {
        stdout,
        stderr,
        status,
        success,
        timed_out: false,
        canceled: false,
    })
}

pub fn log_command_error(context: &str, error: &CommandError) -> ScanLog {
    ScanLog::warning(format!("{context}: {error}"))
}
