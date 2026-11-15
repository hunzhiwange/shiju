pub use desktop_apps_ui::agent_apps_auth::browser::{LinkAction, accepts_paste, read_preference, save_preference};
use crate::app::state::agent_apps_auth::State;
fn preference_path() -> Option<std::path::PathBuf> {
    crate::app::project_dirs().map(|dirs| dirs.config_dir().join("auth-browser.json"))
}
pub(in crate::app) fn load_preference() -> bool {
    preference_path().is_none_or(|path| read_preference(&path))
}
pub(super) fn store_preference(open: bool) -> Result<(), &'static str> {
    const ERROR: &str = "登录浏览器偏好保存失败，本次会话仍会使用当前选择";
    let path = preference_path().ok_or(ERROR)?;
    save_preference(&path, open).map_err(|_| ERROR)
}
pub(super) fn edit_paste(state: &mut State, generation: u64, input: String) {
    desktop_apps_ui::agent_apps_auth::browser::edit_paste(state.login_mut(generation), input)
}
pub(super) fn submit_paste(state: &mut State, generation: u64) -> Result<(), &'static str> {
    desktop_apps_ui::agent_apps_auth::browser::submit_paste(state.login_mut(generation))
}

pub(super) fn deliver(action: LinkAction, url: &reqwest::Url) -> Result<(), &'static str> {
    action.deliver_using(url,
        |url| open::that(url).is_ok(),
        |url| arboard::Clipboard::new().and_then(|mut board| board.set_text(url)).is_ok(),
    )
}
