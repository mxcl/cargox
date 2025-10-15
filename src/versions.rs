use crate::paths::get_install_dir;
use anyhow::{Context, Result};
use semver::{Version, VersionReq};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InstalledBinary {
    pub version: Version,
    pub path: PathBuf,
}

pub fn versioned_binary_name(binary: &str, version: &Version) -> String {
    format!("{binary}-{version}")
}

pub fn versioned_binary_path(binary: &str, version: &Version) -> Result<PathBuf> {
    let bin_dir = ensure_bin_dir()?;
    #[cfg(windows)]
    let path = bin_dir.join(format!("{}.exe", versioned_binary_name(binary, version)));
    #[cfg(not(windows))]
    let path = bin_dir.join(versioned_binary_name(binary, version));
    Ok(path)
}

pub fn list_installed_versions(binary: &str) -> Result<Vec<InstalledBinary>> {
    let bin_dir = ensure_bin_dir()?;
    if !bin_dir.exists() {
        return Ok(vec![]);
    }

    let entries = match fs::read_dir(&bin_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(err) => {
            return Err(err).context(format!(
                "failed to read installed binaries from {}",
                bin_dir.display()
            ));
        }
    };

    let prefix = format!("{binary}-");
    let mut installed = Vec::new();

    for entry in entries {
        let entry = entry.context("failed to iterate installed binaries")?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => continue,
        };

        #[cfg(windows)]
        let name = name.strip_suffix(".exe").unwrap_or(name);

        let Some(version_str) = name.strip_prefix(&prefix) else {
            continue;
        };

        let Ok(version) = Version::parse(version_str) else {
            continue;
        };

        installed.push(InstalledBinary { version, path });
    }

    installed.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(installed)
}

pub fn find_installed_version(
    binary: &str,
    requirement: &VersionReq,
) -> Result<Option<InstalledBinary>> {
    let installed = list_installed_versions(binary)?;
    Ok(installed
        .into_iter()
        .rev()
        .find(|entry| requirement.matches(&entry.version)))
}

pub fn latest_installed(binary: &str) -> Result<Option<InstalledBinary>> {
    let mut installed = list_installed_versions(binary)?;
    Ok(installed.pop())
}

pub fn ensure_bin_dir() -> Result<PathBuf> {
    let install_dir = get_install_dir()?;
    let bin_dir = install_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create binary directory {}", bin_dir.display()))?;
    Ok(bin_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct InstallDirGuard {
        _guard: MutexGuard<'static, ()>,
    }

    impl InstallDirGuard {
        fn new(path: &Path) -> Self {
            let guard = env_lock().lock().unwrap();
            let path_buf = path.to_path_buf();
            unsafe {
                env::set_var("CARGOX_INSTALL_DIR", &path_buf);
            }
            Self { _guard: guard }
        }
    }

    impl Drop for InstallDirGuard {
        fn drop(&mut self) {
            unsafe {
                env::remove_var("CARGOX_INSTALL_DIR");
            }
        }
    }

    fn with_install_dir<F: FnOnce()>(dir: &Path, f: F) {
        let _guard = InstallDirGuard::new(dir);
        f();
    }

    #[test]
    fn versioned_binary_path_uses_version_suffix() {
        let temp = tempdir().unwrap();
        let version = Version::parse("1.2.3").unwrap();

        with_install_dir(temp.path(), || {
            let path = versioned_binary_path("example", &version).unwrap();
            let filename = path.file_name().unwrap().to_string_lossy();
            #[cfg(windows)]
            assert_eq!(filename, "example-1.2.3.exe");
            #[cfg(not(windows))]
            assert_eq!(filename, "example-1.2.3");
        });
    }

    #[test]
    fn list_installed_versions_returns_sorted_versions() {
        let temp = tempdir().unwrap();

        with_install_dir(temp.path(), || {
            let bin_dir = ensure_bin_dir().unwrap();
            fs::write(bin_dir.join("tool-0.1.0"), "").unwrap();
            fs::write(bin_dir.join("tool-0.2.0"), "").unwrap();

            let versions = list_installed_versions("tool").unwrap();
            assert_eq!(versions.len(), 2);
            assert_eq!(versions[0].version, Version::parse("0.1.0").unwrap());
            assert_eq!(versions[1].version, Version::parse("0.2.0").unwrap());
        });
    }

    #[test]
    fn find_installed_version_respects_requirement() {
        let temp = tempdir().unwrap();

        with_install_dir(temp.path(), || {
            let bin_dir = ensure_bin_dir().unwrap();
            fs::write(bin_dir.join("util-1.0.0"), "").unwrap();
            fs::write(bin_dir.join("util-1.5.0"), "").unwrap();

            let req = VersionReq::parse("^1.0").unwrap();
            let result = find_installed_version("util", &req).unwrap().unwrap();
            assert_eq!(result.version, Version::parse("1.5.0").unwrap());
        });
    }
}
