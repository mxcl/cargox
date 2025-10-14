# Testing Documentation

This document describes the test suite for `cargox`, particularly focusing on sandboxing and directory isolation guarantees.

## Test Categories

### 1. Spec Parsing Tests

- `split_spec_without_version` - Verifies parsing of crate names without version specifiers
- `split_spec_with_version` - Verifies parsing of crate names with `@version` syntax
- `split_spec_rejects_empty` - Ensures invalid specs are rejected

### 2. Argument Parsing Tests

These tests verify that command-line arguments are correctly parsed and that arguments intended for the target binary are not intercepted by cargox.

#### Integration Tests (`tests/help_flag.rs`)

**Critical regression prevention tests** for ensuring `cargox bat --help` shows bat's help, not cargox's help:

- `test_help_flag_passed_to_binary` - **Most critical test**: Verifies that flags like `--help` after the crate spec are passed to the target binary and not intercepted by cargox's argument parser. Ensures we don't regress the fix for this common use case.

- `test_cargox_help_still_works` - Verifies that `cargox --help` (without a crate spec) still shows cargox's own help text.

- `test_cargox_flags_before_crate_spec` - Verifies that flags before the crate spec (like `cargox --force bat`) are correctly parsed by cargox.

- `test_bin_flag_parsing` - Verifies that the `--bin` flag works correctly and that subsequent arguments are passed to the binary.

#### Unit Tests (`src/cli.rs`)

- `parse_args_separates_binary_args_correctly` - Unit test demonstrating the difference between standard clap parsing and custom argument separation
- `parse_args_handles_bin_flag` - Verifies `--bin` flag parsing
- `parse_args_handles_force_flag` - Verifies `-f`/`--force` flag parsing

### 3. Install Directory Tests

These tests verify that `cargox` uses the correct, sandboxed installation directories:

#### `get_install_dir_respects_cargox_install_dir`
Ensures that when `CARGOX_INSTALL_DIR` is set, it takes precedence over all other directory determination logic.

#### `get_install_dir_ignores_cargo_install_root`
**Critical sandboxing test**: Verifies that `CARGO_INSTALL_ROOT` is completely ignored, even when set. This ensures we don't accidentally use the user's standard Cargo install location.

#### `get_install_dir_uses_xdg_directories`
Verifies that when no override is set, we use platform-appropriate XDG directories:
- macOS: `~/Library/Application Support/cargox`
- Linux: `~/.local/share/cargox`
- Windows: `%APPDATA%\cargox`

### 3. Directory Isolation Tests

These tests ensure we don't check or use standard Cargo/system directories:

#### `candidate_bin_dirs_only_checks_cargox_dir`
**Critical sandboxing test**: Verifies that:
- Only 2 directories are checked (our install dir and install dir/bin)
- No standard directories like `~/.cargo/bin` are included
- All checked directories contain "cargox" in the path

#### `candidate_bin_dirs_ignores_cargo_env_vars`
**Critical sandboxing test**: Verifies that even when Cargo-related environment variables are set, they are completely ignored:
- `CARGO_INSTALL_ROOT` - ignored
- `CARGO_HOME` - ignored
- `BINSTALL_INSTALL_PATH` - ignored

This ensures complete isolation from the user's Cargo configuration.

### 4. Binary Resolution Tests

#### `resolve_binary_path_checks_which_first`
Verifies that we check `PATH` first via the `which` crate, allowing users to override with binaries already in their PATH.

#### `resolve_binary_path_falls_back_to_cargox_dir`
Verifies that after checking PATH, we fall back to our sandboxed directory.

### 5. Environment Sanitization Tests

#### `sanitize_cargo_env_removes_cargo_variables`
Verifies that the `sanitize_cargo_env` function exists and compiles correctly. The actual environment sanitization is tested through integration tests since we can't directly inspect a `Command`'s environment.

The function removes these variables before calling `cargo install` or `cargo-binstall`:
- `CARGO_INSTALL_ROOT`
- `CARGO_HOME`
- `CARGO_BUILD_TARGET_DIR`
- `CARGO_TARGET_DIR`
- `BINSTALL_INSTALL_PATH`
- `RUSTUP_HOME`
- `RUSTUP_TOOLCHAIN`

### 6. Override Tests

#### `cargox_install_dir_override_is_respected`
Verifies that users can override the install directory with `CARGOX_INSTALL_DIR` and that the override is properly respected.

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests serially (useful for env var tests)
cargo test -- --test-threads=1

# Run a specific test
cargo test test_name

# Run tests in release mode
cargo test --release
```

## Sandboxing Guarantees

The test suite ensures these critical sandboxing guarantees:

1. **Directory Isolation**: `cargox` never checks standard Cargo directories like `~/.cargo/bin`, `~/.local/bin`, or `/usr/local/bin`

2. **Environment Variable Isolation**: Cargo-related environment variables are completely ignored when determining install locations

3. **Installation Isolation**: When installing packages, all Cargo environment variables are removed to prevent any leakage

4. **Predictable Behavior**: The install directory is always deterministic based on platform XDG directories or explicit override

5. **No Cross-Contamination**: Binaries installed via `cargox` cannot interfere with binaries installed via standard `cargo install`

## Safety Notes

Some tests use `unsafe` blocks for `env::set_var` and `env::remove_var` because these functions can cause undefined behavior if called while other threads are reading environment variables. In our test suite, this is acceptable because:

1. Tests that modify environment variables run serially (not in parallel)
2. Environment modifications are always cleaned up before test completion
3. The modifications are only for testing purposes and don't affect production code
