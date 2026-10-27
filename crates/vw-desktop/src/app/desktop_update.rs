//! Desktop update selection and the exact release snapshot used for download.
use vw_shared::update::{DesktopRelease, UpdateSource};

#[derive(Debug)]
pub(crate) struct DesktopUpdate {
    pub source: Option<UpdateSource>,
    pub error: Option<String>,
    pub selected_by_user: bool,
    pub saving: bool,
    pub request: u64,
    pub checking: bool,
    pub busy: bool,
    pub release: Option<DesktopRelease>,
    #[cfg(not(target_arch = "wasm32"))]
    pub ready: Option<super::installer_update::PreparedDesktopUpdate>,
}

impl DesktopUpdate {
    pub fn from_config(config: &serde_json::Value) -> Self {
        let (source, error) = match config.get("app_update_source") {
            None => (Some(UpdateSource::Shiju), None),
            Some(value) => match serde_json::from_value(value.clone()) {
                Ok(source) => (Some(source), None),
                Err(_) => (None, Some("已保存的升级来源无效，请重新选择来源。".into())),
            },
        };
        Self {
            source,
            error,
            selected_by_user: false,
            saving: false,
            request: 0,
            checking: false,
            busy: false,
            release: None,
            #[cfg(not(target_arch = "wasm32"))]
            ready: None,
        }
    }

    pub fn installable(&self) -> bool {
        self.release.as_ref().is_some_and(|release| {
            Some(release.source) == self.source
                && release.is_newer_than(env!("CARGO_PKG_VERSION")).unwrap_or(false)
        })
    }
}

pub(crate) fn source_patch(source: UpdateSource) -> serde_json::Value {
    serde_json::json!({"app_update_source": source})
}
