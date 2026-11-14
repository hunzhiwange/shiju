//! Native model tools share the gateway's authoritative host. Tool arguments never
//! carry host credentials, permission versions or control-session identities.
#[path = "computer_control/chrome.rs"]
mod chrome;

use super::{Tool, ToolCallResult, ToolResult, current_tool_use_context};
use crate::computer_control::{
    host::{ControlHost, HostMessage, HostReply, read_frame, write_frame},
    native::{NativeObservationError, NativeObserver},
    policy::ComputerControlPolicy,
};
use anyhow::{Context, bail};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Weak, atomic::AtomicBool},
    time::Duration,
};
use tokio::{
    net::UnixStream,
    sync::{Mutex, OnceCell},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::{computer_control::*, id::TenantScope, tools::ToolResultContentDto};
use vw_config_types::computer_control::ComputerControlConfig;

static GATEWAY_RUNTIME: RwLock<Weak<NativeControlRuntime>> = RwLock::new(Weak::new());
type Owner = (TenantScope, String, Option<std::path::PathBuf>);
type NativeResult<T> = Result<T, NativeObservationError>;
type BackendFactory = dyn Fn(
        Arc<RwLock<ComputerControlPolicy>>,
        Arc<AtomicBool>,
        ControlSessionId,
        PermissionVersion,
        ComputerControlTarget,
        CancellationToken,
    ) -> NativeResult<Arc<dyn NativeBackend>>
    + Send
    + Sync;

// Narrow external OS boundary; host policy, IPC, sequence checks and registry are real.
#[async_trait::async_trait]
pub(super) trait NativeBackend: Send + Sync {
    async fn observe(&self, screenshot: bool) -> NativeResult<ControlOutput>;
    async fn input(&self, input: &NativeInput) -> NativeResult<()>;
    fn resolve(&self, reference: &ScreenshotReference) -> NativeResult<String>;
}
#[async_trait::async_trait]
impl NativeBackend for NativeObserver {
    async fn observe(&self, screenshot: bool) -> NativeResult<ControlOutput> {
        NativeObserver::observe(self, screenshot).await
    }
    async fn input(&self, input: &NativeInput) -> NativeResult<()> {
        self.execute_input(input).await
    }
    fn resolve(&self, reference: &ScreenshotReference) -> NativeResult<String> {
        self.resolve_screenshot(reference)
    }
}
fn native_error(error: crate::computer_control::native::NativeObservationError) -> anyhow::Error {
    anyhow::anyhow!("native control {:?}: {}", error.code(), error.guidance())
}
fn control_error(code: ControlErrorCode) -> anyhow::Error {
    anyhow::anyhow!("computer control denied: {code:?}")
}
fn local_user() -> String {
    // SAFETY: geteuid has no preconditions.
    format!("uid:{}", unsafe { libc::geteuid() })
}

pub struct NativeControlRuntime {
    pub(super) policy: Arc<RwLock<ComputerControlPolicy>>,
    pub(super) host: Arc<ControlHost>,
    pub(super) locked: Arc<AtomicBool>,
    pub(super) office:
        RwLock<Option<Arc<crate::computer_control::office::transport::OfficeTransport>>>,
    factory: Arc<BackendFactory>,
    input_gate: Arc<Mutex<()>>,
    sessions: Mutex<HashMap<Owner, Arc<NativeSession>>>,
}
impl NativeControlRuntime {
    /// The bootstrap owner keeps this authority alive until gateway shutdown.
    pub(crate) fn start(config: ComputerControlConfig) -> anyhow::Result<Arc<Self>> {
        let mut shared = GATEWAY_RUNTIME.write();
        if shared.upgrade().is_some() {
            bail!("computer control host already running");
        }
        let runtime = Self::with_factory(
            Arc::new(RwLock::new(ComputerControlPolicy::new(config))),
            Arc::new(|policy, locked, session, version, target, cancel| {
                Ok(Arc::new(NativeObserver::new(policy, locked, session, version, target, cancel)?))
            }),
        );
        *shared = Arc::downgrade(&runtime);
        Ok(runtime)
    }
    pub(crate) fn shared() -> Option<Arc<Self>> {
        GATEWAY_RUNTIME.read().upgrade()
    }
    pub(super) fn with_factory(
        policy: Arc<RwLock<ComputerControlPolicy>>,
        factory: Arc<BackendFactory>,
    ) -> Arc<Self> {
        let locked = Arc::new(AtomicBool::new(false));
        Arc::new(Self {
            host: Arc::new(ControlHost::new(policy.clone(), locked.clone())),
            policy,
            locked,
            office: RwLock::new(None),
            factory,
            input_gate: Arc::new(Mutex::new(())),
            sessions: Mutex::new(HashMap::new()),
        })
    }
    /// Trusted local setup only: TLS credentials and origin never come from tool input.
    /// Binding does not pair a document or grant any execution permission.
    pub async fn bind_office(
        &self,
        credentials: &std::path::Path,
        origin: &str,
    ) -> anyhow::Result<Arc<crate::computer_control::office::transport::OfficeTransport>> {
        if self.office.read().is_some() {
            bail!("Office transport already running");
        }
        let transport = Arc::new(
            crate::computer_control::office::transport::OfficeTransport::bind(credentials, origin)
                .await?,
        );
        let mut slot = self.office.write();
        if slot.is_some() {
            bail!("Office transport already running");
        }
        *slot = Some(transport.clone());
        Ok(transport)
    }

    /// Management mutations serialize persistence and publication with pairing/stop.
    pub(crate) async fn stop_all(&self) {
        let mut sessions = self.sessions.lock().await;
        for session in sessions.values() {
            session.shutdown().await;
        }
        sessions.clear();
        if let Some(office) = self.office_transport() { office.revoke_all(); }
    }

    pub(crate) async fn chrome_connected(&self) -> bool {
        self.sessions.lock().await.values().any(|s| matches!(s.target, ComputerControlTarget::ChromeTab { .. }) && !s.task.is_finished() && s.expires > tokio::time::Instant::now())
    }

    pub(crate) fn control_policy(&self) -> &Arc<RwLock<ComputerControlPolicy>> { &self.policy }

    pub(crate) fn office_transport(&self) -> Option<Arc<crate::computer_control::office::transport::OfficeTransport>> {
        self.office.read().clone()
    }

    async fn session(
        &self,
        owner: Owner,
        target: &ComputerControlTarget,
    ) -> anyhow::Result<Arc<NativeSession>> {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|_, session| {
            session.expires > tokio::time::Instant::now() && !session.task.is_finished()
        });
        if let Some(session) = sessions
            .get(&owner)
            .filter(|s| &s.target == target && s.version == self.policy.read().version())
        {
            return Ok(session.clone());
        }
        if let Some(previous) = sessions.remove(&owner) {
            previous.shutdown().await;
        }
        let lifetime = Duration::from_secs(300);
        let (client, server) = UnixStream::pair()?;
        let chrome_listener = if matches!(target, ComputerControlTarget::ChromeTab { .. }) {
            Some(chrome::ChromeListener::bind()?)
        } else {
            None
        };
        let ticket = self.host.issue(target.clone(), lifetime).map_err(control_error)?;
        let bridge_session = ticket.session_id.clone();
        let backend: Arc<OnceCell<Arc<dyn NativeBackend>>> = Arc::new(OnceCell::new());
        let cell = backend.clone();
        let host = self.host.clone();
        let factory = self.factory.clone();
        let policy = self.policy.clone();
        let locked = self.locked.clone();
        let gate = self.input_gate.clone();
        let task = tokio::spawn(async move {
            if let Some(listener) = chrome_listener {
                return listener.serve(bridge_session, &host, server).await;
            }
            host.serve(server, |request, cancel| {
                let (cell, factory, policy, locked, gate) =
                    (cell.clone(), factory.clone(), policy.clone(), locked.clone(), gate.clone());
                async move {
                    let backend = cell
                        .get_or_try_init(|| async {
                            factory(
                                policy.clone(),
                                locked.clone(),
                                request.session_id.clone(),
                                request.permission_version,
                                request.target.clone(),
                                cancel.clone(),
                            )
                        })
                        .await
                        .map_err(|error| error.code())?;
                    match request.action {
                        ComputerControlAction::NativeObserve { screenshot } => {
                            backend.observe(screenshot).await.map_err(|error| error.code())
                        }
                        ComputerControlAction::NativeInput { input } => {
                            let _input = gate.lock().await;
                            if cancel.is_cancelled() {
                                return Err(ControlErrorCode::Cancelled);
                            }
                            policy.read().authorize(
                                &local_user(),
                                &request.target,
                                locked.load(std::sync::atomic::Ordering::SeqCst),
                                request.permission_version,
                            )?;
                            backend.input(&input).await.map_err(|error| error.code())?;
                            Ok(ControlOutput::Acknowledged)
                        }
                        _ => Err(ControlErrorCode::InvalidRequest),
                    }
                }
            })
            .await
        });
        // Construct the guard before any await so cancellation cannot leak a host session.
        let mut session = NativeSession {
            host: self.host.clone(),
            id: ticket.session_id.clone(),
            version: ticket.permission_version,
            target: target.clone(),
            expires: tokio::time::Instant::now() + lifetime,
            client: Mutex::new(Client { stream: client, sequence: 1 }),
            backend,
            task,
        };
        let client = session.client.get_mut();
        write_frame(
            &mut client.stream,
            &HostMessage::Hello { session_id: ticket.session_id, credential: ticket.credential },
        )
        .await?;
        match tokio::time::timeout(
            Duration::from_secs(35),
            read_frame::<_, HostReply>(&mut client.stream),
        )
        .await
        .context("Chrome extension must connect its active document within 35 seconds")??
        {
            HostReply::Ready => {}
            _ => bail!("computer control denied: host handshake failed"),
        }
        let session = Arc::new(session);
        sessions.insert(owner, session.clone());
        Ok(session)
    }
    pub(crate) async fn stop_context(&self, context: &super::ToolUseContext) {
        let owner = (context.tenant_scope(), context.session().into(), context.workspace_root());
        self.stop(&owner).await;
        if let (Some(office), Ok(owner)) = (self.office_transport(), super::excel_control::office_owner(context)) {
            office.revoke(&owner);
        }
    }

    async fn stop(&self, owner: &Owner) {
        if let Some(session) = self.sessions.lock().await.remove(owner) {
            session.shutdown().await;
        }
    }
}
struct Client {
    stream: UnixStream,
    sequence: u64,
}
struct NativeSession {
    host: Arc<ControlHost>,
    id: ControlSessionId,
    version: PermissionVersion,
    target: ComputerControlTarget,
    expires: tokio::time::Instant,
    client: Mutex<Client>,
    backend: Arc<OnceCell<Arc<dyn NativeBackend>>>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Drop for NativeSession {
    fn drop(&mut self) {
        self.host.stop(&self.id);
        self.task.abort();
    }
}
impl NativeSession {
    async fn shutdown(&self) {
        self.host.stop(&self.id);
        self.task.abort();
        // JoinHandle completion includes dropping the Chrome socket owner, so the
        // next owner cannot race a pending unlink of the previous listener.
        while !self.task.is_finished() {
            tokio::task::yield_now().await;
        }
    }
    async fn call(&self, action: ComputerControlAction) -> anyhow::Result<ToolCallResult> {
        let mut client = self.client.lock().await;
        let request_id = ControlRequestId(uuid::Uuid::new_v4().to_string());
        let request = ComputerControlRequest {
            request_id: request_id.clone(),
            session_id: self.id.clone(),
            permission_version: self.version,
            target: self.target.clone(),
            action,
        };
        chrome::approve_transfer(&self.host, &request).await?;
        let sequence = client.sequence;
        client.sequence = sequence.checked_add(1).context("computer control sequence exhausted")?;
        write_frame(&mut client.stream, &HostMessage::Action { sequence, request }).await?;
        let output = match read_frame::<_, HostReply>(&mut client.stream).await? {
            HostReply::Completed {
                request_id: actual,
                outcome: ControlOutcome::Succeeded { output },
            } if actual == request_id => output,
            HostReply::Rejected { error }
            | HostReply::Completed { outcome: ControlOutcome::Rejected { error }, .. } => {
                return Err(control_error(error));
            }
            _ => bail!("computer control invalid host reply"),
        };
        let mut blocks = Vec::new();
        let mut images = Vec::new();
        let completed = if matches!(self.target, ComputerControlTarget::ChromeTab { .. }) {
            "Chrome action completed"
        } else {
            "Native action completed"
        };
        match &output {
            ControlOutput::Observation { text, screenshots } => {
                blocks.push(json!({"type":"text","text":text}));
                for reference in screenshots {
                    let backend =
                        self.backend.get().context("computer control backend disconnected")?;
                    let image = backend.resolve(reference).map_err(native_error)?;
                    images.push(format!("[IMAGE:{image}]"));
                    blocks.push(json!({"type":"image","image_url":image}));
                }
            }
            ControlOutput::ChromeDownload { .. } => {
                blocks.push(json!({"type":"text","text":serde_json::to_string(&output)?}))
            }
            _ => blocks.push(json!({"type":"text","text":completed})),
        }
        let content_blocks = blocks
            .iter()
            .map(|block| ToolResultContentDto::Json { value: block.clone() })
            .collect();
        Ok(ToolCallResult {
            data: json!({"success":true,"output":output}),
            // Native providers consume tool results as text. Deliver image markers
            // as a user message through the existing multimodal preparation path.
            model_result: Value::String(match &output {
                ControlOutput::Observation { text, .. } => text.clone(),
                ControlOutput::ChromeDownload { .. } => serde_json::to_string(&output)?,
                _ => completed.into(),
            }),
            extra_messages: if images.is_empty() {
                vec![]
            } else {
                vec![super::ToolExtraMessage {
                    role: "user".into(),
                    content: Value::String(format!(
                        "Native application screenshot\n{}",
                        images.join("\n")
                    )),
                }]
            },
            content_blocks,
            ..Default::default()
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    target: ComputerControlTarget,
    action: ComputerControlAction,
}
fn parse(input: Value) -> anyhow::Result<Input> {
    let mut input: Input = serde_json::from_value(input)
        .map_err(|_| anyhow::anyhow!("invalid computer control request"))?;
    match (&mut input.target, &input.action) {
        (
            ComputerControlTarget::NativeApp { .. },
            ComputerControlAction::NativeObserve { .. }
            | ComputerControlAction::NativeInput { .. }
            | ComputerControlAction::Stop,
        ) => {}
        (
            ComputerControlTarget::ChromeTab { origin, tab_id },
            ComputerControlAction::Chrome { command },
        ) => {
            if *tab_id == 0 {
                bail!("invalid Chrome tab identity");
            }
            if crate::computer_control::chrome::origin_policy::canonical_origin(origin)
                .map_err(control_error)?
                != *origin
            {
                bail!("Chrome target requires a canonical origin without paths or credentials");
            }
            chrome::validate_command(command)?;
        }
        (ComputerControlTarget::ChromeTab { .. }, ComputerControlAction::Stop) => {}
        _ => bail!("unsupported computer control action or target"),
    }
    Ok(input)
}

pub struct ComputerControlTool {
    runtime: Arc<NativeControlRuntime>,
    session: Option<String>,
}
impl ComputerControlTool {
    pub(crate) fn new(runtime: Arc<NativeControlRuntime>, session: Option<String>) -> Self {
        Self { runtime, session }
    }
    fn context(&self) -> anyhow::Result<Arc<super::ToolUseContext>> {
        let context =
            current_tool_use_context().context("computer control requires executor context")?;
        if context.session().trim().is_empty()
            || self.session.as_ref().is_some_and(|id| id != context.session())
        {
            bail!("computer control denied: session owner mismatch");
        }
        Ok(context)
    }
}
#[async_trait::async_trait]
impl Tool for ComputerControlTool {
    fn name(&self) -> &str {
        "computer_control"
    }
    fn description(&self) -> &str {
        "Control an authorized native macOS app or a Chrome document. Native screenshots precede coordinate input. Chrome requires the enabled extension, active document grant, Chrome switch and separate site approval; navigate invalidates the old document. Upload bounded inline bytes (never paths) or download a same-origin link with separate approval each time. Excel and Word require dedicated tools. No scripts or shell commands."
    }
    fn parameters_schema(&self) -> Value {
        let native = json!({"type":"object","additionalProperties":false,"required":["target","action"],"properties":{
            "target":{"type":"object","additionalProperties":false,"required":["kind","bundle_id","process_id"],"properties":{"kind":{"const":"native_app"},"bundle_id":{"type":"string"},"process_id":{"type":"integer","minimum":1}}},
            "action":{"oneOf":[
                {"type":"object","additionalProperties":false,"required":["action","screenshot"],"properties":{"action":{"const":"native_observe"},"screenshot":{"type":"boolean"}}},
                {"type":"object","additionalProperties":false,"required":["action"],"properties":{"action":{"const":"stop"}}},
                {"type":"object","additionalProperties":false,"required":["action","input"],"properties":{"action":{"const":"native_input"},"input":{"oneOf":[
                    {"type":"object","additionalProperties":false,"required":["kind"],"properties":{"kind":{"const":"activate"}}},
                    {"type":"object","additionalProperties":false,"required":["kind","text"],"properties":{"kind":{"const":"type_text"},"text":{"type":"string","maxLength":16384}}},
                    {"type":"object","additionalProperties":false,"required":["kind","x","y","button"],"properties":{"kind":{"const":"click"},"x":{"type":"integer","minimum":0},"y":{"type":"integer","minimum":0},"button":{"enum":["left","right","middle"]}}},
                    {"type":"object","additionalProperties":false,"required":["kind","delta_x","delta_y"],"properties":{"kind":{"const":"scroll"},"delta_x":{"type":"integer"},"delta_y":{"type":"integer"}}},
                    {"type":"object","additionalProperties":false,"required":["kind","key","modifiers"],"properties":{"kind":{"const":"press"},"key":{"type":"string"},"modifiers":{"type":"array","items":{"enum":["shift","control","alt","meta"]},"uniqueItems":true}}}
                ]}}}
            ]}
        }});
        let chrome = chrome::schema();
        let mut actions = native["properties"]["action"]["oneOf"].as_array().unwrap().clone();
        actions.extend(
            chrome["properties"]["action"]["oneOf"].as_array().unwrap().iter().skip(1).cloned(),
        );
        json!({"type":"object","additionalProperties":false,"required":["target","action"],"properties":{
            "target":{"oneOf":[native["properties"]["target"],chrome["properties"]["target"]]},
            "action":{"oneOf":actions}
        }})
    }
    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        let parsed = parse(input.clone())?;
        Ok(json!({"target":parsed.target,"action":parsed.action}))
    }
    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        let context = self.context()?;
        let input = parse(input.clone())?;
        if matches!(input.action, ComputerControlAction::Stop) {
            return Ok(());
        }
        if let ComputerControlTarget::ChromeTab { ref origin, .. } = input.target {
            chrome::authorize_site(&self.runtime.policy, context.as_ref(), origin).await?;
            if let ComputerControlAction::Chrome { command: ChromeCommand::Navigate { url } } =
                &input.action
            {
                let destination =
                    crate::computer_control::chrome::origin_policy::canonical_origin(url)
                        .map_err(control_error)?;
                chrome::authorize_site(&self.runtime.policy, context.as_ref(), &destination)
                    .await?;
            }
        }
        let policy = self.runtime.policy.read();
        policy
            .authorize(
                &local_user(),
                &input.target,
                self.runtime.locked.load(std::sync::atomic::Ordering::SeqCst),
                policy.version(),
            )
            .map_err(control_error)
    }
    async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> {
        bail!("computer control requires structured executor call");
    }
    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        // Repeat after hooks; full access and generic tool approval never grant OS scope.
        self.check_permissions(&input).await?;
        let input = parse(input)?;
        let context = self.context()?;
        let owner = (context.tenant_scope(), context.session().into(), context.workspace_root());
        if matches!(input.action, ComputerControlAction::Stop) {
            self.runtime.stop_context(&context).await;
            return Ok(ToolCallResult::from_legacy_result(ToolResult {
                success: true,
                output: "Native control stopped".into(),
                error: None,
            }));
        }
        let acquire = self.runtime.session(owner.clone(), &input.target);
        let session = if let Some(cancel) = context.abort_token() {
            tokio::select! { biased; _ = cancel.cancelled() => Err(control_error(ControlErrorCode::Cancelled)), session = acquire => session }
        } else {
            acquire.await
        }?;
        let mut in_flight =
            InFlight { host: self.runtime.host.clone(), id: session.id.clone(), armed: true };
        let action = session.call(input.action);
        let result = if let Some(cancel) = context.abort_token() {
            tokio::select! { biased; _ = cancel.cancelled() => Err(control_error(ControlErrorCode::Cancelled)), result = action => result }
        } else {
            action.await
        };
        if result.is_err() {
            self.runtime.stop(&owner).await;
        } else {
            in_flight.armed = false;
        }
        result
    }
    fn spec(&self) -> super::ToolSpec {
        super::ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_destructive(true)
            .with_concurrency_safe(false)
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn to_audit_input(&self, input: &Value) -> Value {
        json!({"target":input.get("target"),"action":input.pointer("/action/action")})
    }
}

// Cancels effects even when the executor future is dropped or its task is aborted.
struct InFlight {
    host: Arc<ControlHost>,
    id: ControlSessionId,
    armed: bool,
}
impl Drop for InFlight {
    fn drop(&mut self) {
        if self.armed {
            self.host.stop(&self.id);
        }
    }
}
