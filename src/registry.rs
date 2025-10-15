use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct VersionsResponse {
    versions: Vec<CrateVersion>,
}

#[derive(Deserialize)]
struct CrateVersion {
    num: String,
    yanked: bool,
}

pub fn fetch_highest_matching_version(
    crate_name: &str,
    requirement: Option<&VersionReq>,
) -> Result<Version> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")?;

    let mut headers = HeaderMap::new();
    let user_agent = format!("cargox/{}", env!("CARGO_PKG_VERSION"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&user_agent).context("invalid user agent")?,
    );

    let url = format!("https://crates.io/api/v1/crates/{crate_name}");

    let response = client
        .get(url)
        .headers(headers)
        .send()
        .context("failed to contact crates.io")?
        .error_for_status()
        .context("crates.io returned an error status")?;

    let payload: VersionsResponse = response
        .json()
        .context("failed to parse crates.io response")?;

    let mut versions: Vec<Version> = payload
        .versions
        .into_iter()
        .filter(|v| !v.yanked)
        .filter_map(|entry| Version::parse(&entry.num).ok())
        .collect();

    if versions.is_empty() {
        return Err(anyhow!("no published versions found for {crate_name}"));
    }

    versions.sort();

    if let Some(req) = requirement {
        if let Some(version) = versions.iter().rev().find(|v| req.matches(v)) {
            return Ok(version.clone());
        }
        return Err(anyhow!(
            "no published versions of {crate_name} satisfy requirement {req}"
        ));
    }

    versions
        .into_iter()
        .last()
        .ok_or_else(|| anyhow!("no published versions found for {crate_name}"))
}

pub fn fetch_latest_version(crate_name: &str) -> Result<Version> {
    fetch_highest_matching_version(crate_name, None)
}
