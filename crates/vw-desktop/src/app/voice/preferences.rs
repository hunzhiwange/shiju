//! Desktop path selection for device-local voice preferences.
pub use desktop_voice::preferences::*;
use std::path::PathBuf;

pub(crate) fn path() -> Result<PathBuf, String> {
    crate::app::project_dirs()
        .map(|d| d.config_dir().join("voice-preferences.json"))
        .ok_or_else(|| "无法定位本机语音设置。".into())
}
pub(crate) fn load() -> Result<Preferences, String> {
    Preferences::read(&path()?)
}
