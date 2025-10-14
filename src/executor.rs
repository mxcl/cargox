use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitStatus};

pub fn execute_binary(binary_path: &Path, args: &[OsString]) -> Result<ExitStatus> {
    let mut cmd = Command::new(binary_path);
    cmd.args(args);

    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {}", binary_path.display()))?;

    Ok(status)
}
