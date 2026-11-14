//! Inert share parsing and confirmed import orchestration. Callbacks must be
//! bound to the authenticated host's prompt store and guarded activation owner.
use base64::{Engine as _, prelude::*};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use vw_api_types::agent_apps::{
    ActivatePromptRequest, AppId, ImportPromptShareRequest, PromptActivationChoice, PromptEntry,
    PromptEntryRequest, PromptResource, PromptSharePayload,
};

/// Already URL-decoded fields; protocol registration belongs to OS dispatch.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedPromptFields {
    pub resource: String,
    pub app: Option<AppId>,
    pub name: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

pub fn parse(fields: SharedPromptFields) -> Result<PromptSharePayload, &'static str> {
    if fields.resource != "prompt" {
        return Err("expected prompt resource");
    }
    let app = fields.app.ok_or("missing app")?;
    PromptResource::Application { app_id: app }
        .validate_application_access()
        .map_err(|_| "unsupported prompt application")?;
    let name = fields.name.ok_or("missing name")?;
    let raw = fields.content.ok_or("missing content")?;
    // Match source link decoding: query parsers may turn '+' into spaces.
    let normalized = raw.trim_matches(['\r', '\n']).replace(' ', "+");
    let mut padded = normalized.clone();
    let remainder = padded.len() % 4;
    if remainder != 0 {
        padded.extend(std::iter::repeat_n('=', 4 - remainder));
    }
    let mut decoded = None;
    for candidate in [&normalized, &padded] {
        for engine in
            [BASE64_STANDARD, BASE64_STANDARD_NO_PAD, BASE64_URL_SAFE, BASE64_URL_SAFE_NO_PAD]
        {
            if let Ok(bytes) = engine.decode(candidate) {
                decoded = Some(bytes);
                break;
            }
        }
        if decoded.is_some() {
            break;
        }
    }
    let content = String::from_utf8(decoded.ok_or("invalid content Base64")?)
        .map_err(|_| "content must be UTF-8")?;
    validate_text(&name, &content)?;
    Ok(PromptSharePayload {
        name,
        content,
        description: fields.description,
        source_app: app,
        activation_suggested: fields.enabled.unwrap_or(false),
    })
}

fn validate_text(name: &str, content: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        return Err("prompt name must not be blank");
    }
    if content.trim().is_empty() {
        return Err("prompt content must not be blank");
    }
    Ok(())
}

/// Store input deliberately has no enabled flag or client-selected identity.
#[derive(Debug)]
pub struct DisabledSharedPrompt {
    pub id: String,
    pub app_id: AppId,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    /// Milliseconds since Unix epoch, matching the source and PromptEntry.
    pub timestamp_ms: i64,
}

#[derive(Debug)]
pub enum SharedImportError<E> {
    InvalidInput(&'static str),
    Save(E),
}

#[derive(Debug)]
pub enum SharedImportResult<E> {
    ImportedDisabled(PromptEntry),
    Activated(PromptEntry),
    /// The persisted disabled copy is retained and its ID permits retrying.
    ImportedActivationFailed {
        entry: PromptEntry,
        error: E,
    },
}

/// Saves a new disabled copy before calling activation. The save owner must use
/// create-only semantics, issue the revision, and preserve the supplied server
/// ID/time. Activation must enforce authorization, revision and rollback rules.
/// No default store or filesystem fallback is provided.
pub fn import<E>(
    request: ImportPromptShareRequest,
    save_disabled: impl FnOnce(DisabledSharedPrompt) -> Result<PromptEntry, E>,
    activate: impl FnOnce(ActivatePromptRequest) -> Result<PromptEntry, E>,
) -> Result<SharedImportResult<E>, SharedImportError<E>> {
    request.validate().map_err(|_| {
        SharedImportError::InvalidInput("confirmation and supported target required")
    })?;
    validate_text(&request.payload.name, &request.payload.content)
        .map_err(SharedImportError::InvalidInput)?;
    if request.target_app == AppId::Mcode && request.payload.content.len() > 32768 {
        return Err(SharedImportError::InvalidInput("MCode prompt exceeds 32768 UTF-8 bytes"));
    }
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or(SharedImportError::InvalidInput("invalid server clock"))?;
    let entry = save_disabled(DisabledSharedPrompt {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: request.target_app,
        name: request.payload.name,
        content: request.payload.content,
        description: request.payload.description,
        timestamp_ms,
    })
    .map_err(SharedImportError::Save)?;
    match request.activation {
        PromptActivationChoice::KeepDisabled => Ok(SharedImportResult::ImportedDisabled(entry)),
        PromptActivationChoice::Activate { expected_target_revision } => {
            let activation = ActivatePromptRequest {
                entry: PromptEntryRequest {
                    app_id: entry.app_id,
                    id: entry.id.clone(),
                    expected_revision: entry.revision.clone(),
                },
                expected_target_revision,
                confirmed: true,
            };
            match activate(activation) {
                Ok(active) => Ok(SharedImportResult::Activated(active)),
                Err(error) => Ok(SharedImportResult::ImportedActivationFailed { entry, error }),
            }
        }
    }
}
