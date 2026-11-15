//! Gateway-first TUI runtime 骨架。
//!
//! 本模块解决 Phase 1 的两个直接目标：
//! 1. 让 vw-cli 可以不经过其他 UI 子系统，直接构造 `GatewayClient`。
//! 2. 建立 `GatewayUiRuntime` 的最小宿主，集中保存后续新 TUI 所需的
//!    gateway client、目录上下文、session scope 与 session title。
//! 3. 在 runtime 边界补上 CLI 专用 stream adapter，把 gateway 原始事件
//!    规整为内部 `UiRuntimeEvent`，不再让新 TUI 直接解析 `Other(JSON)`。
//!
//! 当前实现刻意保持克制：
//! - 不在这里提前引入 reducer、view model 或渲染逻辑。
//! - 不提前定义“为以后准备”的胖接口。
//! - 只保留后续 slice 一定会复用的稳定入口。

use super::stream_adapter::terminal_from_error_message;
use cli_interaction_model::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::stream_adapter::adapt_gateway_stream_event;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use config_aggregate::AcpAgentConfig;
use vw_api_agent_scheduler::AgentSchedulerScopeDto;
use vw_api_transport::id::ProjectScope;
use {
    vw_api_transport::id::SessionId,
    vw_api_transport::id::TenantScope,
};
use vw_api_project::{ProjectDto, ResolveProjectRequest};
use vw_gateway_client::{
    GatewayAuth, GatewayChatStreamRequest, GatewayClient, GatewayEndpoint,
};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayPreflightOutcome {
    Ready,
    Started,
}

#[cfg(not(target_arch = "wasm32"))]
impl GatewayPreflightOutcome {
    pub fn started_gateway(self) -> bool {
        matches!(self, Self::Started)
    }
}

/// CLI 侧用于构造 `GatewayClient` 的最小引导配置。
///
/// 该结构只覆盖客户端访问网关所需的字段：主机、端口与认证信息。
/// 字段来源为桌面端维护的 `gateway-client-bootstrap.json` 启动缓存，
/// 不把它当作完整系统配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayClientBootstrapConfig {
    /// 目标网关主机名或 IP。
    pub host: String,
    /// 目标网关端口。
    pub port: u16,
    /// 可选 skey，会作为 Bearer token 发送。
    pub skey: Option<String>,
    /// 请求所属的平台租户 ID。
    pub platform_id: String,
    /// 请求所属的公司租户 ID。
    pub company_id: String,
}

impl Default for GatewayClientBootstrapConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: config_gateway::DEFAULT_GATEWAY_PORT,
            skey: None,
            platform_id: "10000".to_string(),
            company_id: "100000000".to_string(),
        }
    }
}

impl GatewayClientBootstrapConfig {
    /// 从 CLI 配置目录的启动缓存读取 gateway client 引导信息。
    ///
    /// 读取失败时会回退到默认值，而不是让 legacy CLI 因配置缺失直接中断。
    /// 这样可以保证 S1-1 先把依赖边界与 runtime 骨架接好，后续再继续补齐真实切换路径。
    pub fn load(config_path: &Path) -> Self {
        let mut bootstrap = Self::default();

        let Some(config_dir) = config_path.parent() else {
            return bootstrap;
        };
        let bootstrap_path = config_dir.join("gateway-client-bootstrap.json");
        let contents = match std::fs::read_to_string(&bootstrap_path) {
            Ok(contents) => contents,
            Err(err) => {
                tracing::warn!(
                    path = %bootstrap_path.display(),
                    error = %err,
                    "failed to read cli gateway bootstrap config, using defaults"
                );
                return bootstrap;
            }
        };

        let root = match serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(root) => root,
            Err(err) => {
                tracing::warn!(
                    path = %bootstrap_path.display(),
                    error = %err,
                    "failed to parse cli gateway bootstrap config, using defaults"
                );
                return bootstrap;
            }
        };

