use anyhow::{Result, anyhow};

#[derive(Debug)]
pub struct Target {
    pub crate_name: String,
    pub version: Option<String>,
    pub binary: String,
}

impl Target {
    pub fn descriptor(&self) -> String {
        match &self.version {
            Some(version) => format!("{}@{}", self.crate_name, version),
            None => self.crate_name.clone(),
        }
    }

    pub fn install_spec(&self) -> String {
        match &self.version {
            Some(version) => format!("{}@{}", self.crate_name, version),
            None => self.crate_name.clone(),
        }
    }
}

pub fn parse_spec(spec: &str) -> Result<(String, Option<String>)> {
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
        return Ok((first.to_owned(), None));
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

    Ok((first.to_owned(), Some(version.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_spec_without_version() {
        let (name, version) = parse_spec("ripgrep").unwrap();
        assert_eq!(name, "ripgrep");
        assert_eq!(version, None);
    }

    #[test]
    fn split_spec_with_version() {
        let (name, version) = parse_spec("ripgrep@13.0.0").unwrap();
        assert_eq!(name, "ripgrep");
        assert_eq!(version.as_deref(), Some("13.0.0"));
    }

    #[test]
    fn split_spec_rejects_empty() {
        assert!(parse_spec("").is_err());
        assert!(parse_spec("@1.0.0").is_err());
        assert!(parse_spec("foo@").is_err());
        assert!(parse_spec("foo@bar@baz").is_err());
    }
}
