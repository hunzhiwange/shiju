//! Structured Excel commands use authenticated Office sessions, never window titles.
use super::computer_control::NativeControlRuntime;
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
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use uuid::Uuid;
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::GrantDecision;

#[derive(Debug, thiserror::Error)]
#[error("Excel control denied: {0:?}")]
struct ControlDenied(ControlErrorCode);
fn denied(code: ControlErrorCode) -> anyhow::Error {
    ControlDenied(code).into()
}
pub(super) fn local_user() -> String {
    format!("uid:{}", unsafe { libc::geteuid() })
}

/// Pairing owners include tenant, chat and workspace; none is chosen by the model.
pub(crate) fn office_owner(context: &ToolUseContext) -> anyhow::Result<ControlSessionId> {
    ensure!(!context.session().trim().is_empty(), "Excel control denied: missing executor session");
    let identity =
        serde_json::to_vec(&(context.tenant_scope(), context.session(), context.workspace_root()))?;
    Ok(ControlSessionId(hex::encode(Sha256::digest(identity))))
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Input {
    Discover,
    Execute {
        document_id: ControlDocumentId,
        #[serde(deserialize_with = "parse_generation")]
        generation: Uuid,
        command: ExcelCommand,
    },
}
fn parse_generation<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
    let text = String::deserialize(deserializer)?;
    Uuid::parse_str(&text).map_err(serde::de::Error::custom)
}
fn parse(input: Value) -> anyhow::Result<Input> {
    ensure!(serde_json::to_vec(&input)?.len() <= 48 * 1024, "Excel request exceeds size limit");
    let input: Input = serde_json::from_value(input).context("invalid Excel request")?;
    if let Input::Execute { document_id, command, .. } = &input {
        ensure!(
            !document_id.0.trim().is_empty()
                && document_id.0.len() <= 256
                && !document_id.0.chars().any(char::is_control),
            "invalid Excel document identity"
        );
        let (sheet, range) = match command {
            ExcelCommand::ReadRange { sheet, range }
            | ExcelCommand::WriteRange { sheet, range, .. } => (sheet, range),
        };
        ensure!(!sheet.trim().is_empty() && sheet.chars().count() <= 31, "invalid Excel worksheet");
        let (rows, columns) = dimensions(range)?;
        if let ExcelCommand::WriteRange { values, .. } = command {
            ensure!(values.len() == rows && values.iter().all(|row|row.len() == columns && row.iter().all(|value| !matches!(value, OfficeCellValue::Text(text) if text.chars().count() > 4096))), "invalid Excel value matrix");
        }
    }
    Ok(input)
}
fn dimensions(range: &str) -> anyhow::Result<(usize, usize)> {
    fn cell(value: &str) -> Option<(usize, usize)> {
        let split = value.find(|c: char| !c.is_ascii_uppercase())?;
        let (letters, digits) = value.split_at(split);
        if letters.is_empty()
            || letters.len() > 3
            || digits.starts_with('0')
            || digits.len() > 7
            || !digits.bytes().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let column = letters.bytes().fold(0usize, |n, c| n * 26 + usize::from(c - b'A' + 1));
        let row = digits.parse::<usize>().ok()?;
        (column <= 16384 && row > 0 && row <= 1048576).then_some((row, column))
    }
    ensure!(range.len() <= 40, "invalid Excel range");
    let mut ends = range.split(':');
    let start = cell(ends.next().unwrap_or_default()).context("invalid Excel A1 range")?;
    let end = ends.next().map(cell).unwrap_or(Some(start)).context("invalid Excel A1 range")?;
    ensure!(
        ends.next().is_none() && end.0 >= start.0 && end.1 >= start.1,
        "invalid Excel rectangle"
    );
    let shape = (end.0 - start.0 + 1, end.1 - start.1 + 1);
    ensure!(shape.0 * shape.1 <= 1024, "Excel range exceeds 1024 cells");
    Ok(shape)
}

pub struct ExcelControlTool {
    runtime: Arc<NativeControlRuntime>,
    session: Option<String>,
}
impl ExcelControlTool {
    pub(crate) fn new(runtime: Arc<NativeControlRuntime>, session: Option<String>) -> Self {
        Self { runtime, session }
    }
    fn context(&self) -> anyhow::Result<Arc<ToolUseContext>> {
        let context =
            current_tool_use_context().context("Excel control requires executor context")?;
        office_owner(&context)?;
        ensure!(
            self.session.as_ref().is_none_or(|s| s == context.session()),
            "Excel control denied: session owner mismatch"
        );
        Ok(context)
    }
    fn enabled(&self) -> anyhow::Result<()> {
        let policy = self.runtime.policy.read();
        let user =
            policy.config().users.get(&local_user()).context("Excel control denied: disabled")?;
        // Locked Office execution needs its own verified backend mechanism.
        if !user.excel || self.runtime.locked.load(Ordering::SeqCst) {
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
                .context("Excel control denied: disabled")?;
            match user.excel_documents.get(&id.0) {
                Some(GrantDecision::Allow) => return Ok(()),
                Some(GrantDecision::Deny) => {
                    return Err(denied(ControlErrorCode::PermissionDenied));
                }
                None => policy.version(),
            }
        };
        if !super::decision::request_control_scope_approval("excel_control.document", json!({"document_id":id,"scope":"Excel document access until revoked or gateway restart; add-in range approval still applies"}), context).await {
            return Err(denied(ControlErrorCode::PermissionDenied));
        }
        let mut policy = self.runtime.policy.write();
        if policy.version() != version {
            return Err(denied(ControlErrorCode::StalePermission));
        }
        policy
            .set_grant(
                &local_user(),
                GrantScope::ExcelDocument(id.0.clone()),
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
        command: &ExcelCommand,
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
                s.handle.surface == OfficeSurface::Excel
                    && s.handle.document_id == *id
                    && s.handle.generation == generation
            })
            .ok_or_else(|| denied(ControlErrorCode::Disconnected))?;
        let capability = match command {
            ExcelCommand::ReadRange { .. } => OfficeCapability::ExcelReadRange,
            ExcelCommand::WriteRange { .. } => OfficeCapability::ExcelWriteRange,
        };
        ensure!(
            session.capabilities.contains(&capability),
            ControlDenied(ControlErrorCode::Unsupported)
        );
        Ok((transport, session.handle))
    }
}

