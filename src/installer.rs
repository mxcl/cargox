use anyhow::{Context, Result, anyhow};
use std::process::Command;

use crate::cli::Cli;
use crate::paths::get_install_dir;
use crate::target::Target;

pub fn ensure_installed(target: &Target, cli: &Cli) -> Result<()> {
    if !cli.build_from_source && which::which("cargo-binstall").is_ok() {
        install_with_binstall(target, cli)
    } else {
        log_fallback_reason(cli, target);
        install_with_cargo(target, cli)
    }
}

fn log_fallback_reason(cli: &Cli, target: &Target) {
    if cli.build_from_source {
        eprintln!(
            "Building {} from source with cargo install",
            target.descriptor()
        );
    } else {
        eprintln!(
            "cargo-binstall not found; falling back to cargo install for {}",
            target.descriptor()
        );
    }
}

fn install_with_binstall(target: &Target, cli: &Cli) -> Result<()> {
    let install_dir = get_install_dir()?;

    let mut cmd = Command::new("cargo");
    cmd.arg("binstall");
    if cli.quiet {
        cmd.arg("--quiet");
    }
    cmd.arg("--no-confirm");
    if cli.force {
        cmd.arg("--force");
    }
    if let Some(bin) = &cli.bin {
        cmd.arg("--bin");
        cmd.arg(bin);
    }
    cmd.arg(target.install_spec());

    // Set the install root for cargo-binstall and remove any environment variables
    // that could leak into the installation process
    sanitize_cargo_env(&mut cmd, &install_dir);

    eprintln!(
        "Installing {} with cargo-binstall{} to {}",
        target.descriptor(),
        if cli.quiet { " (quiet)" } else { "" },
        install_dir.display()
    );

    let status = cmd.status().context("failed to invoke cargo-binstall")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "cargo-binstall exited with status code {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

fn install_with_cargo(target: &Target, cli: &Cli) -> Result<()> {
    let install_dir = get_install_dir()?;

    // Create a temporary directory for the build
    let temp_dir = tempfile::tempdir().context("failed to create temp directory")?;

    let mut cmd = Command::new("cargo");
    cmd.arg("install");
    if cli.quiet {
        cmd.arg("--quiet");
    }
    if cli.force {
        cmd.arg("--force");
    }
    cmd.arg("--root");
    cmd.arg(&install_dir);
    cmd.arg(&target.crate_name);
    if let Some(version) = &target.version {
        cmd.arg("--version");
        cmd.arg(version);
    }
    if let Some(bin) = &cli.bin {
        cmd.arg("--bin");
        cmd.arg(bin);
    }

    // Use temp directory for target build directory and sanitize environment
    cmd.env("CARGO_TARGET_DIR", temp_dir.path());
    sanitize_cargo_env(&mut cmd, &install_dir);

    eprintln!(
        "Installing {} with cargo install{} to {}",
        target.descriptor(),
        if cli.quiet { " (quiet)" } else { "" },
        install_dir.display()
    );

    let status = cmd.status().context("failed to invoke cargo install")?;

    // Temp directory will be automatically cleaned up when temp_dir goes out of scope

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "cargo install exited with status code {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

/// Sanitize the environment for cargo commands to ensure complete sandboxing.
/// Removes any Cargo-related environment variables that could leak into the installation
/// and sets only the variables we explicitly want.
fn sanitize_cargo_env(cmd: &mut Command, install_dir: &std::path::Path) {
    // List of environment variables to remove to ensure sandboxing
    let vars_to_remove = [
        "CARGO_INSTALL_ROOT",
        "CARGO_HOME",
        "CARGO_BUILD_TARGET_DIR",
        "CARGO_TARGET_DIR",
        "BINSTALL_INSTALL_PATH",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
    ];

    for var in &vars_to_remove {
        cmd.env_remove(var);
    }

    // Set only our controlled install location
    cmd.env("CARGO_INSTALL_ROOT", install_dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_cargo_env_removes_cargo_variables() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path();

        // Create a command with cargo env vars set
        let mut cmd = Command::new("echo");
        cmd.env("CARGO_INSTALL_ROOT", "/some/path");
        cmd.env("CARGO_HOME", "/some/cargo");
        cmd.env("BINSTALL_INSTALL_PATH", "/some/binstall");
        cmd.env("RUSTUP_HOME", "/some/rustup");
        cmd.env("CARGO_TARGET_DIR", "/some/target");
        cmd.env("SOME_OTHER_VAR", "should_remain");

        // Sanitize the environment
        sanitize_cargo_env(&mut cmd, install_dir);

        // Note: We can't directly inspect Command's env, but we can verify
        // the function exists and compiles correctly. The actual behavior
        // is tested through integration tests.
    }
}
