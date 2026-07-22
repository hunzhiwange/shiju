#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

use iced::font::Family;

use super::*;

#[cfg(not(target_arch = "wasm32"))]
static RUST_LOG_ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(not(target_arch = "wasm32"))]
struct RustLogGuard {
    previous: Option<OsString>,
}

#[cfg(not(target_arch = "wasm32"))]
impl RustLogGuard {
    fn set(value: &str) -> Self {
        let previous = std::env::var_os("RUST_LOG");
        unsafe { std::env::set_var("RUST_LOG", value) };
        Self { previous }
    }

    fn remove() -> Self {
        let previous = std::env::var_os("RUST_LOG");
        unsafe { std::env::remove_var("RUST_LOG") };
        Self { previous }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for RustLogGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var("RUST_LOG", value) },
            None => unsafe { std::env::remove_var("RUST_LOG") },
        }
    }
}

#[test]
fn main_window_settings_use_expected_desktop_defaults() {
    let settings = main_window_settings();

    assert_eq!(settings.size, MAIN_WINDOW_SIZE);
    assert_eq!(settings.maximized, cfg!(not(target_arch = "wasm32")));
    assert!(matches!(settings.position, iced::window::Position::Centered));
    assert!(!settings.exit_on_close_request);
}

#[cfg(target_arch = "wasm32")]
#[test]
fn main_window_settings_target_expected_dom_element_on_wasm() {
    let settings = main_window_settings();

    assert_eq!(settings.platform_specific.target.as_deref(), Some("vibe-window-app-container"));
}

#[cfg(target_os = "macos")]
#[test]
fn main_window_platform_settings_enable_macos_chrome_integration() {
    let settings = main_window_platform_settings();

    assert!(settings.title_hidden);
    assert!(settings.titlebar_transparent);
    assert!(settings.fullsize_content_view);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn main_window_platform_settings_keep_non_macos_defaults() {
    let settings = main_window_platform_settings();
    let default_settings = iced::window::settings::PlatformSpecific::default();

    assert_eq!(settings, default_settings);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_initial_position_prefers_right_edge_with_margin() {
    let window_size = iced::Size::new(260.0, 180.0);
    let monitor_size = iced::Size::new(1920.0, 1080.0);

    let position = task_pet_initial_position(window_size, monitor_size);

    assert_eq!(position, iced::Point::new(1580.0, 96.0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_initial_position_clamps_to_left_margin() {
    let window_size = iced::Size::new(1200.0, 180.0);
    let monitor_size = iced::Size::new(1000.0, 768.0);

    let position = task_pet_initial_position(window_size, monitor_size);

    assert_eq!(position, iced::Point::new(24.0, 96.0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_window_settings_are_fixed_transparent_overlay() {
    let size = iced::Size::new(240.0, 160.0);

    let settings = task_pet_window_settings(size);

    assert_eq!(settings.size, size);
    assert_eq!(settings.min_size, Some(size));
    assert_eq!(settings.max_size, Some(size));
    assert!(matches!(settings.position, iced::window::Position::SpecificWith(_)));
    assert!(!settings.resizable);
    assert!(!settings.decorations);
    assert!(settings.transparent);
    assert_eq!(settings.level, iced::window::Level::AlwaysOnTop);
    assert!(!settings.exit_on_close_request);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_window_position_callback_uses_initial_position_policy() {
    let window_size = iced::Size::new(240.0, 160.0);
    let monitor_size = iced::Size::new(1440.0, 900.0);
    let settings = task_pet_window_settings(window_size);
    let iced::window::Position::SpecificWith(position) = settings.position else {
        panic!("task pet window should use a position callback");
    };

    assert_eq!(
        position(window_size, monitor_size),
        task_pet_initial_position(window_size, monitor_size)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_tracing_filter_uses_safe_defaults_when_rust_log_is_absent() {
    let _lock = RUST_LOG_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _rust_log = RustLogGuard::remove();
    let expected = tracing_subscriber::EnvFilter::new(
        "info,iced_winit=error,iced_futures::subscription::tracker=error",
    );

    assert_eq!(native_tracing_filter().to_string(), expected.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_tracing_filter_honors_valid_rust_log_directives() {
    let _lock = RUST_LOG_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _rust_log = RustLogGuard::set("vw_desktop=trace");
    let expected = tracing_subscriber::EnvFilter::new("vw_desktop=trace");

    assert_eq!(native_tracing_filter().to_string(), expected.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_tracing_filter_uses_safe_defaults_for_invalid_rust_log() {
    let _lock = RUST_LOG_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _rust_log = RustLogGuard::set("vw_desktop=not-a-level");
    let expected = tracing_subscriber::EnvFilter::new(
        "info,iced_winit=error,iced_futures::subscription::tracker=error",
    );

    assert_eq!(native_tracing_filter().to_string(), expected.to_string());
}

#[test]
fn default_font_uses_platform_family_name() {
    let expected = if cfg!(target_os = "macos") { "PingFang SC" } else { "JetBrains Mono" };

    assert_eq!(default_font().family, Family::Name(expected));
}

#[test]
fn iced_settings_include_default_font_and_embedded_fonts() {
    let settings = iced_settings();

    assert_eq!(settings.default_font, default_font());
    assert!(!settings.fonts.is_empty());
    assert!(settings.antialiasing);
    assert!(settings.vsync);
}
