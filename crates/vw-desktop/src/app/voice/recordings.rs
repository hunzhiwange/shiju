//! Desktop path selection for the local dictation archive.
pub(crate) use voice_recordings::{Entry, Store, pcm};
use std::path::PathBuf;

pub(crate) fn path() -> Result<PathBuf, String> {
    crate::app::project_dirs()
        .map(|d| d.data_local_dir().join("voice-recordings"))
        .ok_or_else(|| "无法定位本机录音目录。".into())
}

pub(crate) fn local() -> Result<Store, String> {
    Store::open(&path()?)
}
