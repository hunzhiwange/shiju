//! Terminal frontends for host-validated native resume plans.
#[cfg(target_os = "macos")]
pub mod custom;
#[cfg(target_os = "macos")]
pub mod apple;
#[cfg(target_os = "macos")]
pub mod ghostty_kitty;
#[cfg(target_os = "macos")]
pub mod wezterm_kaku;
#[cfg(target_os = "macos")]
pub mod otty;
#[cfg(target_os = "macos")]
pub mod alacritty;
#[cfg(target_os = "macos")]
pub mod warp;
