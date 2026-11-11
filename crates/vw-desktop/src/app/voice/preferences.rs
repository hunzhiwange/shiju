//! Physical device preferences stay on this computer, outside tenant configuration.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vw_config_types::voice::VoiceConfig;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub microphone: Option<String>,
    // These capabilities remain unavailable until their dedicated tasks are implemented.
    pub retain_recordings: bool,
}
impl Preferences {
    pub(crate) fn path() -> Result<PathBuf, String> {
        crate::app::project_dirs()
            .map(|d| d.config_dir().join("voice-preferences.json"))
            .ok_or_else(|| "无法定位本机语音设置。".into())
    }
    pub(crate) fn load() -> Result<Self, String> {
        Self::read(&Self::path()?)
    }
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "本机语音设置损坏，请重新保存设置。".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(_) => Err("无法读取本机语音设置。".into()),
        }
    }
    pub(crate) fn write(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("无法定位本机语音设置目录。")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法创建本机语音设置目录。")?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法保存本机语音设置。")?;
        serde_json::to_writer(file.as_file_mut(), self).map_err(|_| "无法保存本机语音设置。")?;
        file.as_file().sync_all().map_err(|_| "无法同步本机语音设置。")?;
        file.persist(path).map_err(|_| "无法替换本机语音设置。")?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Edit {
    pub config: VoiceConfig,
    pub local: Preferences,
    original: VoiceConfig,
    original_local: Preferences,
}
impl Edit {
    pub(crate) fn new(config: VoiceConfig, local: Preferences) -> Self {
        Self { original: config.clone(), original_local: local.clone(), config, local }
    }
    pub(crate) fn cancel(self) -> (VoiceConfig, Preferences) {
        (self.original, self.original_local)
    }
    pub(crate) fn patch(&self) -> Result<serde_json::Value, String> {
        self.config.validate().map_err(str::to_owned)?;
        let before = serde_json::to_value(&self.original).map_err(|e| e.to_string())?;
        let mut after = serde_json::to_value(&self.config).map_err(|e| e.to_string())?;
        after
            .as_object_mut()
            .ok_or("invalid voice settings")?
            .retain(|key, value| before.get(key) != Some(value));
        Ok(after)
    }
}
#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
