//! 桌面应用入口模块，负责启动 iced 应用并注册窗口、字体、主题和渲染器配置。

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod native_crash;

#[cfg(not(test))]
use vw_desktop::{app, fonts};
// Compile the same desktop modules in the explicit binary test target.
#[cfg(test)]
#[allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
#[cfg(test)]
#[allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod apps;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(dead_code, clippy::all, clippy::pedantic)]
mod desktop_update_helper;
#[cfg(test)]
#[allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod fonts;

#[cfg(target_os = "macos")]
use std::io::{BufRead, BufReader, Write};
#[cfg(target_os = "macos")]
use std::os::fd::FromRawFd;
#[cfg(target_os = "macos")]
use std::sync::Once;

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::prelude::*;

const MAIN_WINDOW_SIZE: iced::Size = iced::Size { width: 1720.0, height: 1000.0 };

#[cfg(target_os = "macos")]
const MACOS_IMK_RUN_LOOP_NOISE: &str =
    "error messaging the mach port for IMKCFRunLoopWakeUpReliable";

fn main_window_platform_settings() -> iced::window::settings::PlatformSpecific {
    #[cfg(target_os = "macos")]
    {
        iced::window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        iced::window::settings::PlatformSpecific::default()
    }
}

fn main_window_settings() -> iced::window::Settings {
    let window = iced::window::Settings {
        size: MAIN_WINDOW_SIZE,
        maximized: cfg!(not(target_arch = "wasm32")),
        position: iced::window::Position::Centered,
        platform_specific: main_window_platform_settings(),
        exit_on_close_request: false,
        ..Default::default()
    };

    #[cfg(target_arch = "wasm32")]
    let window = {
        let mut window = window;
        window.platform_specific.target = Some("vibe-window-app-container".to_string());
        window
    };

    window
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_native_graphics_backend() {
    #[cfg(windows)]
    {
        // 已确认 Mesa/GLES 会在 fence_sync 中原生崩溃，Windows 固定使用 DX12。
        // SAFETY: 入口第一步执行，此时尚未创建任何事件循环或工作线程。
        unsafe {
            std::env::set_var("ICED_BACKEND", "wgpu");
            std::env::set_var("WGPU_BACKEND", "dx12");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn init_native_tracing() {
    let terminal_filter = native_tracing_filter();
    let file_filter = native_tracing_filter();
    let runtime_log_dir = vw_shared::runtime_log::default_runtime_log_dir();
    if let Err(err) = vw_shared::runtime_log::ensure_runtime_log_dir(&runtime_log_dir) {
        eprintln!("failed to create runtime log directory {}: {err}", runtime_log_dir.display());
    }

    let terminal_layer = tracing_subscriber::fmt::layer().with_filter(terminal_filter);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
        .with_filter(file_filter);
    let subscriber = tracing_subscriber::registry().with(terminal_layer).with(file_layer);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "vw_desktop", "desktop panic: {info}");
        previous_hook(info);
    }));
    #[cfg(windows)]
    if let Err(error) = native_crash::install(&vw_shared::runtime_log::default_runtime_log_dir()) {
        tracing::warn!(target: "vw_desktop", %error, "failed to install native crash diagnostics");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_tracing_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "info,rmcp=warn,iced_winit=error,iced_futures::subscription::tracker=error",
        )
    })
}

#[cfg(target_os = "macos")]
fn should_suppress_native_stderr_line(line: &str) -> bool {
    line.contains(MACOS_IMK_RUN_LOOP_NOISE)
}

#[cfg(target_os = "macos")]
fn install_macos_native_stderr_filter() {
    static INSTALL: Once = Once::new();

    INSTALL.call_once(|| {
        let mut pipe_fds = [-1; 2];
        let result = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
        if result != 0 {
            return;
        }

        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];
        let original_stderr = unsafe { libc::dup(libc::STDERR_FILENO) };
        if original_stderr < 0 {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
            return;
        }

        let dup_result = unsafe { libc::dup2(write_fd, libc::STDERR_FILENO) };
        unsafe { libc::close(write_fd) };
        if dup_result < 0 {
            unsafe {
                libc::close(read_fd);
                libc::close(original_stderr);
            }
            return;
        }

        std::thread::spawn(move || {
            let reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
            let mut writer = unsafe { std::fs::File::from_raw_fd(original_stderr) };

            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else {
                    break;
                };
                if should_suppress_native_stderr_line(&line) {
                    continue;
                }
                let _ = writeln!(writer, "{line}");
                let _ = writer.flush();
            }
        });
    });
}

fn default_font() -> iced::Font {
    #[cfg(target_os = "macos")]
    {
        iced::Font::with_name("PingFang SC")
    }

    #[cfg(not(target_os = "macos"))]
    {
        iced::Font::with_name("JetBrains Mono")
    }
}

fn iced_settings() -> iced::Settings {
    iced::Settings { default_font: default_font(), fonts: fonts::load_all(), ..Default::default() }
}

fn boot_app() -> (app::App, iced::Task<app::Message>) {
    let (mut app, startup_task) = app::App::new();
    let (main_window_id, main_window_task) = iced::window::open(main_window_settings());

    #[cfg(target_arch = "wasm32")]
    {
        app.register_window_ids(main_window_id, None);

        return (
            app,
            iced::Task::batch([startup_task, main_window_task.map(|_| app::Message::None)]),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.register_window_ids(main_window_id, None);

        (app, iced::Task::batch([startup_task, main_window_task.map(|_| app::Message::None)]))
    }
}

/// 启动桌面应用。
///
/// 返回 iced 运行结果，窗口或渲染器初始化失败时将错误交给进程入口处理。
pub fn main() -> iced::Result {
    #[cfg(not(target_arch = "wasm32"))]
    configure_native_graphics_backend();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(result) = vw_desktop::desktop_update_helper::run_if_requested() {
        if let Err(error) = result {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return Ok(());
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_log::init().expect("Initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_os = "macos")]
        install_macos_native_stderr_filter();

        init_native_tracing();
        #[cfg(windows)]
        tracing::info!(target: "vw_desktop", "Windows renderer forced: ICED_BACKEND=wgpu WGPU_BACKEND=dx12");
    }

    let result = iced::daemon(boot_app, app::App::update, app::App::view_window)
        .title(app::App::title_for_window)
        .theme(app::App::theme_for_window)
        .subscription(app::App::subscription)
        .settings(iced_settings())
        .run();
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = &result {
        tracing::error!(target: "vw_desktop", ?error, "desktop event loop failed");
    }
    result
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
