//! 桌面应用入口模块，负责启动 iced 应用并注册窗口、字体、主题和渲染器配置。

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use vw_desktop::{app, fonts};

#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::prelude::*;

const MAIN_WINDOW_SIZE: iced::Size = iced::Size { width: 1720.0, height: 1000.0 };

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
            "info,iced_winit=error,iced_futures::subscription::tracker=error",
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn task_pet_initial_position(window_size: iced::Size, monitor_size: iced::Size) -> iced::Point {
    iced::Point::new((monitor_size.width - window_size.width - 80.0).max(24.0), 96.0)
}

#[cfg(not(target_arch = "wasm32"))]
fn task_pet_window_settings(size: iced::Size) -> iced::window::Settings {
    iced::window::Settings {
        size,
        min_size: Some(size),
        max_size: Some(size),
        position: iced::window::Position::SpecificWith(task_pet_initial_position),
        resizable: false,
        decorations: false,
        transparent: true,
        level: iced::window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
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
        let (task_pet_window_id, task_pet_window_task) =
            iced::window::open(task_pet_window_settings(app.task_pet_window_size()));

        app.register_window_ids(main_window_id, Some(task_pet_window_id));

        (
            app,
            iced::Task::batch([
                startup_task,
                main_window_task.map(|_| app::Message::None),
                task_pet_window_task.map(|_| app::Message::None),
            ]),
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
