//! Credential-free registry downloads. Archives stay separate from installed state.
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};
use url::Url;

const MAX_METADATA: usize = 2 * 1024 * 1024;
const MAX_ARCHIVE: usize = 50 * 1024 * 1024;

/// Verified bytes and resolved provenance; no package code has been executed.
#[derive(Debug)]
pub struct NpmPluginArchive {
    pub package: String,
    pub version: String,
    pub registry: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

/// Resolve an exact version or registry tag and verify its SHA-512/SHA-256 SRI.
/// Redirects, private destinations and ambient credentials/proxies are rejected.
pub async fn fetch_npm_plugin(
    package: &str,
    version: Option<&str>,
    registry: Option<&str>,
) -> Result<NpmPluginArchive> {
    fetch(package, version, registry.unwrap_or("https://registry.npmjs.org/"), false).await
}

#[derive(Deserialize)]
struct Metadata {
    name: String,
    version: String,
    dist: Distribution,
}
#[derive(Deserialize)]
struct Distribution {
    tarball: String,
    integrity: String,
}
fn valid_package(package: &str) -> bool {
    let parts: Vec<_> = package.strip_prefix('@').unwrap_or(package).split('/').collect();
    parts.len() == if package.starts_with('@') { 2 } else { 1 }
        && parts.iter().all(|p| {
            !p.is_empty()
                && !p.starts_with(['.', '_'])
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
fn valid_selector(selector: &str) -> bool {
    !selector.is_empty()
        && selector != "."
        && selector != ".."
        && selector.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.+".contains(&b))
}
fn exact_version(version: &str) -> bool {
    if !valid_selector(version) {
        return false;
    }
    let (version, build) = version.split_once('+').map_or((version, None), |(v, b)| (v, Some(b)));
    let (core, prerelease) = version.split_once('-').map_or((version, None), |(v, p)| (v, Some(p)));
    let identifiers = |raw: &str, prerelease: bool| {
        raw.split('.').all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !(prerelease
                    && p.len() > 1
                    && p.starts_with('0')
                    && p.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && (p.len() == 1 || !p.starts_with('0'))
                && p.bytes().all(|b| b.is_ascii_digit())
        })
        && build.is_none_or(|b| identifiers(b, false))
        && prerelease.is_none_or(|p| identifiers(p, true))
}
fn endpoint(raw: &str, fixture: bool) -> Result<Url> {
    let url = Url::parse(raw).context("invalid npm endpoint")?;
    // This escape hatch is only compiled into unit tests and only permits loopback.
    if cfg!(test)
        && fixture
        && url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
    {
        return Ok(url);
    }
    super::mcp_oauth_discovery::endpoint(raw).context("npm requires public HTTPS endpoints")
}
async fn download(url: Url, limit: usize, fixture: bool) -> Result<Vec<u8>> {
    endpoint(url.as_str(), fixture)?;
    let client =
        if cfg!(test) && fixture && url.scheme() == "http" && url.host_str() == Some("127.0.0.1") {
            reqwest_012::Client::builder()
                .no_proxy()
                .redirect(reqwest_012::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()?
        } else {
            super::mcp_oauth_discovery::public_client(&url).await?
        };
    let mut response = client.get(url).send().await.context("npm request failed")?;
    if !response.status().is_success() {
        bail!("npm HTTP status {}", response.status());
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        bail!("npm response exceeds size limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > limit - bytes.len() {
            bail!("npm response exceeds size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
fn verify(bytes: &[u8], integrity: &str) -> Result<()> {
    // SRI's strongest recognized algorithm wins, preventing weaker-hash downgrade.
    for algorithm in ["sha512", "sha256"] {
        let candidates: Vec<_> = integrity
            .split_whitespace()
            .filter_map(|part| part.strip_prefix(&format!("{algorithm}-")))
            .collect();
        if !candidates.is_empty() {
            let actual = if algorithm == "sha512" {
                Sha512::digest(bytes).to_vec()
            } else {
                Sha256::digest(bytes).to_vec()
            };
            if candidates
                .iter()
                .any(|s| STANDARD.decode(s).is_ok_and(|expected| expected == actual))
            {
                return Ok(());
            }
            bail!("npm integrity mismatch");
        }
    }
    bail!("npm requires SHA-512 or SHA-256 integrity");
}
pub(super) async fn fetch(
    package: &str,
    version: Option<&str>,
    registry: &str,
    fixture: bool,
) -> Result<NpmPluginArchive> {
    if !valid_package(package) || !valid_selector(version.unwrap_or("latest")) {
        bail!("invalid npm package or version selector");
    }
    let mut root = endpoint(registry, fixture)?;
    if !root.path().ends_with('/') {
        root.set_path(&format!("{}/", root.path()));
    }
    let mut metadata_url = root.clone();
    metadata_url
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("invalid registry base"))?
        .pop_if_empty()
        .push(package)
        .push(version.unwrap_or("latest"));
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let metadata: Metadata =
            serde_json::from_slice(&download(metadata_url, MAX_METADATA, fixture).await?)
                .context("invalid npm metadata")?;
        if metadata.name != package || !exact_version(&metadata.version) {
            bail!("npm metadata name or resolved version mismatch");
        }
        if let Some(requested) = version {
            if exact_version(requested) && requested != metadata.version {
                bail!("npm resolved version mismatch");
            }
        }
        let tarball = endpoint(&metadata.dist.tarball, fixture)?;
        let bytes = download(tarball, MAX_ARCHIVE, fixture).await?;
        verify(&bytes, &metadata.dist.integrity)?;
        Ok(NpmPluginArchive {
            package: package.to_owned(),
            version: metadata.version,
            registry: root.to_string(),
            sha256: hex::encode(Sha256::digest(&bytes)),
            bytes,
        })
    })
    .await
    .context("npm download timed out")?
}
