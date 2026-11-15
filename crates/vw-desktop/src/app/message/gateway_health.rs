//! 客户端网关健康检查。

use crate::app::state::GatewayClientServerDraft;

#[cfg(not(target_arch = "wasm32"))]
fn health_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn health_client() -> Result<reqwest::Client, String> {
    Ok(reqwest::Client::new())
}

pub(crate) use desktop_settings_network::server_health_key;

async fn check_server(client: &reqwest::Client, server: &GatewayClientServerDraft) -> bool {
    let Some(url) = server_health_key(server) else {
        return false;
    };
    #[cfg(not(target_arch = "wasm32"))]
    let direct_client = {
        let host = reqwest::Url::parse(&url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_default();
        if gateway_endpoint::GatewayEndpoint::new(host, server.port).is_loopback() {
            match reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(4))
                .build()
            {
                Ok(client) => Some(client),
                Err(_) => return false,
            }
        } else {
            None
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let client = direct_client.as_ref().unwrap_or(client);
    match client.get(&url).send().await {
        Ok(response) => response.status() == reqwest::StatusCode::OK,
        Err(error) => {
            tracing::debug!(target: "vw_desktop", url = %url, error = ?error, "gateway health request failed");
            false
        }
    }
}

pub(crate) async fn check_servers(servers: Vec<GatewayClientServerDraft>) -> Vec<(String, bool)> {
    let Ok(client) = health_client() else {
        return servers
            .into_iter()
            .filter_map(|server| server_health_key(&server).map(|key| (key, false)))
            .collect();
    };

    let mut results = Vec::with_capacity(servers.len());
    for server in servers {
        let Some(key) = server_health_key(&server) else {
            continue;
        };
        let healthy = check_server(&client, &server).await;
        results.push((key, healthy));
    }
    results
}
