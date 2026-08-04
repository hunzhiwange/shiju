//! 桌面应用入口模块，负责启动 iced 应用并注册窗口、字体、主题和渲染器配置。

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use vw_desktop::{app, fonts};

#[cfg(target_os = "macos")]
use std::io::{BufRead, BufReader, Write};
#[cfg(target_os = "macos")]
use std::os::fd::FromRawFd;
#[cfg(target_os = "macos")]
use std::sync::Once;

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::prelude::*;

const MAIN_WINDOW_SIZE: iced::Size = iced::Size { width: 1720.0, height: 1000.0 };

#[cfg(any(test, target_os = "macos"))]
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
}

#[cfg(not(target_arch = "wasm32"))]
fn native_tracing_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "info,rmcp=warn,iced_winit=error,iced_futures::subscription::tracker=error",
        )
    })
}

#[cfg(any(test, target_os = "macos"))]
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

        (
            app,
            iced::Task::batch([startup_task, main_window_task.map(|_| app::Message::None)]),
        )
    }
}

/// 启动桌面应用。
///
/// 返回 iced 运行结果，窗口或渲染器初始化失败时将错误交给进程入口处理。
pub fn main() -> iced::Result {
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
    }

    iced::daemon(boot_app, app::App::update, app::App::view_window)
        .title(app::App::title_for_window)
        .theme(app::App::theme_for_window)
        .subscription(app::App::subscription)
        .settings(iced_settings())
        .run()
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
