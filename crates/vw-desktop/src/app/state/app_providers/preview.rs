use iced::{task, widget::text_editor::Content};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        editor_merge::{EditorChoice, EditorEdit, EditorResult, EditorSave, EditorView},
        operations::{ProviderOperationProgress, ProviderOperationRequest},
        preview::ProviderPreview,
    },
};

pub(crate) const SCROLL_ID: &str = "provider-preview";

#[derive(Clone)]
pub(crate) struct Conflict {
    pub paths: Vec<Vec<String>>,
    pub resolution: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    Conflict,
    Reload,
    Rejected,
}

#[derive(Clone, Default)]
pub(crate) struct Confirmation {
    pub editor: Option<EditorView>,
    pub conflict: Option<Conflict>,
}

impl Confirmation {
    pub fn request(
        &self,
        host: String,
        edits: Vec<EditorEdit>,
        choice: EditorChoice,
    ) -> Option<EditorSave> {
        let resolution = match (&self.conflict, choice) {
            (None, EditorChoice::Refuse) => None,
            (Some(conflict), EditorChoice::KeepMine | EditorChoice::KeepTheirs) => {
                Some(conflict.resolution.clone())
            }
            _ => return None,
        };
        Some(EditorSave {
            target_host_id: host,
            context: self.editor.as_ref()?.context.clone(),
            edits,
            choice,
            resolution,
        })
    }

    pub fn finish(
        &mut self,
        choice: EditorChoice,
        result: Result<EditorResult, u16>,
    ) -> SaveOutcome {
        match result {
            Ok(value) if value.saved => {
                *self = Self::default();
                SaveOutcome::Saved
            }
            Ok(value) if choice == EditorChoice::Refuse && !value.conflicts.is_empty() => {
                if let Some(resolution) = value.resolution {
                    self.conflict = Some(Conflict { paths: value.conflicts, resolution });
                    SaveOutcome::Conflict
                } else {
                    SaveOutcome::Rejected
                }
            }
            Ok(_) if choice != EditorChoice::Refuse => {
                *self = Self::default();
                SaveOutcome::Reload
            }
            Err(409) => {
                *self = Self::default();
                SaveOutcome::Reload
            }
            _ => SaveOutcome::Rejected,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Display {
    Native(ProviderPreview),
    Saved(serde_json::Value),
}

#[derive(Clone)]
pub(crate) struct Save {
    pub request: ProviderOperationRequest,
    pub choice: EditorChoice,
    pub progress: Option<ProviderOperationProgress>,
    pub cancel_requested: bool,
    pub waiting: bool,
}

#[derive(Clone)]
pub(crate) struct Form {
    pub id: String,
    pub name: String,
    pub draft: Content,
    pub view: Option<Display>,
    pub confirmation: Confirmation,
    pub reading: Option<task::Handle>,
    pub save: Option<Save>,
    pub notice: Option<String>,
}

impl Form {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            draft: Content::with_text("[]"),
            view: None,
            confirmation: Confirmation::default(),
            reading: None,
            save: None,
            notice: None,
        }
    }
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderPreviewForm([redacted])")
    }
}

pub(crate) fn available(app: AppId) -> bool {
    matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild)
}