#[async_trait::async_trait]
impl Tool for ExcelControlTool {
    fn name(&self) -> &str {
        "excel_control"
    }
    fn description(&self) -> &str {
        "Discover paired Excel workbooks, then read or write an exact document_id and generation. Requires Excel switch, document grant and add-in range approval. Writes return actual calculated values, formulas and number formats. Re-discover after reconnect or Save As. No title selection, scripts, shell, macros or offline file fallback."
    }
    fn parameters_schema(&self) -> Value {
        let sheet = json!({"type":"string","minLength":1,"maxLength":31});
        let range = json!({"type":"string","minLength":2,"maxLength":40,"description":"Single bounded uppercase A1 rectangle, at most 1024 cells"});
        json!({"oneOf":[
            {"type":"object","additionalProperties":false,"required":["operation"],"properties":{"operation":{"const":"discover"}}},
            {"type":"object","additionalProperties":false,"required":["operation","document_id","generation","command"],"properties":{
                "operation":{"const":"execute"},"document_id":{"type":"string","minLength":1,"maxLength":256},"generation":{"type":"string","format":"uuid"},
                "command":{"oneOf":[
                    {"type":"object","additionalProperties":false,"required":["kind","sheet","range"],"properties":{"kind":{"const":"read_range"},"sheet":sheet,"range":range}},
                    {"type":"object","additionalProperties":false,"required":["kind","sheet","range","values"],"properties":{"kind":{"const":"write_range"},"sheet":sheet,"range":range,"values":{"type":"array","minItems":1,"maxItems":1024,"items":{"type":"array","minItems":1,"maxItems":1024,"items":{"type":["string","number","boolean","null"],"maxLength":4096}},"description":"Rectangular values; approved formulas use = strings. Format and overwrite consent are fixed in the add-in."}}}
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
        bail!("Excel control requires structured executor call")
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
                sessions.retain(|s| s.handle.surface == OfficeSurface::Excel);
                sessions.sort_by_key(|s| (s.handle.document_id.0.clone(), s.handle.generation));
                json!({"success":true,"sessions":sessions.iter().map(|s| json!({"document_id":s.handle.document_id,"generation":s.handle.generation.to_string(),"title":s.title,"capabilities":s.capabilities.iter().map(|c|match c {OfficeCapability::ExcelReadRange=>"read_range",OfficeCapability::ExcelWriteRange=>"write_range",_=>"unsupported"}).collect::<Vec<_>>()})).collect::<Vec<_>>()})
            }
            Input::Execute { document_id, generation, command } => {
                let (transport, handle) =
                    self.select(&owner, &document_id, generation, &command)?;
                let target =
                    ComputerControlTarget::ExcelWorkbook { document_id: document_id.clone() };
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
                    action: ComputerControlAction::Excel { command },
                };
                self.runtime.host.recheck(&request).map_err(denied)?;
                let run = transport.execute(&owner, &handle, &request.target, &request.action);
                let result = if let Some(cancel) = context.abort_token() {
                    tokio::select! { biased; _ = cancel.cancelled() => Err(denied(ControlErrorCode::Cancelled)), result = run => result }
                } else {
                    run.await
                }?;
                self.runtime.host.recheck(&request).map_err(denied)?;
                self.enabled()?;
                ensure!(
                    serde_json::to_vec(&result)?.len() <= 48 * 1024,
                    "Excel result exceeds size limit"
                );
                let (sheet, range) = match &request.action {
                    ComputerControlAction::Excel {
                        command:
                            ExcelCommand::ReadRange { sheet, range }
                            | ExcelCommand::WriteRange { sheet, range, .. },
                    } => (sheet, range),
                    _ => unreachable!(),
                };
                let (rows, columns) = dimensions(range)?;
                let matrices_match = ["values", "formulas", "number_format"].iter().all(|key| {
                    result[*key].as_array().is_some_and(|matrix| {
                        matrix.len() == rows
                            && matrix.iter().all(|row| {
                                row.as_array().is_some_and(|cells| {
                                    cells.len() == columns
                                        && cells.iter().all(|cell| {
                                            if *key == "number_format" {
                                                cell.is_string()
                                            } else {
                                                cell.is_null()
                                                    || cell.is_string()
                                                    || cell.is_boolean()
                                                    || cell.is_number()
                                            }
                                        })
                                })
                            })
                    })
                });
                ensure!(
                    result["sheet"] == *sheet
                        && result["range"] == *range
                        && result["address"]
                            .as_str()
                            .is_some_and(|address| !address.is_empty() && address.len() <= 1024)
                        && matrices_match,
                    "invalid Excel readback"
                );
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
