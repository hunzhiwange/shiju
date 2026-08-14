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

use super::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_gateway_stream_event};
use crate::app::agent::config::Config;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use vw_config_types::config::AcpAgentConfig;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::ProjectScope;
use vw_gateway_client::vw_api_types::id::{
    COMPANY_ID_HEADER, PLATFORM_ID_HEADER, SessionId, TenantScope,
};
use vw_gateway_client::vw_api_types::project::{ProjectDto, ResolveProjectRequest};
use vw_gateway_client::{GatewayAuth, GatewayChatStreamRequest, GatewayClient, GatewayEndpoint};

#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_HEALTH_PATH: &str = "/v1/health";
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_TIMEOUT: Duration = Duration::from_secs(8);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(150);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GatewayPreflightOutcome {
    Ready,
    Started,
}

#[cfg(not(target_arch = "wasm32"))]
impl GatewayPreflightOutcome {
    pub(crate) fn started_gateway(self) -> bool {
        matches!(self, Self::Started)
    }
}

/// CLI 侧用于构造 `GatewayClient` 的最小引导配置。
///
/// 该结构只覆盖客户端访问网关所需的字段：主机、端口与认证信息。
/// 字段来源为桌面端维护的 `gateway-client-bootstrap.json` 启动缓存，
/// 不把它当作完整系统配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GatewayClientBootstrapConfig {
    /// 目标网关主机名或 IP。
    pub(crate) host: String,
    /// 目标网关端口。
    pub(crate) port: u16,
    /// 可选 skey，会作为 Bearer token 发送。
    pub(crate) skey: Option<String>,
    /// 请求所属的平台租户 ID。
    pub(crate) platform_id: String,
    /// 请求所属的公司租户 ID。
    pub(crate) company_id: String,
}

