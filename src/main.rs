mod cli;
mod executor;
mod installer;
mod paths;
mod registry;
mod target;
mod versions;

use std::path::PathBuf;
use std::process::{ExitStatus, exit};

use anyhow::Result;
use semver::{Version, VersionReq};

use cli::Cli;
use executor::execute_binary;
use installer::ensure_installed;
use paths::resolve_binary_path;
use registry::{fetch_highest_matching_version, fetch_latest_version};
use target::{Target, VersionSpec, parse_spec};
use versions::{find_installed_version, latest_installed, versioned_binary_path};

enum RunPlan {
    UseInstalled { path: PathBuf },
    UseSystem { path: PathBuf },
    InstallAndRun { version: Version },
}

fn main() {
    match run_application() {
        Ok(status) => exit_with_status(status),
        Err(err) => exit_with_error(err),
    }
}

fn run_application() -> Result<ExitStatus> {
    let cli = parse_arguments()?;
    let target = parse_target_from_cli(&cli)?;

    let plan = resolve_run_plan(&target, &cli)?;
    execute_plan(&plan, &target, &cli)
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

fn resolve_run_plan(target: &Target, cli: &Cli) -> Result<RunPlan> {
    match &target.version {
        VersionSpec::Unspecified => resolve_unspecified(target, cli),
        VersionSpec::Latest => resolve_latest(target, cli),
        VersionSpec::Requirement(requirement) => resolve_requirement(target, cli, requirement),
    }
}

fn resolve_unspecified(target: &Target, cli: &Cli) -> Result<RunPlan> {
    if !cli.force {
        if let Some(installed) = latest_installed(&target.binary)? {
            return Ok(RunPlan::UseInstalled {
                path: installed.path,
            });
        }

        if let Ok(path) = resolve_binary_path(&target.binary) {
            return Ok(RunPlan::UseSystem { path });
        }
    }

    let version = fetch_latest_version(&target.crate_name)?;
    Ok(RunPlan::InstallAndRun { version })
}

fn resolve_latest(target: &Target, cli: &Cli) -> Result<RunPlan> {
    let installed = latest_installed(&target.binary)?;
    let remote = fetch_latest_version(&target.crate_name)?;

    if cli.force {
        return Ok(RunPlan::InstallAndRun { version: remote });
    }

    if let Some(installed) = installed
        && installed.version >= remote
    {
        return Ok(RunPlan::UseInstalled {
            path: installed.path,
        });
    }

    Ok(RunPlan::InstallAndRun { version: remote })
}

fn resolve_requirement(target: &Target, cli: &Cli, requirement: &VersionReq) -> Result<RunPlan> {
    if !cli.force
        && let Some(installed) = find_installed_version(&target.binary, requirement)?
    {
        return Ok(RunPlan::UseInstalled {
            path: installed.path,
        });
    }

    let version = fetch_highest_matching_version(&target.crate_name, Some(requirement))?;
    Ok(RunPlan::InstallAndRun { version })
}

fn execute_plan(plan: &RunPlan, target: &Target, cli: &Cli) -> Result<ExitStatus> {
    match plan {
        RunPlan::UseInstalled { path } => execute_binary(path, &cli.args),
        RunPlan::UseSystem { path } => execute_binary(path, &cli.args),
        RunPlan::InstallAndRun { version } => {
            ensure_installed(target, cli, version)?;
            let binary_path = versioned_binary_path(&target.binary, version)?;
            execute_binary(&binary_path, &cli.args)
        }
    }
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
