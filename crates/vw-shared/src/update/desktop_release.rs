//! Desktop installer snapshots. No binary replacement and no implicit source fallback.
use super::{DEFAULT_RELEASE_API, UpdateProgress, current_version};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SHIJU_ROOT: &str = "https://www.shiju.cloud/agents/shiju-agent";
const SHIJU_MANIFEST: &str = "https://www.shiju.cloud/agents/shiju-agent/latest/manifest.json";
const GITHUB_ROOT: &str = "https://github.com/hunzhiwange/shiju/releases";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    #[default]
    Shiju,
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallerKind {
    Rpm,
    Deb,
    Msi,
    Dmg,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopInstaller {
    pub name: String,
    pub target: String,
    pub kind: InstallerKind,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// The exact release checked by the UI. Downloading never fetches latest again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRelease {
    pub source: UpdateSource,
    pub version: String,
    pub assets: Vec<DesktopInstaller>,
}

impl DesktopRelease {
    /// Compare stable versions numerically, rejecting malformed versions on either side.
    pub fn is_newer_than(&self, installed: &str) -> Result<bool> {
        Ok(version_tuple(&self.version)? > version_tuple(installed)?)
    }

    pub fn installer_for_target(
        &self,
        target: &str,
        kind: InstallerKind,
    ) -> Result<&DesktopInstaller> {
        self.validate()?;
        self.assets
            .iter()
            .find(|a| a.target == target && a.kind == kind)
            .context("Unsupported desktop target")
    }

    fn validate(&self) -> Result<()> {
        let version = canonical_version(&self.version)?;
        let specs = installer_specs(version);
        if self.assets.len() != specs.len() {
            bail!("Expected exactly eight desktop installers (including Linux DEB and RPM)");
        }
        for (target, kind, name) in specs {
            let matches: Vec<_> =
                self.assets.iter().filter(|a| a.target == target && a.kind == kind).collect();
            if matches.len() != 1 {
                bail!("Missing or duplicate installer target: {target}");
            }
            let asset = matches[0];
            if asset.name != name || asset.kind != kind {
                bail!("Installer name/kind does not match target: {target}");
            }
            if asset.size == 0
                || asset.sha256.len() != 64
                || !asset.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                bail!("Invalid installer size or SHA-256: {name}");
            }
            let expected_url = match self.source {
                UpdateSource::Shiju => format!("{SHIJU_ROOT}/releases/v{version}/{name}"),
                UpdateSource::Github => format!("{GITHUB_ROOT}/download/v{version}/{name}"),
            };
            if asset.url != expected_url {
                bail!("Unexpected installer URL: {name}");
            }
        }
        Ok(())
    }
}

fn canonical_version(value: &str) -> Result<&str> {
    version_tuple(value)?;
    Ok(value.strip_prefix('v').unwrap_or(value))
}

fn version_tuple(value: &str) -> Result<(u64, u64, u64)> {
    let value = value.strip_prefix('v').unwrap_or(value);
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        bail!("Expected a stable X.Y.Z version");
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

fn installer_specs(version: &str) -> Vec<(&'static str, InstallerKind, String)> {
    vec![
        ("aarch64-unknown-linux-gnu", InstallerKind::Deb, format!("shiju_{version}_arm64.deb")),
        ("x86_64-unknown-linux-gnu", InstallerKind::Deb, format!("shiju_{version}_amd64.deb")),
        ("aarch64-unknown-linux-gnu", InstallerKind::Rpm, format!("shiju-{version}-1.aarch64.rpm")),
        ("x86_64-unknown-linux-gnu", InstallerKind::Rpm, format!("shiju-{version}-1.x86_64.rpm")),
        (
            "aarch64-pc-windows-msvc",
            InstallerKind::Msi,
            "ShiJu-aarch64-pc-windows-msvc-dist.msi".into(),
        ),
        (
            "x86_64-pc-windows-msvc",
            InstallerKind::Msi,
            "ShiJu-x86_64-pc-windows-msvc-dist.msi".into(),
        ),
        (
            "aarch64-apple-darwin",
            InstallerKind::Dmg,
            "ShiJu-macos-aarch64-apple-darwin-dist.dmg".into(),
        ),
        (
            "x86_64-apple-darwin",
            InstallerKind::Dmg,
            "ShiJu-macos-x86_64-apple-darwin-dist.dmg".into(),
        ),
    ]
}

#[derive(Deserialize)]
struct ShijuManifest {
    schema_version: u64,
    version: String,
    published_at: String,
    release_url: String,
    assets: Vec<ShijuAsset>,
}

#[derive(Deserialize)]
struct ShijuAsset {
    #[serde(flatten)]
    installer: DesktopInstaller,
    github_url: String,
}

#[derive(Deserialize)]
struct GithubManifest {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<serde_json::Value>,
}

fn parse_desktop_release(
    source: UpdateSource,
    payload: serde_json::Value,
) -> Result<DesktopRelease> {
    let release = match source {
        UpdateSource::Shiju => {
            let raw: ShijuManifest = serde_json::from_value(payload)?;
            let version = canonical_version(&raw.version)?.to_owned();
            if raw.schema_version != 1
                || raw.published_at.trim().is_empty()
                || raw.release_url != format!("{GITHUB_ROOT}/tag/v{version}")
            {
                bail!("Unsupported or invalid desktop manifest");
            }
            for asset in &raw.assets {
                if asset.github_url
                    != format!("{GITHUB_ROOT}/download/v{version}/{}", asset.installer.name)
                {
                    bail!("Unexpected GitHub installer URL");
                }
            }
            DesktopRelease {
                source,
                version,
                assets: raw.assets.into_iter().map(|a| a.installer).collect(),
            }
        }
        UpdateSource::Github => {
            let raw: GithubManifest = serde_json::from_value(payload)?;
            let version = canonical_version(&raw.tag_name)?.to_owned();
            if raw.draft || raw.prerelease || raw.tag_name != format!("v{version}") {
                bail!("Expected a published stable GitHub release");
            }
            let mut assets = Vec::new();
            for (target, kind, name) in installer_specs(&version) {
                let matches: Vec<_> =
                    raw.assets.iter().filter(|a| a["name"].as_str() == Some(&name)).collect();
                if matches.len() != 1 {
                    bail!("Missing or duplicate GitHub installer: {name}");
                }
                let asset = matches[0];
                assets.push(DesktopInstaller {
                    name,
                    target: target.into(),
                    kind,
                    url: asset["browser_download_url"]
                        .as_str()
                        .context("Missing download URL")?
                        .into(),
                    size: asset["size"].as_u64().context("Missing installer size")?,
                    sha256: asset["digest"]
                        .as_str()
                        .and_then(|s| s.strip_prefix("sha256:"))
                        .context("Missing SHA-256 digest")?
                        .into(),
                });
            }
            DesktopRelease { source, version, assets }
        }
    };
    release.validate()?;
    Ok(release)
}

fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    // No authorization headers: neither metadata nor redirected downloads receive credentials.
    Ok(reqwest::Client::builder()
        .user_agent(format!("vibewindow/{}", current_version()))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many installer redirects")
            } else if attempt.url().scheme() != "https"
                || !attempt.url().username().is_empty()
                || attempt.url().password().is_some()
            {
                attempt.error("Installer redirects must use HTTPS without credentials")
            } else {
                attempt.follow()
            }
        }))
        .build()?)
}