impl Default for GatewayClientBootstrapConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: vw_config_types::gateway::DEFAULT_GATEWAY_PORT,
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
    pub(crate) fn load(config: &Config) -> Self {
        let mut bootstrap = Self::default();

        let Some(config_dir) = config.config_path.parent() else {
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
    pub(crate) fn endpoint(&self) -> GatewayEndpoint {
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
pub(crate) struct GatewaySessionSeed {
    id: Option<String>,
    directory: PathBuf,
    scope: Option<String>,
    title: Option<String>,
}

impl GatewaySessionSeed {
    /// 使用请求目录创建新的会话上下文。
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self { id: None, directory, scope: None, title: None }
    }

    /// 返回当前 runtime 绑定的会话 ID（如果已设置）。
    pub(crate) fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// 返回当前 runtime 绑定的请求目录。
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    /// 返回会话 scope（如果已设置）。
    pub(crate) fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// 返回会话标题（如果已设置）。
    pub(crate) fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// 用新的会话 ID 覆盖当前种子。
    pub(crate) fn with_id(mut self, id: Option<String>) -> Self {
        self.id = normalize_optional_string(id);
        self
    }

    /// 用新的 scope 覆盖当前种子。
    pub(crate) fn with_scope(mut self, scope: Option<String>) -> Self {
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
pub(crate) struct GatewayUiRuntime {
    client: GatewayClient,
    session: GatewaySessionSeed,
}

impl GatewayUiRuntime {
    /// 基于外部已创建好的 client 与会话种子构造 runtime。
    pub(crate) fn new(client: GatewayClient, session: GatewaySessionSeed) -> Self {
        Self { client, session }
    }

    /// 基于 CLI 配置与给定会话种子直接创建 runtime。
    ///
    /// 这是 S1-1 的主要接线点：CLI 现在可以直接读取自身配置并构造 `GatewayClient`，
    /// 不必经过 legacy processor 或桌面端模块。
    pub(crate) fn from_config(
        config: &Config,
        session: GatewaySessionSeed,
    ) -> Result<Self, String> {
        let client = gateway_client(config)?;
        Ok(Self::new(client, session))
    }

    /// 以当前 shell 的请求目录作为默认请求目录创建 runtime。
    pub(crate) fn for_workspace(config: &Config) -> Result<Self, String> {
        let request_root = std::env::current_dir().unwrap_or_else(|_| config.workspace_dir.clone());
        Self::from_config(config, GatewaySessionSeed::new(request_root))
    }

    /// 返回统一的 gateway client 引用。
    pub(crate) fn client(&self) -> &GatewayClient {
        &self.client
    }

    /// 返回当前 runtime 绑定的 gateway endpoint。
    pub(crate) fn endpoint(&self) -> &GatewayEndpoint {
        self.client.endpoint()
    }

    /// 返回当前 runtime 的会话种子。
    pub(crate) fn session(&self) -> &GatewaySessionSeed {
        &self.session
    }

    /// 返回当前请求目录。
    pub(crate) fn directory(&self) -> &Path {
        self.session.directory()
    }

    /// 返回当前会话 scope。
    pub(crate) fn scope(&self) -> Option<&str> {
        self.session.scope()
    }

    /// 返回当前会话标题。
    pub(crate) fn title(&self) -> Option<&str> {
        self.session.title()
    }

    /// 返回当前会话 ID。
    pub(crate) fn session_id(&self) -> Option<&str> {
        self.session.id()
    }

    /// 用当前 UI 已确认的 session 元信息覆盖 runtime seed，保持后续默认回退一致。
    pub(crate) fn bind_session_seed(
        &mut self,
        session_id: Option<String>,
        scope: Option<String>,
        title: Option<String>,
    ) {
        self.session = self.session.clone().with_id(session_id).with_scope(scope).with_title(title);
    }

    pub(crate) fn rebind_validated_directory(&mut self, path: &Path) -> Result<PathBuf, String> {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("cannot resolve directory {}: {error}", path.display()))?;
        if !canonical.is_dir() {
            return Err(format!("target is not a directory: {}", canonical.display()));
        }
        self.session.directory = canonical.clone();
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
        self.client
            .project_resolve(self.endpoint().tenant_scope(), &request)
            .await
            .map(|response| response.project)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn resolve_project_directory_blocking(
        &self,
        directory: &Path,
        create_if_missing: bool,
    ) -> Result<ProjectDto, String> {
        block_on_gateway(self.resolve_project_directory(directory, create_if_missing))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn agent_scheduler_scope_blocking(&self) -> Result<AgentSchedulerScopeDto, String> {
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
    pub(crate) fn resolve_session_id<'a>(
        &'a self,
        session_id: Option<&'a str>,
    ) -> Result<&'a str, String> {
        normalize_optional_str_ref(session_id)
            .or_else(|| self.session_id())
            .ok_or_else(|| "gateway runtime session id is required".to_string())
    }

    /// 返回当前 runtime 应透传给 gateway 的目录查询值。
    pub(crate) fn directory_value(&self) -> Option<String> {
        runtime_directory_value(self.directory())
    }

    pub(crate) async fn enabled_acp_backend_keys(&self) -> Result<Vec<String>, String> {
        let catalog =
            self.client.global_acp_config_get::<HashMap<String, AcpAgentConfig>>().await?;
        let config = self.client.global_config_get().await?;
        Ok(resolve_enabled_acp_backend_keys(&catalog, &config))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn enabled_acp_backend_keys_blocking(&self) -> Result<Vec<String>, String> {
        block_on_gateway(self.enabled_acp_backend_keys())
    }

    /// 以异步方式发起聊天流，并把 gateway 事件规整为 `UiRuntimeEvent`。
    ///
    /// 返回值始终是统一的终态控制事件，用于让上层明确区分
    /// done、cancel、timeout 与 error。
    pub(crate) async fn stream_chat(
        &self,
        body: &GatewayChatStreamRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let directory = self.directory_value();
        let body = self.prepare_stream_request(body);
        let mut terminal = None;

        let result =
            GatewayClient::stream_chat(self.endpoint(), directory.as_deref(), &body, |event| {
                let runtime_event = adapt_gateway_stream_event(event);
                if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                    terminal = Some(runtime_terminal.clone());
                }

                if on_event(runtime_event) {
                    true
                } else {
                    if terminal.is_none() {
                        terminal = Some(cancelled_by_consumer_terminal());
                    }
                    false
                }
            })
            .await
            .map_err(|err| annotate_gateway_transport_error(err, self.endpoint()));

        finalize_stream_terminal(result, terminal)
    }

    /// 以阻塞方式发起聊天流，并把 gateway 事件规整为 `UiRuntimeEvent`。
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn stream_chat_blocking(
        &self,
        body: &GatewayChatStreamRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let directory = self.directory_value();
        let body = self.prepare_stream_request(body);
        let mut terminal = None;

        let result = GatewayClient::stream_chat_blocking(
            self.endpoint(),
            directory.as_deref(),
            &body,
            |event| {
                let runtime_event = adapt_gateway_stream_event(event);
                if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                    terminal = Some(runtime_terminal.clone());
                }

                if on_event(runtime_event) {
                    true
                } else {
                    if terminal.is_none() {
                        terminal = Some(cancelled_by_consumer_terminal());
                    }
                    false
                }
            },
        )
        .map_err(|err| annotate_gateway_transport_error(err, self.endpoint()));

        finalize_stream_terminal(result, terminal)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 在进入 tui_v2 前预检 gateway；若 endpoint 指向本地 loopback，则自动尝试拉起。
    pub(crate) fn ensure_local_gateway_ready_blocking(
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
pub(crate) fn block_on_gateway<F, T>(future: F) -> Result<T, String>
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

/// 根据 CLI 运行时配置构造 gateway endpoint。
pub(crate) fn gateway_client_endpoint(config: &Config) -> GatewayEndpoint {
    GatewayClientBootstrapConfig::load(config).endpoint()
}

/// 根据 CLI 运行时配置直接构造 `GatewayClient`。
pub(crate) fn gateway_client(config: &Config) -> Result<GatewayClient, String> {
    GatewayClient::new(gateway_client_endpoint(config))
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
        "{message}. Gateway endpoint {} is unavailable. Start it with `vibewindow gateway --host {} --port {}` and retry.",
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
pub(crate) fn normalize_optional_str_ref(value: Option<&str>) -> Option<&str> {
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
        Err(err) => UiRuntimeTerminalEvent::from_error_message(err),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn is_local_loopback_endpoint(endpoint: &GatewayEndpoint) -> bool {
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
fn local_gateway_startup_failure_message(
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
fn wait_for_gateway_health(endpoint: &GatewayEndpoint, timeout: Duration) -> bool {
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
fn gateway_health_ready(endpoint: &GatewayEndpoint) -> bool {
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
fn resolve_gateway_socket_address(endpoint: &GatewayEndpoint) -> Option<std::net::SocketAddr> {
    let host = endpoint.normalized_host();
    if host.is_empty() || host.chars().any(char::is_whitespace) {
        return None;
    }
    format!("{host}:{}", endpoint.port)
        .to_socket_addrs()
        .ok()?
        .find(|addr| addr.is_ipv4() || addr.is_ipv6())
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;
