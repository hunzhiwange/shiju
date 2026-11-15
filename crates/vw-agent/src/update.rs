//! 重新导出共享更新模块的公开接口。
//! 代理侧通过该薄层保持历史导入路径稳定，实际更新逻辑集中在共享 crate。

pub use update_types::{UpdateProgress, current_version};
pub use vw_shared::update::{
    DesktopInstaller, DesktopRelease, InstallerKind, UpdateSource, check_for_update,
    desktop_release, desktop_self_update, desktop_self_update_with_progress, fetch_latest_version,
    self_update,
};
#[cfg(not(target_arch = "wasm32"))]
pub use vw_shared::update::{download_desktop_installer, fetch_desktop_release};
