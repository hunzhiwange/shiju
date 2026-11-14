//! Explicit imports create inert catalog data; they never project to native files.
use vw_api_types::agent_apps::{AppId, PromptContentSnapshot, PromptResource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportError {
    UnsupportedTarget,
    MissingFile,
    InvalidUtf8,
    InvalidName,
    ContentTooLarge,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnsupportedTarget => "unsupported prompt import target",
            Self::MissingFile => "prompt file does not exist",
            Self::InvalidUtf8 => "imported prompt is not valid UTF-8",
            Self::InvalidName => "imported prompt name is empty",
            Self::ContentTooLarge => "MCode prompt exceeds 32768 UTF-8 bytes",
        })
    }
}
impl std::error::Error for ImportError {}

/// No activation state or caller-selected ID. Persist with create-only semantics
/// in the authenticated project/host/application scope, comparing catalog revision.
#[derive(Debug)]
pub struct DisabledImport {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
}

/// Consume the exact snapshot read by the authorized host's live_file owner.
/// Empty files are valid imports; missing files are explicit failures.
pub fn from_snapshot(snapshot: PromptContentSnapshot) -> Result<DisabledImport, ImportError> {
    let PromptResource::Application { app_id } = snapshot.resource else {
        return Err(ImportError::UnsupportedTarget);
    };
    validate_app(app_id)?;
    prepare(
        app_id,
        "导入的提示词".to_owned(),
        snapshot.content.ok_or(ImportError::MissingFile)?,
        Some("从现有配置文件导入".to_owned()),
    )
}

/// Client-selected file bytes only. A client path must never reach the host
/// resolver. Decode strictly and preserve text, whitespace and line endings.
pub fn from_upload(
    app: AppId,
    name: String,
    content: Vec<u8>,
    description: Option<String>,
) -> Result<DisabledImport, ImportError> {
    validate_app(app)?;
    let content = String::from_utf8(content).map_err(|_| ImportError::InvalidUtf8)?;
    prepare(app, name, content, description)
}

fn validate_app(app: AppId) -> Result<(), ImportError> {
    PromptResource::Application { app_id: app }
        .validate_application_access()
        .map_err(|_| ImportError::UnsupportedTarget)
}

fn prepare(
    app: AppId,
    name: String,
    content: String,
    description: Option<String>,
) -> Result<DisabledImport, ImportError> {
    if name.trim().is_empty() {
        return Err(ImportError::InvalidName);
    }
    if app == AppId::Mcode && super::mcode::validate_content(&content).is_err() {
        return Err(ImportError::ContentTooLarge);
    }
    Ok(DisabledImport {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        content,
        description,
    })
}