/// Fetch the explicitly selected source; ignores CLI update environment overrides.
pub async fn fetch_desktop_release(source: UpdateSource) -> Result<DesktopRelease> {
    let endpoint = match source {
        UpdateSource::Shiju => SHIJU_MANIFEST,
        UpdateSource::Github => DEFAULT_RELEASE_API,
    };
    fetch_desktop_release_at(source, endpoint, &http_client(Duration::from_secs(30))?).await
}

async fn fetch_desktop_release_at(
    source: UpdateSource,
    endpoint: &str,
    client: &reqwest::Client,
) -> Result<DesktopRelease> {
    let mut response = client.get(endpoint).send().await?.error_for_status()?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > 1024 * 1024 {
            bail!("Desktop manifest exceeds 1 MiB");
        }
        body.extend_from_slice(&chunk);
    }
    parse_desktop_release(source, serde_json::from_slice(&body)?)
}

/// Download into an existing destination directory and atomically publish the verified name.
/// Existing files are never overwritten. Failure/cancellation drops the temporary file.
pub async fn download_desktop_installer<F>(
    release: &DesktopRelease,
    target: &str,
    kind: InstallerKind,
    destination: &Path,
    on_progress: F,
) -> Result<PathBuf>
where
    F: FnMut(UpdateProgress) + Send,
{
    let asset = release.installer_for_target(target, kind)?;
    download_installer_at(
        asset,
        &asset.url,
        destination,
        on_progress,
        &http_client(Duration::from_secs(30 * 60))?,
    )
    .await
}

async fn download_installer_at<F>(
    asset: &DesktopInstaller,
    url: &str,
    destination: &Path,
    mut on_progress: F,
    client: &reqwest::Client,
) -> Result<PathBuf>
where
    F: FnMut(UpdateProgress) + Send,
{
    let final_path = destination.join(&asset.name);
    if final_path.symlink_metadata().is_ok() {
        bail!("Installer destination already exists");
    }
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response.content_length().is_some_and(|length| length != asset.size) {
        bail!("Installer Content-Length differs from manifest");
    }
    let mut temporary = tempfile::NamedTempFile::new_in(destination)?;
    let mut digest = Sha256::new();
    let start = Instant::now();
    let mut last_tick = start;
    let mut downloaded_bytes = 0_u64;
    on_progress(UpdateProgress {
        asset_name: asset.name.clone(),
        download_url: asset.url.clone(),
        downloaded_bytes,
        total_bytes: Some(asset.size),
        bytes_per_second: 0.0,
        average_bytes_per_second: 0.0,
    });
    while let Some(chunk) = response.chunk().await? {
        downloaded_bytes =
            downloaded_bytes.checked_add(chunk.len() as u64).context("Installer size overflow")?;
        if downloaded_bytes > asset.size {
            bail!("Installer exceeds manifest size");
        }
        temporary.write_all(&chunk)?;
        digest.update(&chunk);
        let now = Instant::now();
        on_progress(UpdateProgress {
            asset_name: asset.name.clone(),
            download_url: asset.url.clone(),
            downloaded_bytes,
            total_bytes: Some(asset.size),
            bytes_per_second: chunk.len() as f64
                / now.duration_since(last_tick).as_secs_f64().max(f64::EPSILON),
            average_bytes_per_second: downloaded_bytes as f64
                / now.duration_since(start).as_secs_f64().max(f64::EPSILON),
        });
        last_tick = now;
    }
    if downloaded_bytes != asset.size || hex::encode(digest.finalize()) != asset.sha256 {
        bail!("Installer size or SHA-256 verification failed");
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(&final_path)
        .map_err(|error| error.error)
        .context("Failed to publish verified installer")?;
    Ok(final_path)
}

#[cfg(test)]
#[path = "desktop_release_tests.rs"]
mod desktop_release_tests;

#[path = "linux_installer.rs"]
mod linux_installer;
pub use linux_installer::linux_installer_kind;
