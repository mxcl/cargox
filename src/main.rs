mod cli;
mod executor;
mod installer;
mod paths;
mod target;

use std::path::PathBuf;
use std::process::{ExitStatus, exit};

use anyhow::{Context, Result};

use cli::Cli;
use executor::execute_binary;
use installer::ensure_installed;
use paths::{resolve_binary_path, resolve_cargox_binary_path};
use target::{Target, parse_spec};

fn main() {
    match run_application() {
        Ok(status) => exit_with_status(status),
        Err(err) => exit_with_error(err),
    }
}

fn run_application() -> Result<ExitStatus> {
    let cli = parse_arguments()?;
    let target = parse_target_from_cli(&cli)?;

    if should_use_existing_binary(&cli, &target) {
        return run_existing_binary(&target, &cli);
    }

    install_and_run_binary(&target, &cli)
}

fn parse_arguments() -> Result<Cli> {
    Cli::parse_args()
}

fn parse_target_from_cli(cli: &Cli) -> Result<Target> {
    let (crate_name, version) = parse_spec(&cli.crate_spec)?;
    let binary = cli.bin.clone().unwrap_or_else(|| crate_name.clone());

    Ok(Target {
        crate_name,
        version,
        binary,
    })
}

fn should_use_existing_binary(cli: &Cli, target: &Target) -> bool {
    if cli.force {
        return false;
    }

    if target.version.is_some() {
        return false;
    }

    find_existing_binary(&target.binary).is_some()
}

fn run_existing_binary(target: &Target, cli: &Cli) -> Result<ExitStatus> {
    let binary_path = find_existing_binary(&target.binary)
        .expect("Binary should exist when this function is called");
    execute_binary(&binary_path, &cli.args)
}

fn install_and_run_binary(target: &Target, cli: &Cli) -> Result<ExitStatus> {
    ensure_installed(target, cli)?;
    let binary_path = locate_installed_binary(target)?;
    execute_binary(&binary_path, &cli.args)
}

fn find_existing_binary(name: &str) -> Option<PathBuf> {
    resolve_binary_path(name).ok()
}

fn locate_installed_binary(target: &Target) -> Result<PathBuf> {
    if target.version.is_some() {
        return resolve_cargox_binary_path(&target.binary).with_context(|| {
            format!(
                "{} should be available in cargox's install directory after installation",
                target.binary
            )
        });
    }

    resolve_binary_path(&target.binary)
        .with_context(|| format!("{} should be on PATH after installation", target.binary))
}

fn exit_with_status(status: ExitStatus) -> ! {
    if let Some(code) = status.code() {
        exit(code);
    } else {
        eprintln!("process terminated by signal");
        exit(1);
    }
}

fn exit_with_error(err: anyhow::Error) -> ! {
    eprintln!("error: {err}");
    let mut source = err.source();
    while let Some(next) = source {
        eprintln!("  caused by: {next}");
        source = next.source();
    }
    exit(1);
}
