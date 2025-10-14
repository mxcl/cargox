use std::process::Command;

/// This test ensures that flags like --help after the crate spec are passed to the target binary
/// and not intercepted by cargox's own argument parser.
///
/// Regression test for: cargox bat --help should show bat's help, not cargox's help
#[test]
fn test_help_flag_passed_to_binary() {
    // Build the binary first
    let binary_path = env!("CARGO_BIN_EXE_cargox");

    // Run: cargox bat --help
    // We can't actually install bat in the test, but we can verify that:
    // 1. cargox doesn't show its own help (which would exit with code 0)
    // 2. The --help flag gets passed through to the execution logic
    //
    // Since bat isn't installed, this will fail trying to find/install bat,
    // but importantly it won't show cargox's help text
    let output = Command::new(binary_path)
        .args(["bat", "--help"])
        .output()
        .expect("Failed to execute cargox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The key assertion: cargox's help should NOT appear
    // If the help was intercepted, we'd see "Run Cargo binaries on demand" in the output
    assert!(
        !stdout.contains("Run Cargo binaries on demand"),
        "cargox help text appeared in stdout, indicating --help was intercepted.\nStdout:\n{}",
        stdout
    );
    assert!(
        !stderr.contains("Run Cargo binaries on demand"),
        "cargox help text appeared in stderr, indicating --help was intercepted.\nStderr:\n{}",
        stderr
    );

    // We should see some error about bat not being found/installed
    // or if it's already on PATH, it would run bat's help
    // Either way, we shouldn't see cargox's help
}

/// Test that cargox's own help still works when invoked without a crate spec
#[test]
fn test_cargox_help_still_works() {
    let binary_path = env!("CARGO_BIN_EXE_cargox");

    let output = Command::new(binary_path)
        .args(["--help"])
        .output()
        .expect("Failed to execute cargox");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // When --help comes before the crate spec, it should show cargox's help
    assert!(
        stdout.contains("Run Cargo binaries on demand"),
        "cargox help should appear when --help is used without a crate spec.\nStdout:\n{}",
        stdout
    );
    assert!(
        output.status.success(),
        "cargox --help should exit successfully"
    );
}

/// Test that flags before the crate spec are still parsed by cargox
#[test]
fn test_cargox_flags_before_crate_spec() {
    let binary_path = env!("CARGO_BIN_EXE_cargox");

    // Run: cargox --force bat
    // The --force flag should be recognized by cargox
    let output = Command::new(binary_path)
        .args(["--force", "bat"])
        .output()
        .expect("Failed to execute cargox");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // If --force was parsed correctly, cargox would proceed with installation
    // We should not see cargox's help text
    assert!(
        !stderr.contains("Run Cargo binaries on demand"),
        "cargox help text appeared, indicating argument parsing failed.\nStderr:\n{}",
        stderr
    );
}

/// Test that --bin flag works correctly
#[test]
fn test_bin_flag_parsing() {
    let binary_path = env!("CARGO_BIN_EXE_cargox");

    // Run: cargox --bin custom mycrate --help
    // The --help should be passed to the binary, not cargox
    let output = Command::new(binary_path)
        .args(["--bin", "custom", "mycrate", "--help"])
        .output()
        .expect("Failed to execute cargox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not show cargox help
    assert!(
        !stdout.contains("Run Cargo binaries on demand"),
        "cargox help appeared when it shouldn't.\nStdout:\n{}",
        stdout
    );
    assert!(
        !stderr.contains("Run Cargo binaries on demand"),
        "cargox help appeared when it shouldn't.\nStderr:\n{}",
        stderr
    );
}
