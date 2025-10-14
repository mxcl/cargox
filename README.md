# cargox

[![CI](https://github.com/mxcl/cargox/actions/workflows/ci.yml/badge.svg)](https://github.com/mxcl/cargox/actions/workflows/ci.yml)

`cargox` runs Rust binaries on demand, installing them automatically if necessary.
It mirrors the convenience of `npx` for the Cargo ecosystem while prioritising
`cargo-binstall` to download prebuilt executables whenever possible.

## Features

- Executes `crate[@version]` binaries, installing them on demand.
- Prefers `cargo-binstall` for fast installs falling back to `cargo install`.
- Passes through additional arguments to the invoked binary via `--`.

## Usage

```bash
cargox <crate[@version]> [--] [binary-args...]
```

Examples:

```bash
# Run the latest wasm-pack, installing it if necessary
cargox bat ./README.md

# Install and run a pinned version
cargox cargo-deny@0.16.3 check

# Force a reinstall, building from source instead of using cargo-binstall
cargox --force --build-from-source cargo-nextest
```

> [!TIP]
>
> - Arguments before the first positional are passed to `cargox`.
> - Arguments after `--` are passed to the invoked binary.
> - Use `--` if necessary to define the separation point.

### Flags

- `--bin <name>`: choose a specific binary when a crate exposes several.
- `-f`, `--force`: reinstall even if the binary already exists on `PATH`.
- `-q`, `--quiet`: suppress installer output (still prints a short status line).
- `-s`, `--build-from-source`: build from source using `cargo install` instead of `cargo-binstall`.

### Where binaries are stored

`cargox` operates in a **completely sandboxed environment**, isolated from your
system's Cargo installation. This ensures that binaries installed by `cargox` are
separate and don't interfere with your regular `cargo install` workflow.

**Default install locations:**

- **Linux/Unix**: `~/.local/share/cargox/bin` (XDG Data Directory)
- **macOS**: `~/Library/Application Support/cargox/bin`
- **Windows**: `%APPDATA%\cargox\bin`

**Customizing the install directory:**

You can override the default by setting:

- `CARGOX_INSTALL_DIR`: Custom location for `cargox` installations

**Complete sandboxing:**

`cargox` ensures complete isolation by:

1. **Not checking standard Cargo directories**: Binaries in `~/.cargo/bin`,
   `~/.local/bin`, `/usr/local/bin`, or `CARGO_HOME/bin` are ignored when looking
   for already-installed binaries.

2. **Binary lookup is restricted to**:
   - Binaries already on your `PATH` (via `which`)
   - The `cargox` install directory only

3. **Environment isolation**: When installing packages, `cargox` removes all
   Cargo-related environment variables (like `CARGO_INSTALL_ROOT`, `CARGO_HOME`,
   `BINSTALL_INSTALL_PATH`, etc.) to prevent any leakage into the installation
   process. Only the controlled `cargox` install directory is set.

This sandboxing guarantees that:

- You can test different versions without affecting your system installations
- `cargox` binaries won't accidentally shadow your regular Cargo binaries
- The installation process is predictable and reproducible

**Build artifact cleanup:**

`cargox` automatically cleans up build artifacts after installation:

- When using `cargo-binstall`, binaries are downloaded pre-built (no artifacts to clean)
- When using `cargo install`, build artifacts are placed in a temporary directory
  that is automatically cleaned up after installation completes

This keeps your system clean and prevents build cache bloat.