        bootstrap.apply_json_root(&root);
        bootstrap
    }

    /// 将引导配置转换为 `GatewayEndpoint`。
    pub fn endpoint(&self) -> GatewayEndpoint {
        let tenant_scope = TenantScope::new(self.platform_id.clone(), self.company_id.clone())
            .expect("CLI gateway bootstrap tenant IDs must not be empty");
        let mut endpoint =
            GatewayEndpoint::new(self.host.clone(), self.port).with_tenant_scope(tenant_scope);
        if let Some(auth) = self.auth() {
            endpoint = endpoint.with_auth(auth);
        }
        endpoint
    }

    /// 将顶层 JSON 根对象中的 `gateway_client` 块映射到当前结构。
    fn apply_json_root(&mut self, root: &serde_json::Value) {
        let gateway = root.get("gateway_client").unwrap_or(root).as_object();
        let Some(gateway) = gateway else {
            return;
        };

        if let Some(host) = gateway.get("host").and_then(serde_json::Value::as_str) {
            let host = host.trim();
            if !host.is_empty() {
                self.host = host.to_string();
            }
        }

        if let Some(port) = gateway.get("port").and_then(serde_json::Value::as_u64)
            && let Ok(port) = u16::try_from(port)
            && port != 0
        {
            self.port = port;
        }

        self.skey = gateway_optional_string(gateway.get("skey"))
            .or_else(|| gateway_optional_string(gateway.get("bearer_token")));
        if let Some(platform_id) = gateway_optional_string(gateway.get("platform_id")) {
            self.platform_id = platform_id;
        }
        if let Some(company_id) = gateway_optional_string(gateway.get("company_id")) {
            self.company_id = company_id;
        }
    }

    /// 仅当配置中存在至少一种认证信息时才生成认证对象。
    fn auth(&self) -> Option<GatewayAuth> {
        let auth = GatewayAuth { skey: self.skey.clone() };

        if auth.skey.is_none() { None } else { Some(auth) }
    }
}

/// 新 TUI 运行时的会话级上下文种子。
///
/// 这里先只保存后续 runtime 一定需要的三项数据：
/// - 当前会话 ID
/// - 请求目录
/// - 会话 scope
/// - 会话标题
///
/// 后续 slice 会继续围绕该结构补齐 session snapshot、restore 和状态线所需信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewaySessionSeed {
    id: Option<String>,
    directory: PathBuf,
    scope: Option<String>,
    title: Option<String>,
}

impl GatewaySessionSeed {
    /// 使用请求目录创建新的会话上下文。
    pub fn new(directory: PathBuf) -> Self {
        Self { id: None, directory, scope: None, title: None }
    }

    /// 返回当前 runtime 绑定的会话 ID（如果已设置）。
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// 返回当前 runtime 绑定的请求目录。
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// 返回会话 scope（如果已设置）。
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// 返回会话标题（如果已设置）。
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// 用新的会话 ID 覆盖当前种子。
    pub fn with_id(mut self, id: Option<String>) -> Self {
        self.id = normalize_optional_string(id);
        self
    }

    /// 用新的 scope 覆盖当前种子。
    pub fn with_scope(mut self, scope: Option<String>) -> Self {
        self.scope = normalize_optional_string(scope);
        self
    }

    /// 用新的标题覆盖当前种子。
    pub(crate) fn with_title(mut self, title: Option<String>) -> Self {
        self.title = normalize_optional_string(title);
        self
    }
}

/// 新 TUI 的 gateway-first runtime 宿主。
///
/// 当前 slice 中，该结构只负责两件事：
/// 1. 持有统一的 `GatewayClient`，作为后续 stream/session/question/todo 请求入口。
/// 2. 持有会话种子信息，为后续 session_ui 与状态层接线提供稳定上下文。
#[derive(Debug, Clone)]
pub struct GatewayUiRuntime {
    client: GatewayClient,
    session: GatewaySessionSeed,
}

impl GatewayUiRuntime {
    /// 基于外部已创建好的 client 与会话种子构造 runtime。
    pub fn new(client: GatewayClient, session: GatewaySessionSeed) -> Self {
        Self { client, session }
    }

