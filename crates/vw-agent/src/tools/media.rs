//! Native agent tools for authenticated, session-owned gateway media jobs.

use agent_gateway::api::media::{MediaJob, MediaJobId, MediaOperation};
use anyhow::{Context as _, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::gateway::api::handlers::media::{self, ToolSubmission};

use super::{
    MEDIA_STATUS_TOOL_ID, MEDIA_SUBMIT_TOOL_ID, Tool, ToolCallResult, ToolResult, ToolSpec,
    ToolUseContext, current_tool_use_context,
};

pub(super) enum MediaTool {
    Submit,
    Status,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmitArgs {
    operation: MediaOperation,
    #[serde(default)]
    asset_ids: Vec<String>,
    deployment_id: Option<String>,
    model_id: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusArgs {
    job_id: MediaJobId,
}

fn active_context() -> Result<std::sync::Arc<ToolUseContext>> {
    current_tool_use_context()
        .filter(|context| context.media_authority().is_some())
        .context("Media tool denied: an authenticated gateway session is required")
}

fn submission_id(context: &ToolUseContext) -> Result<MediaJobId> {
    let turn = context
        .message_id()
        .or_else(|| context.turn_id())
        .context("Media submission requires a tool turn ID")?;
    let call = context.tool_call_id().context("Media submission requires a tool call ID")?;
    let mut hash = Sha256::new();
    hash.update(b"shiju:media-tool:submit:v1");
    for value in [context.session(), turn, call] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    let digest = hash.finalize();
    let bytes = digest[..16].try_into().expect("hash prefix is 16 bytes");
    Ok(uuid::Builder::from_random_bytes(bytes).into_uuid())
}

impl MediaTool {
    async fn run(&self, input: Value) -> Result<MediaJob> {
        let context = active_context()?;
        let job = match self {
            Self::Submit => {
                let args: SubmitArgs = serde_json::from_value(input)
                    .map_err(|_| anyhow::anyhow!("Invalid media submission arguments"))?;
                media::submit_for_tool(
                    &context,
                    ToolSubmission {
                        request_id: submission_id(&context)?,
                        operation: args.operation,
                        deployment_id: args.deployment_id,
                        model_id: args.model_id,
                        text: args.text,
                        asset_ids: args.asset_ids,
                    },
                )
                .await
            }
            Self::Status => {
                let args: StatusArgs = serde_json::from_value(input)
                    .map_err(|_| anyhow::anyhow!("Invalid media job ID"))?;
                media::job_for_tool(&context, args.job_id).await
            }
        };
        job.map_err(|code| anyhow::anyhow!(code.message()))
    }
}

#[async_trait::async_trait]
impl Tool for MediaTool {
    fn name(&self) -> &str {
        match self {
            Self::Submit => MEDIA_SUBMIT_TOOL_ID,
            Self::Status => MEDIA_STATUS_TOOL_ID,
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Submit => {
                "Generate or edit images, generate videos or speech, transcribe audio, or analyze images and videos. Specify a model or deployment only when the user requests a particular model for the media job. Otherwise the gateway uses the configured default for the operation, then prefers the manually selected conversation model when it supports the operation, then a compatible media model. Use media_status with the job ID to check progress or retrieve the result. Supply only asset IDs returned by media jobs in this session."
            }
            Self::Status => {
                "Read a session media job by ID. Returns its state, result text, and session asset references when available."
            }
        }
    }

    fn parameters_schema(&self) -> Value {
        match self {
            Self::Submit => json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["operation"],
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": [
                            MediaOperation::ImageGenerate, MediaOperation::ImageEdit,
                            MediaOperation::VideoGenerate, MediaOperation::SpeechSynthesize,
                            MediaOperation::AudioTranscribe, MediaOperation::ImageUnderstand,
                            MediaOperation::VideoUnderstand
                        ]
                    },
                    "text": { "type": "string", "description": "Prompt or speech text." },
                    "asset_ids": {
                        "type": "array", "maxItems": 8, "items": { "type": "string" },
                        "description": "Current session media asset IDs."
                    },
                    "deployment_id": { "type": "string", "description": "Optional authorized media deployment ID." },
                    "model_id": { "type": "string", "description": "Optional media model ID that supports the operation." }
                }
            }),
            Self::Status => json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["job_id"],
                "properties": { "job_id": { "type": "string" } }
            }),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_read_only(self.is_read_only())
            .with_strict(matches!(self, Self::Status))
    }

    fn is_read_only(&self) -> bool {
        matches!(self, Self::Status)
    }

    fn to_audit_input(&self, input: &Value) -> Value {
        match self {
            Self::Submit => json!({"operation": input.get("operation").and_then(|operation| {
                serde_json::from_value::<MediaOperation>(operation.clone()).ok()
            })}),
            Self::Status => json!({"job_id": input.get("job_id").and_then(|id| {
                serde_json::from_value::<MediaJobId>(id.clone()).ok()
            })}),
        }
    }

    async fn check_permissions(&self, _input: &Value) -> Result<()> {
        active_context().map(|_| ())
    }

    async fn execute(&self, input: Value) -> Result<ToolResult> {
        let job = self.run(input).await?;
        Ok(ToolResult { success: true, output: serde_json::to_string(&job)?, error: None })
    }

    async fn call(&self, input: Value) -> Result<ToolCallResult> {
        let job = self.run(input).await?;
        let metadata = json!({
            "media_job": { "id": job.id, "operation": job.operation, "session_id": job.session_id },
        });
        let job = serde_json::to_value(job)?;
        Ok(ToolCallResult {
            data: job.clone(),
            model_result: job,
            render_hint: Some(tool_types::ToolRenderHint { metadata, ..Default::default() }),
            ..Default::default()
        })
    }
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
