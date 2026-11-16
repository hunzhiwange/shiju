pub mod ui_types;
pub mod components {
 pub mod tab_bar;
 pub mod top_bar;
 pub mod about_modal;
 pub mod install_cli_modal;
 mod install_cli_modal_progress;
 pub mod cli_command_help;
 pub mod message;
 #[cfg(not(target_arch="wasm32"))] pub mod install_web_modal;
 pub mod model_hover_tooltip;
 pub mod notification;
 mod notification_styles;
 pub mod startup_transition;
 #[cfg(test)] mod model_hover_tooltip_tests;
 #[cfg(test)] mod install_cli_modal_tests;
 #[cfg(test)] mod notification_tests;
 #[cfg(test)] mod startup_transition_tests;
}

pub mod message { pub mod notification; }

pub mod presentation;
pub mod views { pub mod home; }

pub mod app_init;

pub mod view_layout;

pub mod panel;

pub mod status_bar;
pub mod external_apps_update;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_03_tests;

#[cfg(test)]
mod desktop_split_18_tests;

pub mod window_lifecycle;

#[cfg(test)]
mod desktop_split_30_tests;

#[cfg(test)]
mod host_module_15_tests;

pub mod mobile_layout;

/// Explicit native acceptance fixtures.
pub mod acceptance;

#[cfg(test)]
mod host_module_40_tests;

#[cfg(test)]
mod host_module_38_tests;

#[cfg(test)]
mod host_module_31_tests;