    /// 基于已解析的网关地址与会话种子创建 runtime。
    pub fn from_endpoint(
        endpoint: GatewayEndpoint,
        session: GatewaySessionSeed,
    ) -> Result<Self, String> {
        let client = GatewayClient::new(endpoint)?;
        Ok(Self::new(client, session))
    }

    /// 返回统一的 gateway client 引用。
    pub fn client(&self) -> &GatewayClient {
        &self.client
    }

    /// 返回当前 runtime 绑定的 gateway endpoint。
    pub fn endpoint(&self) -> &GatewayEndpoint {
        self.client.endpoint()
    }

    /// 返回当前 runtime 的会话种子。
    pub fn session(&self) -> &GatewaySessionSeed {
        &self.session
    }

    /// 返回当前请求目录。
    pub fn directory(&self) -> &Path {
        self.session.directory()
    }

    /// 返回当前会话 scope。
    pub fn scope(&self) -> Option<&str> {
        self.session.scope()
    }

    /// 返回当前会话标题。
    pub fn title(&self) -> Option<&str> {
        self.session.title()
    }

    /// 返回当前会话 ID。
    pub fn session_id(&self) -> Option<&str> {
        self.session.id()
    }

    /// 用当前 UI 已确认的 session 元信息覆盖 runtime seed，保持后续默认回退一致。
    pub fn bind_session_seed(
        &mut self,
        session_id: Option<String>,
        scope: Option<String>,
        title: Option<String>,
    ) {
        self.session = self.session.clone().with_id(session_id).with_scope(scope).with_title(title);
    }

    /// 绑定本地可规范化目录，并清空旧目录的会话身份与作用域。
    pub fn rebind_validated_directory(&mut self, path: &Path) -> Result<PathBuf, String> {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("cannot resolve directory {}: {error}", path.display()))?;
        if !canonical.is_dir() {
            return Err(format!("target is not a directory: {}", canonical.display()));
        }
        self.session = GatewaySessionSeed::new(canonical.clone());
        Ok(canonical)
    }

    pub(crate) async fn resolve_project_directory(
        &self,
        directory: &Path,
        create_if_missing: bool,
    ) -> Result<ProjectDto, String> {
        let directory =
            directory.to_str().ok_or_else(|| "project directory is not valid UTF-8".to_string())?;
        let request = ResolveProjectRequest { directory: directory.to_string(), create_if_missing };
        gateway_project::GatewayProjectClient::new(self.client.transport().clone()).project_resolve(self.endpoint().tenant_scope(), &request)
            .await
            .map(|response| response.project)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn resolve_project_directory_blocking(
        &self,
        directory: &Path,
        create_if_missing: bool,
    ) -> Result<ProjectDto, String> {
        block_on_gateway(self.resolve_project_directory(directory, create_if_missing))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn agent_scheduler_scope_blocking(&self) -> Result<AgentSchedulerScopeDto, String> {
        let project = self.resolve_project_directory_blocking(self.directory(), false)?;
        let project_scope = ProjectScope::new(self.endpoint().tenant_scope().clone(), project.id)
            .map_err(|error| error.to_string())?;
        Ok(AgentSchedulerScopeDto::new(project_scope))
    }

    /// 将缺失 `session_id` 的流式请求补齐为当前 runtime 绑定的会话。
    pub(crate) fn prepare_stream_request(
        &self,
        body: &GatewayChatStreamRequest,
    ) -> GatewayChatStreamRequest {
        let mut body = body.clone();
        if body.session_id.is_none()
            && let Some(session_id) = self.session_id()
        {
            body.session_id = Some(SessionId::from(session_id));
        }
        body
    }

    /// 解析调用方传入的会话 ID；若未提供，则回退到 runtime 当前会话。
    pub fn resolve_session_id<'a>(
        &'a self,
        session_id: Option<&'a str>,
    ) -> Result<&'a str, String> {
        normalize_optional_str_ref(session_id)
            .or_else(|| self.session_id())
            .ok_or_else(|| "gateway runtime session id is required".to_string())
    }

    /// 返回当前 runtime 应透传给 gateway 的目录查询值。
    pub fn directory_value(&self) -> Option<String> {
        runtime_directory_value(self.directory())
    }

    pub async fn enabled_acp_backend_keys(&self) -> Result<Vec<String>, String> {
        let catalog =
            gateway_config::GatewayConfigClient::new(self.client.transport().clone()).global_acp_config_get::<HashMap<String, AcpAgentConfig>>().await?;
        let config = gateway_config::GatewayConfigClient::new(self.client.transport().clone()).global_config_get().await?;
        Ok(resolve_enabled_acp_backend_keys(&catalog, &config))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn enabled_acp_backend_keys_blocking(&self) -> Result<Vec<String>, String> {
        block_on_gateway(self.enabled_acp_backend_keys())
    }
}

