//! Client-side delivery of host-issued authorization links and scoped Claude input.

use desktop_settings_account::types::{Login, LoginPhase, LoginTarget, now};
use std::{
    io,
    path::Path,
};
use vw_api_app_auth::AuthPasteRequest;

#[derive(Clone, Copy)]
pub enum LinkAction {
    Open,
    Copy,
}

impl LinkAction {
    pub fn for_preference(copy_link: bool) -> Self {
        if copy_link { Self::Copy } else { Self::Open }
    }

    pub fn deliver_using(
        self,
        url: &reqwest::Url,
        open: impl FnOnce(&str) -> bool,
        copy: impl FnOnce(&str) -> bool,
    ) -> Result<(), &'static str> {
        match self {
            Self::Open => open(url.as_str()).then_some(()).ok_or("无法打开授权浏览器"),
            Self::Copy => copy(url.as_str()).then_some(()).ok_or("无法复制授权链接"),
        }
    }
}

pub fn read_preference(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<bool>(&bytes).ok())
        .unwrap_or(true)
}

pub fn save_preference(path: &Path, open: bool) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(file.as_file_mut(), &open)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

pub fn accepts_paste(login: &Login) -> bool {
    matches!(login.target, LoginTarget::Claude)
        && !login.paste_submitting
        && matches!(&login.phase, LoginPhase::Pending(challenge)
            if challenge.can_paste && challenge.expires_at.0 > now().0)
}

pub fn edit_paste(login: Option<&mut Login>, input: String) {
    if let Some(login) = login.filter(|login| accepts_paste(login)) {
        login.paste_input = input;
    }
}

pub fn submit_paste(login: Option<&mut Login>) -> Result<(), &'static str> {
    let Some(login) = login.filter(|login| accepts_paste(login)) else {
        return Err("当前登录流程不接受回填");
    };
    let pasted = std::mem::take(&mut login.paste_input);
    if pasted.trim().is_empty() || pasted.len() > 16 * 1024 {
        return Err("请粘贴有效的 Claude 授权码或回调地址");
    }
    login
        .paste_sender
        .try_send(AuthPasteRequest { pasted })
        .map_err(|_| "当前登录流程无法接收回填，请重试")?;
    login.paste_submitting = true;
    Ok(())
}
