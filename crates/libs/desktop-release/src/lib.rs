//! Desktop release snapshots and verified installer downloads.

mod desktop_release;

pub use desktop_release::{
    DesktopInstaller, DesktopRelease, InstallerKind, SHIJU_MANIFEST, UpdateSource,
};
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_release::{
    download_desktop_installer, fetch_desktop_release, linux_installer_kind,
};