const DEFAULT_ENABLED_ACP_BACKENDS: &[&str] =
    &["claude", "gemini", "opencode", "codex", "openclaw", "copilot"];

fn resolve_enabled_acp_backend_keys(
    catalog: &HashMap<String, AcpAgentConfig>,
    config: &serde_json::Value,
) -> Vec<String> {
    let configured = config
        .get("acp")
        .and_then(serde_json::Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut keys = if configured.is_empty() {
        DEFAULT_ENABLED_ACP_BACKENDS
            .iter()
            .filter(|key| catalog.contains_key(**key))
            .map(|key| (*key).to_string())
            .collect()
    } else {
        configured
    };
    keys.sort();
    keys.dedup();
    keys
}

#[cfg(not(target_arch = "wasm32"))]
/// 以最小依赖方式在 CLI runtime 中阻塞等待 gateway 异步请求。
pub fn block_on_gateway<F, T>(future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|err| err.to_string())?;
            runtime.block_on(future)
        }
    }
}

pub(crate) fn annotate_gateway_transport_error(
    error: String,
    endpoint: &GatewayEndpoint,
) -> String {
    let message = normalize_optional_string(Some(error))
        .unwrap_or_else(|| "gateway request failed".to_string());

    if !looks_like_gateway_transport_error(message.as_str()) {
        return message;
    }

    format!(
        "{message}. Gateway endpoint {} is unavailable. Start it with `shiju gateway --host {} --port {}` and retry.",
        endpoint.describe(),
        endpoint.normalized_host(),
        endpoint.port
    )
}

/// 将 JSON 字符串字段规整为可选字符串。
fn gateway_optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// 将外部传入的字符串归一化为“空白即无值”。
fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn looks_like_gateway_transport_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.contains("error sending request")
        || normalized.contains("gateway preflight failed")
        || normalized.contains("connection refused")
        || normalized.contains("tcp connect error")
        || normalized.contains("dns error")
        || normalized.contains("couldn't connect to server")
}

/// 将可选字符串引用归一化为“空白即无值”。
pub fn normalize_optional_str_ref(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// 将目录路径规整为适合 gateway 查询参数的 UTF-8 字符串。
fn runtime_directory_value(directory: &Path) -> Option<String> {
    let directory = directory.to_string_lossy();
    let directory = directory.trim();
    if directory.is_empty() { None } else { Some(directory.to_string()) }
}
/// 生成“调用方主动停止消费流”时的统一终态。
fn cancelled_by_consumer_terminal() -> UiRuntimeTerminalEvent {
    UiRuntimeTerminalEvent::Cancelled {
        reason: Some("cancel requested; stream stopped after the next runtime event".to_string()),
        usage: None,
        message_id: None,
        parent_message_id: None,
    }
}

/// 在 stream 结束后，收口成稳定的终态返回值。
fn finalize_stream_terminal(
    result: Result<(), String>,
    terminal: Option<UiRuntimeTerminalEvent>,
) -> UiRuntimeTerminalEvent {
    if let Some(terminal) = terminal {
        return terminal;
    }

    match result {
        Ok(()) => {
            UiRuntimeTerminalEvent::Error("gateway stream closed before terminal event".to_string())
        }
        Err(err) => terminal_from_error_message(err),
    }
}

mod stream;
#[cfg(not(target_arch = "wasm32"))]
mod process;

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;
