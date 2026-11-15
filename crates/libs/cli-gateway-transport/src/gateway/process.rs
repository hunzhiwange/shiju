use super::*;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use vw_api_transport::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

#[cfg(not(target_arch = "wasm32"))]
pub(super) const GATEWAY_HEALTH_PATH: &str = "/v1/health";
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_TIMEOUT: Duration = Duration::from_secs(8);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(150);

impl GatewayUiRuntime {
    #[cfg(not(target_arch = "wasm32"))]
    /// 在进入 tui 前预检 gateway；若 endpoint 指向本地 loopback，则自动尝试拉起。
    pub fn ensure_local_gateway_ready_blocking(
        &self,
    ) -> Result<GatewayPreflightOutcome, String> {
        if gateway_health_ready(self.endpoint()) {
            return Ok(GatewayPreflightOutcome::Ready);
        }

        if !is_local_loopback_endpoint(self.endpoint()) {
            return Err(annotate_gateway_transport_error(
                "gateway preflight failed: connection refused".to_string(),
                self.endpoint(),
            ));
        }

        let startup_error = start_local_gateway_process(self.endpoint()).err();
        if wait_for_gateway_health(self.endpoint(), GATEWAY_STARTUP_TIMEOUT) {
            return Ok(GatewayPreflightOutcome::Started);
        }

        Err(local_gateway_startup_failure_message(
            self.endpoint(),
            startup_error,
            GATEWAY_STARTUP_TIMEOUT,
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn is_local_loopback_endpoint(endpoint: &GatewayEndpoint) -> bool {
    matches!(
        endpoint
            .normalized_host()
            .trim()
            .trim_matches(|ch| ch == '[' || ch == ']')
            .to_ascii_lowercase()
            .as_str(),
        "127.0.0.1" | "localhost" | "::1"
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn local_gateway_startup_failure_message(
    endpoint: &GatewayEndpoint,
    startup_error: Option<String>,
    timeout: Duration,
) -> String {
    let mut message =
        annotate_gateway_transport_error("gateway preflight failed".to_string(), endpoint);
    if let Some(startup_error) = startup_error {
        message.push_str(" Auto-start failed: ");
        message.push_str(startup_error.as_str());
    } else {
        let _ = write!(
            message,
            " Auto-start did not reach {} with status=ok within {}s.",
            GATEWAY_HEALTH_PATH,
            timeout.as_secs()
        );
    }
    message
}

#[cfg(not(target_arch = "wasm32"))]
fn start_local_gateway_process(endpoint: &GatewayEndpoint) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|err| format!("resolve current executable failed: {err}"))?;

    Command::new(executable)
        .arg("gateway")
        .arg("--host")
        .arg(endpoint.normalized_host())
        .arg("--port")
        .arg(endpoint.port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to start local gateway: {err}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn wait_for_gateway_health(endpoint: &GatewayEndpoint, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if gateway_health_ready(endpoint) {
            return true;
        }
        std::thread::sleep(GATEWAY_STARTUP_POLL_INTERVAL);
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn gateway_health_ready(endpoint: &GatewayEndpoint) -> bool {
    let address = match resolve_gateway_socket_address(endpoint) {
        Some(address) => address,
        None => return false,
    };

    let mut stream = match TcpStream::connect_timeout(&address, GATEWAY_HEALTH_CONNECT_TIMEOUT) {
        Ok(stream) => stream,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(GATEWAY_HEALTH_CONNECT_TIMEOUT));
    let _ = stream.set_write_timeout(Some(GATEWAY_HEALTH_CONNECT_TIMEOUT));

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\n{PLATFORM_ID_HEADER}: {}\r\n{COMPANY_ID_HEADER}: {}\r\nConnection: close\r\n\r\n",
        GATEWAY_HEALTH_PATH,
        endpoint.describe(),
        endpoint.tenant_scope().platform_id().as_ref(),
        endpoint.tenant_scope().company_id().as_ref(),
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }

    let Some(status_line) = response.lines().next() else {
        return false;
    };

    status_line.contains(" 200 ")
        && (response.contains("\"status\":\"ok\"") || response.contains("\"status\": \"ok\""))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_gateway_socket_address(endpoint: &GatewayEndpoint) -> Option<std::net::SocketAddr> {
    let host = endpoint.normalized_host();
    if host.is_empty() || host.chars().any(char::is_whitespace) {
        return None;
    }
    format!("{host}:{}", endpoint.port)
        .to_socket_addrs()
        .ok()?
        .find(|addr| addr.is_ipv4() || addr.is_ipv6())
}
