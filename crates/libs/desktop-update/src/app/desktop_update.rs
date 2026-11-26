//! Desktop update selection and the exact release snapshot used for download.
use crate::automatic::{CONFIG_KEY, DownloadPolicy};
use desktop_release::{DesktopRelease, UpdateSource};

pub const UPDATE_CHECK_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(3 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SkippedRelease {
    pub source: UpdateSource,
    pub version: String,
}

#[derive(Debug)]
pub struct DesktopUpdate {
    pub source: Option<UpdateSource>,
    pub skipped: Option<SkippedRelease>,
    pub download_policy: DownloadPolicy,
    pub saving_download_policy: bool,
    download_policy_edited: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) automatic_probe_pending: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) automatic_download_active: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) automatic_retry_after: Option<std::time::Instant>,
    pub saving_skip: bool,
    pub initial_check_started: bool,
    pub background_request: Option<u64>,
    pub error: Option<String>,
    pub selected_by_user: bool,
    pub saving: bool,
    pub request: u64,
    pub checking: bool,
    pub busy: bool,
    pub download_abort: Option<iced::task::Handle>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) download_pause: Option<tokio::sync::watch::Sender<bool>>,
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
            skipped: config
                .get("app_update_skipped_release")
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            download_policy: read_download_policy(config),
            saving_download_policy: false,
            download_policy_edited: false,
            #[cfg(not(target_arch = "wasm32"))]
            automatic_probe_pending: false,
            #[cfg(not(target_arch = "wasm32"))]
            automatic_download_active: false,
            #[cfg(not(target_arch = "wasm32"))]
            automatic_retry_after: None,
            saving_skip: false,
            initial_check_started: false,
            background_request: None,
            error,
            selected_by_user: false,
            saving: false,
            request: 0,
            checking: false,
            busy: false,
            download_abort: None,
            #[cfg(not(target_arch = "wasm32"))]
            download_pause: None,
            release: None,
            #[cfg(not(target_arch = "wasm32"))]
            ready: None,
        }
    }

    pub fn notification_release(&self) -> Option<&DesktopRelease> {
        self.release.as_ref().filter(|release| {
            self.installable()
                && !self.skipped.as_ref().is_some_and(|skipped| {
                    skipped.source == release.source && skipped.version == release.version
                })
        })
    }

    pub fn installable(&self) -> bool {
        self.release.as_ref().is_some_and(|release| {
            Some(release.source) == self.source
                && release.is_newer_than(update_types::current_version()).unwrap_or(false)
        })
    }

    pub fn download_paused(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.download_pause.as_ref().is_some_and(|pause| *pause.borrow());
        #[cfg(target_arch = "wasm32")]
        false
    }
}

pub fn source_patch(source: UpdateSource) -> serde_json::Value {
    serde_json::json!({"app_update_source": source})
}

fn read_download_policy(cfg: &serde_json::Value) -> DownloadPolicy {
    cfg.get(CONFIG_KEY)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

impl DesktopUpdate {
    pub(crate) fn set_download_policy(&mut self, policy: DownloadPolicy) {
        self.download_policy = policy;
        self.download_policy_edited = true;
    }

    pub fn restore_preferences(&mut self, cfg: &serde_json::Value) {
        if !self.download_policy_edited {
            self.download_policy = read_download_policy(cfg);
        }
        if !self.selected_by_user
            && !self.checking
            && self.background_request.is_none()
            && !self.saving_skip
            && !self.busy
            && self.release.is_none()
        {
            let restored = DesktopUpdate::from_config(cfg);
            self.source = restored.source;
            self.error = restored.error;
            self.skipped = restored.skipped;
        }
    }
}
