use anyhow::{Result, anyhow};
use semver::VersionReq;

#[derive(Debug)]
pub struct Target {
    pub crate_name: String,
    pub version: VersionSpec,
    pub binary: String,
}

#[derive(Debug, Clone)]
pub enum VersionSpec {
    Unspecified,
    Latest,
    Requirement(VersionReq),
}

pub fn parse_spec(spec: &str) -> Result<(String, VersionSpec)> {
    if spec.trim().is_empty() {
        return Err(anyhow!("crate spec cannot be empty"));
    }

    let mut parts = spec.split('@');
    let first = parts
        .next()
        .expect("split always yields at least one element")
        .trim();

    if first.is_empty() {
        return Err(anyhow!("crate name cannot be empty"));
    }

    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return Ok((first.to_owned(), VersionSpec::Unspecified));
    }

    if rest.len() > 1 {
        return Err(anyhow!(
            "invalid crate spec `{spec}`: expected at most one `@version` suffix"
        ));
    }

    let version = rest[0].trim();
    if version.is_empty() {
        return Err(anyhow!(
            "invalid crate spec `{spec}`: version cannot be empty after `@`"
        ));
    }

    if version.eq_ignore_ascii_case("latest") {
        return Ok((first.to_owned(), VersionSpec::Latest));
    }

    let requirement = VersionReq::parse(version).map_err(|err| {
        anyhow!(
            "invalid crate spec `{spec}`: failed to parse version requirement `{version}`: {err}"
        )
    })?;

    Ok((first.to_owned(), VersionSpec::Requirement(requirement)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_spec_without_version() {
        let (name, version) = parse_spec("ripgrep").unwrap();
        assert_eq!(name, "ripgrep");
        assert!(matches!(version, VersionSpec::Unspecified));
    }

    #[test]
    fn split_spec_with_version_requirement() {
        let (name, version) = parse_spec("ripgrep@13.0.0").unwrap();
        assert_eq!(name, "ripgrep");
        match version {
            VersionSpec::Requirement(req) => {
                assert_eq!(req.to_string(), "^13.0.0");
            }
            other => panic!("unexpected version spec: {other:?}"),
        }
    }

    #[test]
    fn split_spec_rejects_empty() {
        assert!(parse_spec("").is_err());
        assert!(parse_spec("@1.0.0").is_err());
        assert!(parse_spec("foo@").is_err());
        assert!(parse_spec("foo@bar@baz").is_err());
    }

    #[test]
    fn split_spec_parses_latest() {
        let (name, version) = parse_spec("ripgrep@latest").unwrap();
        assert_eq!(name, "ripgrep");
        assert!(matches!(version, VersionSpec::Latest));
    }
}
