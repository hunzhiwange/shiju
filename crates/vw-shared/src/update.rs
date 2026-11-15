//! 应用自更新与桌面安装包的历史兼容入口。

pub use app_self_update::*;

pub use ::desktop_release;
pub use desktop_release::{DesktopInstaller, DesktopRelease, InstallerKind, UpdateSource};
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_release::{download_desktop_installer, fetch_desktop_release};
