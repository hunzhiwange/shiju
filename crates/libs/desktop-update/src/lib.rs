//! Desktop update selection, verified downloads and native installation.
pub mod app {
    pub mod desktop_update;
    #[cfg(not(target_arch = "wasm32"))]
    pub mod installer_update;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod desktop_update_helper;

pub mod cli_install;

pub mod panel;

pub mod notifications;

#[cfg(test)]
mod desktop_split_03_tests;

#[cfg(test)]
mod desktop_split_01_tests;

#[cfg(test)]
mod host_module_41_tests;
