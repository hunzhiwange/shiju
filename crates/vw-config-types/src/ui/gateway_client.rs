use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientServerConfig {
    #[serde(default = "default_gateway_client_server_id")]
    pub id: String,
    #[serde(default = "default_gateway_client_server_name")]
    pub name: String,
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientSystemSettingsConfig {
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
    #[serde(default)]
    pub active_server_id: String,
    #[serde(default)]
    pub servers: Vec<GatewayClientServerConfig>,
}

fn default_gateway_client_host() -> String {
    "127.0.0.1".to_string()
}

fn default_gateway_client_port() -> u16 {
    config_gateway::DEFAULT_GATEWAY_PORT
}

fn default_platform_id() -> String {
    "10000".to_string()
}

fn default_company_id() -> String {
    "100000000".to_string()
}

fn deserialize_tenant_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("tenant id must not be empty"));
    }
    Ok(value)
}

fn default_gateway_client_server_id() -> String {
    "local".to_string()
}

fn default_gateway_client_server_name() -> String {
    "本地网关".to_string()
}

impl Default for GatewayClientSystemSettingsConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
            active_server_id: default_gateway_client_server_id(),
            servers: vec![GatewayClientServerConfig::default()],
        }
    }
}

impl Default for GatewayClientServerConfig {
    fn default() -> Self {
        Self {
            id: default_gateway_client_server_id(),
            name: default_gateway_client_server_name(),
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
        }
    }
}

impl GatewayClientSystemSettingsConfig {
    pub fn normalized_servers(&self) -> Vec<GatewayClientServerConfig> {
        if self.servers.is_empty() {
            return vec![GatewayClientServerConfig {
                id: default_gateway_client_server_id(),
                name: default_gateway_client_server_name(),
                host: self.host.clone(),
                port: self.port,
                platform_id: self.platform_id.clone(),
                company_id: self.company_id.clone(),
                bearer_token: self.bearer_token.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
                skey: self.skey.clone(),
            }];
        }

        self.servers
            .iter()
            .enumerate()
            .map(|(index, server)| {
                let mut server = server.clone();
                if server.id.trim().is_empty() {
                    server.id = if index == 0 {
                        default_gateway_client_server_id()
                    } else {
                        format!("gateway-{}", index + 1)
                    };
                }
                if server.name.trim().is_empty() {
                    server.name = if index == 0 {
                        default_gateway_client_server_name()
                    } else {
                        format!("网关 {}", index + 1)
                    };
                }
                server
            })
            .collect()
    }

    pub fn active_server(&self) -> GatewayClientServerConfig {
        let servers = self.normalized_servers();
        servers
            .iter()
            .find(|server| server.id == self.active_server_id)
            .cloned()
            .or_else(|| servers.first().cloned())
            .unwrap_or_default()
    }

    pub fn set_servers(
        &mut self,
        servers: Vec<GatewayClientServerConfig>,
        active_server_id: String,
    ) {
        let servers =
            if servers.is_empty() { vec![GatewayClientServerConfig::default()] } else { servers };
        let active = servers
            .iter()
            .find(|server| server.id == active_server_id)
            .or_else(|| servers.first())
            .cloned()
            .unwrap_or_default();

        self.active_server_id = active.id.clone();
        self.host = active.host.clone();
        self.port = active.port;
        self.platform_id = active.platform_id.clone();
        self.company_id = active.company_id.clone();
        self.bearer_token = active.bearer_token.clone();
        self.username = active.username.clone();
        self.password = active.password.clone();
        self.skey = active.skey.clone();
        self.servers = servers;
    }
}

#[cfg(test)]
#[path = "gateway_client_tests.rs"]
mod tests;
