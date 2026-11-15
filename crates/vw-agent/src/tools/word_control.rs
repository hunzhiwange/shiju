//! Structured Word commands use authenticated Office sessions, never window titles.
use super::computer_control::NativeControlRuntime;
use super::excel_control::{local_user, office_owner};
use super::{Tool, ToolCallResult, ToolResult, ToolUseContext, current_tool_use_context};
use crate::computer_control::{
    office::{
        sessions::{OfficeCapability, OfficeHandle, OfficeSurface},
        transport::OfficeTransport,
    },
    policy::GrantScope,
};
use anyhow::{Context, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::GrantDecision;

#[derive(Debug, thiserror::Error)]
#[error("Word control denied: {0:?}")]
struct ControlDenied(ControlErrorCode);
fn denied(code: ControlErrorCode) -> anyhow::Error {
    ControlDenied(code).into()
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Input {
    Discover,
    Execute {
        document_id: ControlDocumentId,
        #[serde(deserialize_with = "parse_generation")]
        generation: Uuid,
        command: WordCommand,
    },
}
fn parse_generation<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
    let text = String::deserialize(deserializer)?;
    Uuid::parse_str(&text).map_err(serde::de::Error::custom)
}
fn parse(input: Value) -> anyhow::Result<Input> {
    ensure!(serde_json::to_vec(&input)?.len() <= 48 * 1024, "Word request exceeds size limit");
    let input: Input = serde_json::from_value(input).context("invalid Word request")?;
    if let Input::Execute { document_id, command, .. } = &input {
        ensure!(
            !document_id.0.trim().is_empty()
                && document_id.0.len() <= 256
                && !document_id.0.chars().any(char::is_control),
            "invalid Word document identity"
        );
        match command {
            WordCommand::ReadDocument => {}
            WordCommand::FindText { text } => validate_text(text, false)?,
            WordCommand::ReplaceText { search, replacement } => {
                validate_text(search, false)?;
                validate_text(replacement, true)?;
            }
            WordCommand::InsertText { text, .. } => validate_text(text, false)?,
        }
    }
    Ok(input)
}
fn validate_text(text: &str, empty: bool) -> anyhow::Result<()> {
    ensure!((empty || !text.is_empty()) && text.encode_utf16().count() <= 8192
        && !text.chars().any(|c| c.is_control() && c != '\t'),
        "invalid Word single-paragraph text");
    Ok(())
}

pub struct WordControlTool {
    runtime: Arc<NativeControlRuntime>,
    session: Option<String>,
}
impl WordControlTool {
    pub(crate) fn new(runtime: Arc<NativeControlRuntime>, session: Option<String>) -> Self {
        Self { runtime, session }
    }
    fn context(&self) -> anyhow::Result<Arc<ToolUseContext>> {
        let context =
            current_tool_use_context().context("Word control requires executor context")?;
        office_owner(&context)?;
        ensure!(
            self.session.as_ref().is_none_or(|s| s == context.session()),
            "Word control denied: session owner mismatch"
        );
        Ok(context)
    }
    fn enabled(&self) -> anyhow::Result<()> {
        let restricted = self.runtime.host.restricted_session();
        let policy = self.runtime.policy.read();
        let user =
            policy.config().users.get(&local_user()).context("Word control denied: disabled")?;
        // The paired Office runtime executes document commands without GUI input.
        if !user.word || (restricted && !user.locked_use) {
            return Err(denied(ControlErrorCode::PermissionDenied));
        }
        Ok(())
    }
    async fn authorize_document(
        &self,
        id: &ControlDocumentId,
        context: &ToolUseContext,
    ) -> anyhow::Result<()> {
        self.enabled()?;
        let version = {
            let policy = self.runtime.policy.read();
            let user = policy
                .config()
                .users
                .get(&local_user())
                .context("Word control denied: disabled")?;
            match user.word_documents.get(&id.0) {
                Some(GrantDecision::Allow) => return Ok(()),
                Some(GrantDecision::Deny) => {
                    return Err(denied(ControlErrorCode::PermissionDenied));
                }
                None => policy.version(),
            }
        };
        if self.runtime.host.restricted_session() {
            return Err(denied(ControlErrorCode::PermissionDenied));
        }
        if !super::decision::request_control_scope_approval("word_control.document", json!({"document_id":id,"scope":"Word document access until revoked or gateway restart; add-in range approval still applies"}), context).await {
            return Err(denied(ControlErrorCode::PermissionDenied));
        }
        if self.runtime.host.restricted_session() {
            return Err(denied(ControlErrorCode::PermissionDenied));
        }
        let mut policy = self.runtime.policy.write();
        if policy.version() != version {
            return Err(denied(ControlErrorCode::StalePermission));
        }
        policy
            .set_grant(
                &local_user(),
                GrantScope::WordDocument(id.0.clone()),
                Some(GrantDecision::Allow),
            )
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }
    fn select(
        &self,
        owner: &ControlSessionId,
        id: &ControlDocumentId,
        generation: Uuid,
        command: &WordCommand,
    ) -> anyhow::Result<(Arc<OfficeTransport>, OfficeHandle)> {
        let transport = self
            .runtime
            .office
            .read()
            .clone()
            .ok_or_else(|| denied(ControlErrorCode::Disconnected))?;
        let session = transport
            .documents(owner)
            .into_iter()
            .find(|s| {
                s.handle.surface == OfficeSurface::Word
                    && s.handle.document_id == *id
                    && s.handle.generation == generation
            })
            .ok_or_else(|| denied(ControlErrorCode::Disconnected))?;
        let capability = match command {
            WordCommand::ReadDocument => OfficeCapability::WordReadDocument,
            WordCommand::FindText { .. } => OfficeCapability::WordFindText,
            WordCommand::ReplaceText { .. } => OfficeCapability::WordReplaceText,
            WordCommand::InsertText { .. } => OfficeCapability::WordInsertText,
        };
        ensure!(
            session.capabilities.contains(&capability),
            ControlDenied(ControlErrorCode::Unsupported)
        );
        Ok((transport, session.handle))
    }
}

#[async_trait::async_trait]
impl Tool for WordControlTool {
    fn name(&self) -> &str {
        "word_control"
    }
    fn description(&self) -> &str {
        "Discover paired Word documents, then read, find, replace or insert in the approved paragraph using exact document_id and generation. Read or find before every write; replacement search is the expected text and must match uniquely. Writes return actual changed text, previous text and paragraph. Requires independent Word and document approval. Rediscover after reconnect. No scripts or offline file fallback."
    }
    fn parameters_schema(&self) -> Value {
        let text = json!({"type":"string","minLength":1,"maxLength":8192});
        json!({"oneOf":[
            {"type":"object","additionalProperties":false,"required":["operation"],"properties":{"operation":{"const":"discover"}}},
            {"type":"object","additionalProperties":false,"required":["operation","document_id","generation","command"],"properties":{
                "operation":{"const":"execute"},"document_id":{"type":"string","minLength":1,"maxLength":256},"generation":{"type":"string","format":"uuid"},
                "command":{"oneOf":[
                    {"type":"object","additionalProperties":false,"required":["kind"],"properties":{"kind":{"const":"read_document"}}},
                    {"type":"object","additionalProperties":false,"required":["kind","text"],"properties":{"kind":{"const":"find_text"},"text":text}},
                    {"type":"object","additionalProperties":false,"required":["kind","search","replacement"],"properties":{"kind":{"const":"replace_text"},"search":text,"replacement":{"type":"string","maxLength":8192}}},
                    {"type":"object","additionalProperties":false,"required":["kind","position","text"],"properties":{"kind":{"const":"insert_text"},"position":{"enum":["start","end","selection"]},"text":text}}
                ]}
            }}
        ]})
    }
    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        parse(input.clone())?;
        Ok(input)
    }
    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        let context = self.context()?;
        self.enabled()?;
        if let Input::Execute { document_id, generation, command } = parse(input.clone())? {
            self.select(&office_owner(&context)?, &document_id, generation, &command)?;
            self.authorize_document(&document_id, &context).await?;
        }
        Ok(())
    }
    async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> {
        bail!("Word control requires structured executor call")
    }
    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        self.check_permissions(&input).await?;
        let context = self.context()?;
        let owner = office_owner(&context)?;
        let data = match parse(input)? {
            Input::Discover => {
                let transport = self.runtime.office.read().clone();
                let mut sessions =
                    transport.as_ref().map(|t| t.documents(&owner)).unwrap_or_default();
                sessions.retain(|s| s.handle.surface == OfficeSurface::Word);
                sessions.sort_by_key(|s| (s.handle.document_id.0.clone(), s.handle.generation));
                json!({"success":true,"sessions":sessions.iter().map(|s| json!({"document_id":s.handle.document_id,"generation":s.handle.generation.to_string(),"title":s.title,"capabilities":s.capabilities.iter().map(|c|match c {OfficeCapability::WordReadDocument=>"read_document",OfficeCapability::WordFindText=>"find_text",OfficeCapability::WordReplaceText=>"replace_text",OfficeCapability::WordInsertText=>"insert_text",_=>"unsupported"}).collect::<Vec<_>>()})).collect::<Vec<_>>()})
            }
            Input::Execute { document_id, generation, command } => {
                let (transport, handle) =
                    self.select(&owner, &document_id, generation, &command)?;
                let target =
                    ComputerControlTarget::WordDocument { document_id: document_id.clone() };
                let ticket = self
                    .runtime
                    .host
                    .issue(target.clone(), Duration::from_secs(30))
                    .map_err(denied)?;
                let mut guard = InFlight {
                    runtime: self.runtime.clone(),
                    transport: transport.clone(),
                    owner: owner.clone(),
                    session: ticket.session_id.clone(),
                    armed: true,
                };
                let request = ComputerControlRequest {
                    request_id: ControlRequestId(Uuid::new_v4().to_string()),
                    session_id: ticket.session_id,
                    permission_version: ticket.permission_version,
                    target,
                    action: ComputerControlAction::Word { command },
                };
                self.runtime.host.recheck(&request).map_err(denied)?;
                let run = self.runtime.host.guard_execution(
                    &request,
                    transport.execute(&owner, &handle, &request.target, &request.action),
                );
                let result = if let Some(cancel) = context.abort_token() {
                    tokio::select! { biased; _ = cancel.cancelled() => Err(denied(ControlErrorCode::Cancelled)), result = run => result }
                } else {
                    run.await
                }?;
                self.runtime.host.recheck(&request).map_err(denied)?;
                self.enabled()?;
                ensure!(
                    serde_json::to_vec(&result)?.len() <= 48 * 1024,
                    "Word result exceeds size limit"
                );
                ensure!(result["revision"].as_u64().is_some()
                    && result["text"].is_string() && result["paragraph"].is_string(),
                    "invalid Word readback");
                match &request.action {
                    ComputerControlAction::Word { command: WordCommand::ReadDocument } => {
                        ensure!(result["selection"].is_string(), "invalid Word readback");
                    }
                    ComputerControlAction::Word { command: WordCommand::FindText { text } } => {
                        ensure!(result["text"] == *text, "invalid Word readback");
                    }
                    ComputerControlAction::Word { command: WordCommand::ReplaceText { search, replacement } } => {
                        ensure!(result["previous"] == *search && result["text"] == *replacement,
                            "invalid Word readback");
                    }
                    ComputerControlAction::Word { command: WordCommand::InsertText { text, .. } } => {
                        ensure!(result["previous"].is_string() && result["text"] == *text,
                            "invalid Word readback");
                    }
                    _ => unreachable!(),
                }
                guard.armed = false;
                json!({"success":true,"document_id":document_id,"generation":generation.to_string(),"permission_version":request.permission_version,"result":result})
            }
        };
        Ok(ToolCallResult {
            model_result: Value::String(serde_json::to_string(&data)?),
            data,
            ..Default::default()
        })
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
        json!({"operation":input.get("operation"),"document_id":input.get("document_id"),"generation":input.get("generation"),"command":input.pointer("/command/kind")})
    }
}
struct InFlight {
    runtime: Arc<NativeControlRuntime>,
    transport: Arc<OfficeTransport>,
    owner: ControlSessionId,
    session: ControlSessionId,
    armed: bool,
}
impl Drop for InFlight {
    fn drop(&mut self) {
        self.runtime.host.stop(&self.session);
        if self.armed {
            self.transport.revoke(&self.owner);
        }
    }
}
