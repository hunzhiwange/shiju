//! Loopback-only Hermes dashboard probe on the authorized executor.
//! Callers authorize the project/action and dispatch before entering this module.
use std::time::Duration;

const DEFAULT_PORT: u16 = 9119;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardStatus {
    Online { url: String },
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardError {
    UnauthorizedHost,
    InvalidPage,
    Client,
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnauthorizedHost => "Hermes executor is not the authorized host",
            Self::InvalidPage => "Hermes dashboard requires a relative page path",
            Self::Client => "failed to build Hermes dashboard probe client",
        })
    }
}
impl std::error::Error for DashboardError {}

/// Reads HERMES_WEB_PORT only on the executing host. No browser is opened here;
/// remote loopback URLs must not be opened on the desktop without host support.
pub async fn probe_dashboard(
    authorized_host: &str,
    executor_host: &str,
    page: Option<&str>,
) -> Result<DashboardStatus, DashboardError> {
    if authorized_host.is_empty() || authorized_host != executor_host {
        return Err(DashboardError::UnauthorizedHost);
    }
    probe_port(port_from_env(std::env::var("HERMES_WEB_PORT").ok().as_deref()), page).await
}

fn port_from_env(value: Option<&str>) -> u16 {
    value.and_then(|raw| raw.trim().parse().ok()).unwrap_or(DEFAULT_PORT)
}

fn page_url(port: u16, page: Option<&str>) -> Result<String, DashboardError> {
    let page = page.unwrap_or("");
    // A page is a path, not a URL/query/fragment. Reject escaped delimiters too,
    // so a browser or application cannot reinterpret them as another authority.
    if page.starts_with("//")
        || page.contains(['\\', ':', '?', '#', '%'])
        || page.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(DashboardError::InvalidPage);
    }
    let path = page.strip_prefix('/').unwrap_or(page);
    if path.split('/').any(|part| matches!(part, "." | "..")) {
        return Err(DashboardError::InvalidPage);
    }
    let base = format!("http://127.0.0.1:{port}");
    let url = reqwest::Url::parse(&format!("{base}/{path}"))
        .map_err(|_| DashboardError::InvalidPage)?;
    Ok(url.to_string())
}

async fn probe_port(port: u16, page: Option<&str>) -> Result<DashboardStatus, DashboardError> {
    let url = page_url(port, page)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(1200))
        .build()
        .map_err(|_| DashboardError::Client)?;
    // Any HTTP response, including auth failures, server failures and redirects,
    // proves reachability. Do not follow redirects outside the loopback boundary.
    match client.get(format!("http://127.0.0.1:{port}/api/status")).send().await {
        Ok(_) => Ok(DashboardStatus::Online { url }),
        Err(_) => Ok(DashboardStatus::Offline),
    }
}

#[cfg(test)]
#[path = "hermes_dashboard_tests.rs"]
mod tests;
