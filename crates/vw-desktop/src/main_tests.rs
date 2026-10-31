//! 桌面程序入口配置的测试，覆盖窗口默认值、平台差异和日志过滤。
//! 环境变量守卫确保 RUST_LOG 用例互不污染，并在离开作用域后恢复现场。
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
fn native_tracing_filter_uses_safe_defaults_when_rust_log_is_absent() {
    let _lock = RUST_LOG_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _rust_log = RustLogGuard::remove();
    let expected = tracing_subscriber::EnvFilter::new(
        "info,rmcp=warn,iced_winit=error,iced_futures::subscription::tracker=error",
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
        "info,rmcp=warn,iced_winit=error,iced_futures::subscription::tracker=error",
    );

    assert_eq!(native_tracing_filter().to_string(), expected.to_string());
}

#[cfg(target_os = "macos")]
#[test]
fn native_stderr_filter_suppresses_macos_imk_run_loop_noise() {
    let line = "2026-07-22 23:35:59.262 vibe-window[98293:81530284] error messaging the mach port for IMKCFRunLoopWakeUpReliable";

    assert!(should_suppress_native_stderr_line(line));
}

#[cfg(target_os = "macos")]
#[test]
fn native_stderr_filter_keeps_unrelated_stderr_lines() {
    assert!(!should_suppress_native_stderr_line(
        "failed to create runtime log directory /tmp/vibe-window: permission denied",
    ));
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

#[test]
fn channel_qr_flow_stops_on_close_and_fills_confirmed_credentials() {
    let open = app::ChannelQrFlowSnapshot {
        channel: Some("qq".to_string()),
        generation: 7,
        poll_in_flight: true,
        credentials_ready: false,
        configured: false,
    };
    assert!(open.accepts_result("qq", 7));

    let closed = app::ChannelQrFlowSnapshot {
        channel: None,
        generation: 8,
        poll_in_flight: false,
        credentials_ready: false,
        configured: false,
    };
    assert!(!closed.accepts_result("qq", 7));

    let confirmed = app::ChannelQrFlowSnapshot {
        channel: Some("qq".to_string()),
        generation: 8,
        poll_in_flight: false,
        credentials_ready: true,
        configured: true,
    };
    assert!(confirmed.accepts_result("qq", 8));
    assert!(confirmed.credentials_ready);
    assert!(confirmed.configured);
}

#[cfg(windows)]
#[test]
fn windows_graphics_backend_forces_dx12_over_inherited_settings() {
    struct RestoreBackends(Option<OsString>, Option<OsString>);
    impl Drop for RestoreBackends {
        fn drop(&mut self) {
            for (name, previous) in [("WGPU_BACKEND", &self.0), ("ICED_BACKEND", &self.1)] {
                unsafe {
                    match previous {
                        Some(value) => std::env::set_var(name, value),
                        None => std::env::remove_var(name),
                    }
                }
            }
        }
    }
    let _restore =
        RestoreBackends(std::env::var_os("WGPU_BACKEND"), std::env::var_os("ICED_BACKEND"));
    for inherited in [None, Some(""), Some("gl"), Some("vulkan"), Some("all")] {
        unsafe {
            match inherited {
                Some(value) => std::env::set_var("WGPU_BACKEND", value),
                None => std::env::remove_var("WGPU_BACKEND"),
            }
            std::env::set_var("ICED_BACKEND", "tiny-skia");
        }
        configure_native_graphics_backend();
        assert_eq!(std::env::var("WGPU_BACKEND").as_deref(), Ok("dx12"));
        assert_eq!(std::env::var("ICED_BACKEND").as_deref(), Ok("wgpu"));
    }
}
