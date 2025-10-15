use anyhow::{Result, anyhow};
use clap::Parser;
use std::env;
use std::ffi::OsString;

/// Run Cargo binaries on demand, installing them via `cargo-binstall` when missing.
#[derive(Parser, Debug)]
#[command(name = "cargox", author, version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// Crate to run, optionally suffixed with `@version`
    #[arg(value_name = "crate[@version]")]
    pub crate_spec: String,

    /// Execute this binary from the crate (defaults to crate name)
    #[arg(long, value_name = "NAME")]
    pub bin: Option<String>,

    /// Force reinstall; ignore any existing binary
    #[arg(short, long)]
    pub force: bool,

    /// Suppress installer output
    #[arg(short, long)]
    pub quiet: bool,

    /// Build from source using `cargo install` instead of `cargo-binstall`
    #[arg(short = 's', long)]
    pub build_from_source: bool,

    /// Arguments passed to the executed binary (use `--` to delimit)
    #[arg(trailing_var_arg = true, value_name = "binary-args")]
    pub args: Vec<OsString>,
}

impl Cli {
    /// Parse arguments, ensuring that arguments after the crate spec are passed to the binary
    /// rather than being intercepted by clap. This allows `cargox bat --help` to show bat's
    /// help rather than cargox's help.
    pub fn parse_args() -> Result<Self> {
        let mut args: Vec<OsString> = env::args_os().collect();

        // Skip the program name
        if args.is_empty() {
            return Err(anyhow!("no program name in arguments"));
        }
        args.remove(0);

        // Find the first positional argument (crate spec) by iterating through args
        // and stopping at the first argument that doesn't start with `-` and isn't a value for a flag
        let mut crate_spec_idx = None;
        let mut i = 0;
        let mut skip_next = false;

        while i < args.len() {
            if skip_next {
                skip_next = false;
                i += 1;
                continue;
            }

            let arg = args[i].to_string_lossy();

            // Check if this is a flag that takes a value
            if arg == "--bin" {
                skip_next = true;
                i += 1;
                continue;
            }

            // If it doesn't start with `-`, it's the crate spec
            if !arg.starts_with('-') {
                crate_spec_idx = Some(i);
                break;
            }

            i += 1;
        }

        // If we found a crate spec, split args at that point
        let (cargox_args, binary_args) = if let Some(idx) = crate_spec_idx {
            let mut cargox_args = args[..idx].to_vec();
            // Add the crate spec to cargox args
            cargox_args.push(args[idx].clone());
            // Everything after crate spec goes to the binary
            let binary_args = args[idx + 1..].to_vec();
            (cargox_args, binary_args)
        } else {
            // No crate spec found, let clap handle it (will show help or error)
            (args, vec![])
        };

        // Parse cargox arguments with clap
        let mut cli =
            match Cli::try_parse_from(std::iter::once(OsString::from("cargox")).chain(cargox_args))
            {
                Ok(cli) => cli,
                Err(e) => {
                    // Let clap print the error/help message and exit
                    e.exit();
                }
            };

        // Set the binary arguments
        cli.args = binary_args;

        Ok(cli)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_separates_binary_args_correctly() {
        // Simulate: cargox bat --help
        let result = Cli::try_parse_from(["cargox", "bat", "--help"]);
        // This would fail because clap would see --help and try to show cargox's help
        // Our custom parse_args should prevent this by separating the args
        assert!(result.is_err()); // Without custom parsing, this intercepts --help

        // The custom parse needs to be tested differently since it reads from env::args_os()
        // We'll add an integration test for this behavior
    }

    #[test]
    fn parse_args_handles_bin_flag() {
        let cli = Cli::try_parse_from(["cargox", "--bin", "foo", "mycrate"]).unwrap();
        assert_eq!(cli.crate_spec, "mycrate");
        assert_eq!(cli.bin, Some("foo".to_string()));
        assert_eq!(cli.args.len(), 0);
    }

    #[test]
    fn parse_args_handles_force_flag() {
        let cli = Cli::try_parse_from(["cargox", "-f", "mycrate"]).unwrap();
        assert_eq!(cli.crate_spec, "mycrate");
        assert!(cli.force);
    }
}
