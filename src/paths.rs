use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn get_install_dir() -> Result<PathBuf> {
    // First check if user has explicitly set an install path
    if let Some(path) = env::var_os("CARGOX_INSTALL_DIR") {
        return Ok(PathBuf::from(path));
    }

    // Use XDG data directory for Linux/Unix or equivalent on other platforms
    if let Some(proj_dirs) = ProjectDirs::from("", "", "cargox") {
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)
            .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;
        return Ok(data_dir.to_path_buf());
    }

    // Fallback to .local/share/cargox
    if let Some(home) = home_dir() {
        let fallback = home.join(".local").join("share").join("cargox");
        fs::create_dir_all(&fallback).with_context(|| {
            format!(
                "failed to create fallback directory: {}",
                fallback.display()
            )
        })?;
        return Ok(fallback);
    }

    Err(anyhow!("unable to determine install directory"))
}

pub fn resolve_binary_path(name: &str) -> Result<PathBuf> {
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    for dir in candidate_bin_dirs() {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
        #[cfg(windows)]
        {
            let exe_candidate = candidate.with_extension("exe");
            if exe_candidate.exists() {
                return Ok(exe_candidate);
            }
        }
    }

    Err(anyhow!("cannot find binary path"))
}

fn candidate_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Only check our sandboxed install directory
    if let Ok(install_dir) = get_install_dir() {
        dirs.push(install_dir.join("bin"));
        dirs.push(install_dir);
    }

    dirs
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_install_dir_respects_cargox_install_dir() {
        let temp = tempfile::tempdir().unwrap();
        let custom_path = temp.path().to_path_buf();

        unsafe {
            env::set_var("CARGOX_INSTALL_DIR", &custom_path);
        }
        let result = get_install_dir().unwrap();
        unsafe {
            env::remove_var("CARGOX_INSTALL_DIR");
        }

        assert_eq!(result, custom_path);
    }

    #[test]
    fn get_install_dir_ignores_cargo_install_root() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_root = temp.path().join("cargo_root");

        // Set CARGO_INSTALL_ROOT - it should be ignored
        unsafe {
            env::set_var("CARGO_INSTALL_ROOT", &cargo_root);
        }
        let result = get_install_dir().unwrap();
        unsafe {
            env::remove_var("CARGO_INSTALL_ROOT");
        }

        // Should NOT use CARGO_INSTALL_ROOT, should use XDG directories instead
        assert_ne!(result, cargo_root);
        assert!(result.to_string_lossy().contains("cargox"));
    }

    #[test]
    fn get_install_dir_uses_xdg_directories() {
        // Clear any override env vars
        unsafe {
            env::remove_var("CARGOX_INSTALL_DIR");
        }

        let result = get_install_dir().unwrap();

        // Should contain "cargox" in the path (XDG directory)
        assert!(result.to_string_lossy().contains("cargox"));

        // Should be platform-appropriate
        #[cfg(target_os = "macos")]
        assert!(result.to_string_lossy().contains("Application Support"));

        #[cfg(target_os = "linux")]
        assert!(result.to_string_lossy().contains(".local/share"));

        #[cfg(target_os = "windows")]
        assert!(result.to_string_lossy().contains("AppData"));
    }

    #[test]
    fn candidate_bin_dirs_only_checks_cargox_dir() {
        // Clear any override env vars
        unsafe {
            env::remove_var("CARGOX_INSTALL_DIR");
        }

        let dirs = candidate_bin_dirs();

        // Should only have 2 entries: install_dir/bin and install_dir
        assert_eq!(dirs.len(), 2);

        // Both should contain "cargox"
        for dir in &dirs {
            assert!(dir.to_string_lossy().contains("cargox"));
        }

        // Should NOT contain any standard cargo directories
        let dir_strings: Vec<String> = dirs
            .iter()
            .map(|d| d.to_string_lossy().to_string())
            .collect();

        for dir_str in &dir_strings {
            assert!(
                !dir_str.contains("/.cargo/bin"),
                "Should not check ~/.cargo/bin, found: {dir_str}"
            );
            assert!(
                !dir_str.contains("\\.cargo\\bin"),
                "Should not check ~/.cargo/bin, found: {dir_str}"
            );
        }
    }

    #[test]
    fn candidate_bin_dirs_ignores_cargo_env_vars() {
        let temp = tempfile::tempdir().unwrap();

        // Set various cargo-related env vars
        unsafe {
            env::set_var("CARGO_INSTALL_ROOT", temp.path().join("cargo_root"));
            env::set_var("CARGO_HOME", temp.path().join("cargo_home"));
            env::set_var("BINSTALL_INSTALL_PATH", temp.path().join("binstall"));
        }

        let dirs = candidate_bin_dirs();

        // Clean up
        unsafe {
            env::remove_var("CARGO_INSTALL_ROOT");
            env::remove_var("CARGO_HOME");
            env::remove_var("BINSTALL_INSTALL_PATH");
        }

        // None of the candidate dirs should match the env var paths
        let dir_strings: Vec<String> = dirs
            .iter()
            .map(|d| d.to_string_lossy().to_string())
            .collect();

        for dir_str in &dir_strings {
            assert!(
                !dir_str.contains("cargo_root"),
                "Should ignore CARGO_INSTALL_ROOT, found: {dir_str}"
            );
            assert!(
                !dir_str.contains("cargo_home"),
                "Should ignore CARGO_HOME, found: {dir_str}"
            );
            assert!(
                !dir_str.contains("binstall"),
                "Should ignore BINSTALL_INSTALL_PATH, found: {dir_str}"
            );
        }
    }

    #[test]
    fn resolve_binary_path_checks_which_first() {
        // This test verifies that we check PATH first via which
        // We use a binary that's definitely on PATH (like "ls" on Unix or "cmd" on Windows)
        #[cfg(unix)]
        let result = resolve_binary_path("ls");

        #[cfg(windows)]
        let result = resolve_binary_path("cmd");

        // Should find the system binary via which
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_binary_path_falls_back_to_cargox_dir() {
        // Clear any override env vars
        unsafe {
            env::remove_var("CARGOX_INSTALL_DIR");
        }

        // Try to find a binary that doesn't exist
        let result = resolve_binary_path("this_binary_definitely_does_not_exist_12345");

        // Should fail because it's not in PATH and not in our install dir
        assert!(result.is_err());
    }

    #[test]
    fn cargox_install_dir_override_is_respected() {
        let temp = tempfile::tempdir().unwrap();
        let custom_dir = temp.path().join("my_custom_cargox");

        unsafe {
            env::set_var("CARGOX_INSTALL_DIR", &custom_dir);
        }

        let dirs = candidate_bin_dirs();

        unsafe {
            env::remove_var("CARGOX_INSTALL_DIR");
        }

        // Should use the custom directory
        assert!(dirs.iter().any(|d| d == &custom_dir.join("bin")));
        assert!(dirs.iter().any(|d| d == &custom_dir));
    }
}
