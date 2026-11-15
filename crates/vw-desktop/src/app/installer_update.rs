//! Verified installer API owned by desktop-update.
pub use desktop_update::app::installer_update::{
    PreparedDesktopUpdate, open_desktop_installer, prepare_desktop_update,
    reveal_desktop_installer,
};

#[cfg(test)]
#[path = "installer_update_tests.rs"]
mod installer_update_tests;

#[cfg(test)]
#[path = "desktop_integration_49_tests.rs"]
mod desktop_integration_49_tests;
