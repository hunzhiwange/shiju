//! # VibeWindow WebView 应用入口
//!
//! 本模块是 VibeWindow WebView 应用的主入口点，提供跨平台的 WebView 窗口功能。

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
use webview_external_protocol::dialog::show_error_dialog;
#[cfg(not(target_arch = "wasm32"))]
mod view_region;

#[cfg(not(target_arch = "wasm32"))]
use webview_local_content::{
    StartTarget, local_protocol_response, normalize_url, percent_encode_url_path, resolve_target,
};
#[cfg(not(target_arch = "wasm32"))]
use webview_launch::{DEFAULT_TARGET, launch_log_summary, parse_launch_options};
#[cfg(not(target_arch = "wasm32"))]
use webview_core::events::UserEvent;
#[cfg(not(target_arch = "wasm32"))]
use webview_init_script::build_init_script;
#[cfg(not(target_arch = "wasm32"))]
use webview_ipc::{navigation_event_line, parse_navigation_ipc_message, parse_stdin_event};
#[cfg(not(target_arch = "wasm32"))]
use webview_core::navigation::is_safe_navigation_url;
#[cfg(not(target_arch = "wasm32"))]
use webview_storage::{append_log, webview_data_dir};

#[cfg(target_os = "macos")]
use tao::platform::macos::WindowBuilderExtMacOS;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowBuilderExtWindows;
#[cfg(not(target_arch = "wasm32"))]
use webview_runtime_cookie::{
    read_shiju_app_set_cookie_for_target, runtime_cookie, runtime_cookie_headers,
};

#[cfg(not(target_arch = "wasm32"))]
const SAFARI_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Safari/605.1.15";
#[cfg(not(target_arch = "wasm32"))]
fn emit_navigation_url(url: &str) {
    use std::io::Write;

    let Some(line) = navigation_event_line(url) else {
        return;
    };
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let _ = writeln!(output, "{line}");
    let _ = output.flush();
}

#[cfg(not(any(target_arch = "wasm32", coverage)))]
fn main() {
    #[cfg(all(unix, not(target_os = "macos")))]
    use gtk::prelude::*;
    use std::env;
    use std::io::{self, BufRead};
    use std::thread;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    #[cfg(all(unix, not(target_os = "macos")))]
    use tao::platform::unix::WindowExtUnix;
    use tao::window::WindowBuilder;
    use wry::{WebContext, WebViewBuilder};
    #[cfg(all(unix, not(target_os = "macos")))]
    use wry::{WebViewBuilderExtUnix, WebViewExtUnix};

    use webview_external_protocol::ExternalProtocols;
    use webview_core::navigation::ExternalTarget;
    use crate::view_region::to_wry_rect;
    use webview_toolbar::{
        ToolbarAction, parse_toolbar_ipc_message, toolbar_address_script, toolbar_html,
        window_regions,
    };

    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--mcp-app-stdin") {
        if let Err(error) = webview_mcp_runtime::run() {
            eprintln!("MCP app failed: {error}");
        }
        return;
    }
    let target = normalize_url(args.first().map(String::as_str).unwrap_or(DEFAULT_TARGET));
    let options = parse_launch_options(&args[1..]);
    let shiju_app_set_cookie = if options.shiju_app_set_cookie_stdin {
        let stdin = io::stdin();
        match read_shiju_app_set_cookie_for_target(&target, &mut stdin.lock()) {
            Ok(set_cookie) => Some(set_cookie),
            Err(error) => {
                show_error_dialog("VibeWindow WebView", &error.to_string());
                return;
            }
        }
    } else {
        None
    };
    let (url, local_root) = match resolve_target(&target) {
        StartTarget::Url(url) => (url, None),
        StartTarget::LocalDir { root, entry } => {
            let entry = percent_encode_url_path(&entry);
            (format!("vibe://localhost{entry}"), Some(root))
        }
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let embedded_mode = options.mode == "embedded";
    #[cfg(target_os = "macos")]
    webview_native_menu::install_edit_menu(proxy.clone(), &url);
    {
        let stdin_proxy = proxy.clone();
        thread::spawn(move || {
            let stdin = io::stdin();
            for line in stdin.lock().lines().map_while(Result::ok) {
                if let Some(event) = parse_stdin_event(&line) {
                    let _ = stdin_proxy.send_event(event);
                }
            }
            // The desktop owns the embedded control pipe. Closing it ends the view's lifetime.
            if embedded_mode {
                let _ = stdin_proxy.send_event(UserEvent::Close);
            }
        });
    }

    let mut wb = WindowBuilder::new().with_title(options.window_title.clone());
    if embedded_mode {
        wb = wb.with_decorations(false).with_always_on_top(true);
        #[cfg(target_os = "macos")]
        {
            wb = wb.with_has_shadow(false);
        }
        #[cfg(target_os = "windows")]
        {
            wb = wb.with_undecorated_shadow(false);
        }
    }

    let size_w = options.width.unwrap_or(1280) as f64;
    let size_h = options.height.unwrap_or(800) as f64;
    wb = wb.with_inner_size(tao::dpi::LogicalSize::new(size_w, size_h));
    if let (Some(px), Some(py)) = (options.pos_x, options.pos_y) {
        wb = wb.with_position(tao::dpi::LogicalPosition::new(px as f64, py as f64));
    }

    let data_path = webview_data_dir();
    let log_path = data_path.join("vw-webview.log");
    {
        let log_path_for_panic = log_path.clone();
        std::panic::set_hook(Box::new(move |info| {
            append_log(&log_path_for_panic, &format!("panic: {info}"));
        }));
    }

    if let Err(e) = std::fs::create_dir_all(&data_path) {
        eprintln!("Failed to create data directory: {e}");
        append_log(&log_path, &format!("Failed to create data directory: {e}"));
    } else {
        println!("WebView data path: {data_path:?}");
        append_log(&log_path, &format!("WebView data path: {data_path:?}"));
    }
    append_log(&log_path, &launch_log_summary(&target, &options));

    let window = match wb.build(&event_loop) {
        Ok(w) => w,
        Err(e) => {
            let msg = format!("Failed to create window: {e:?}\nLog: {log_path:?}");
            append_log(&log_path, &msg);
            show_error_dialog("VibeWindow WebView", &msg);
            return;
        }
    };

    let init_script = match build_init_script(&data_path, &options.cookies_json, SAFARI_USER_AGENT)
    {
        Ok(script) => script,
        Err(e) => {
            let msg = format!("Failed to build initialization script: {e}\nLog: {log_path:?}");
            append_log(&log_path, &msg);
            show_error_dialog("VibeWindow WebView", &msg);
            return;
        }
    };

    let independent_mode = !embedded_mode;
    let logical_size = window.inner_size().to_logical::<u32>(window.scale_factor());
    let (toolbar_region, content_region) =
        window_regions(logical_size.width, logical_size.height, independent_mode);

    #[cfg(all(unix, not(target_os = "macos")))]
    let gtk_container = window.default_vbox();

    let toolbar_webview = if let Some(region) = toolbar_region {
        let toolbar_proxy = proxy.clone();
        let toolbar_ready_proxy = proxy.clone();
        let toolbar_builder = WebViewBuilder::new()
            .with_html(if options.shiju_app_set_cookie_stdin {
                toolbar_html()
                    .replace("type=\"text\"", "type=\"text\" readonly")
                    .replace("post('navigate', address.value);", "")
            } else {
                toolbar_html()
            })
            .with_clipboard(true)
            .with_bounds(to_wry_rect(region))
            .with_on_page_load_handler(move |event, _url| {
                if matches!(event, wry::PageLoadEvent::Finished) {
                    let _ = toolbar_ready_proxy.send_event(UserEvent::ToolbarReady);
                }
            })
            .with_ipc_handler(move |request| {
                let Some(action) = parse_toolbar_ipc_message(request.body()) else {
                    return;
                };
                let event = match action {
                    ToolbarAction::Back => UserEvent::Back,
                    ToolbarAction::Forward => UserEvent::Forward,
                    ToolbarAction::Navigate(url) => UserEvent::OpenUrl(url),
                    ToolbarAction::Reload => UserEvent::Reload,
                    ToolbarAction::OpenExternal(url) => UserEvent::OpenExternal(url),
                    ToolbarAction::Clear => UserEvent::ClearBrowser,
                };
                let _ = toolbar_proxy.send_event(event);
            });

        #[cfg(all(unix, not(target_os = "macos")))]
        let toolbar_result =
            gtk_container.ok_or(wry::Error::UnsupportedWindowHandle).and_then(|container| {
                // GTK must manage input focus for both views. Wrapping independent
                // X11 child windows leaves the address bar without keyboard input.
                let toolbar = toolbar_builder.build_gtk(container)?;
                let native = toolbar.webview();
                native.set_size_request(-1, region.height as i32);
                container.set_child_packing(&native, false, true, 0, gtk::PackType::Start);
                Ok(toolbar)
            });
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        let toolbar_result = toolbar_builder.build_as_child(&window);

        let toolbar = match toolbar_result {
            Ok(toolbar) => toolbar,
            Err(error) => {
                let msg = format!("WebView toolbar build error: {error:?}\nLog: {log_path:?}");
                append_log(&log_path, &msg);
                show_error_dialog("VibeWindow WebView", &msg);
                return;
            }
        };
        Some(toolbar)
    } else {
        None
    };

    let mut web_context = WebContext::new(Some(data_path.clone()));

    let initial_url = shiju_app_set_cookie.as_ref().map_or(url.as_str(), |_| DEFAULT_TARGET);

    let mut builder = {
        let page_load_proxy = proxy.clone();
        let navigation_proxy = proxy.clone();
        WebViewBuilder::new_with_web_context(&mut web_context)
            .with_url(initial_url)
            .with_devtools(true)
            .with_clipboard(true)
            .with_accept_first_mouse(true)
            .with_initialization_script(&init_script)
            .with_on_page_load_handler(move |_event, url| {
                let _ = page_load_proxy.send_event(UserEvent::NavigationChanged(url));
            })
            .with_ipc_handler(move |request| {
                if let Some(url) = parse_navigation_ipc_message(request.body()) {
                    let _ = navigation_proxy.send_event(UserEvent::NavigationChanged(url));
                }
            })
    };

    #[cfg(target_os = "macos")]
    {
        builder = builder.with_user_agent(SAFARI_USER_AGENT);
    }
    #[cfg(unix)]
    {
        // WebKitGTK shares this download handler with related popup views.
        builder = webview_downloads::with_downloads(builder, &log_path);
    }

    let external = ExternalProtocols::default();
    let popups = webview_popup::PopupWindows::new(
        proxy.clone(),
        log_path.clone(),
        external.clone(),
        #[cfg(unix)]
        DEFAULT_TITLE,
        #[cfg(target_os = "macos")]
        SAFARI_USER_AGENT,
        #[cfg(windows)]
        &window,
    );
    builder = popups.with_handler(builder, None, local_root.is_some());

    if let Some(root) = local_root.clone() {
        builder = builder.with_custom_protocol("vibe".into(), move |_webview_id, request| {
            local_protocol_response(&root, request)
        });
    }

    if independent_mode {
        builder = builder.with_bounds(to_wry_rect(content_region));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    let webview_result = if independent_mode {
        gtk_container
            .ok_or(wry::Error::UnsupportedWindowHandle)
            .and_then(|container| builder.build_gtk(container))
    } else {
        builder.build(&window)
    };
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let webview_result =
        if independent_mode { builder.build_as_child(&window) } else { builder.build(&window) };
    let webview = match webview_result {
        Ok(w) => w,
        Err(e) => {
            let msg = format!(
                "WebView build error: {e:?}\nLog: {log_path:?}\nHint: install Microsoft Edge WebView2 Runtime (Evergreen x64)."
            );
            append_log(&log_path, &msg);
            show_error_dialog("VibeWindow WebView", &msg);
            return;
        }
    };

    #[cfg(windows)]
    if webview_external_protocol::block_native_external_launches(&webview, local_root.is_some()).is_err() {
        show_error_dialog("VibeWindow WebView", "无法安装外部协议保护。");
        return;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    webview_media_permissions::install(&webview, window.gtk_window().upcast_ref());

    #[cfg(target_os = "macos")]
    let attachment_delegate = webview_downloads::enable_attachments(&webview);

    if let Some(set_cookie) = shiju_app_set_cookie.as_deref() {
        let cookie = match runtime_cookie(set_cookie) {
            Ok(cookie) => cookie,
            Err(error) => {
                show_error_dialog("VibeWindow WebView", error);
                return;
            }
        };
        let headers = match runtime_cookie_headers(&cookie) {
            Ok(headers) => headers,
            Err(error) => {
                show_error_dialog("VibeWindow WebView", error);
                return;
            }
        };
        if let Err(error) = webview.set_cookie(&cookie) {
            let msg = format!("设置识矩应用浏览器会话失败: {error}");
            append_log(&log_path, &msg);
            show_error_dialog("VibeWindow WebView", &msg);
            return;
        }
        if let Err(error) = webview.load_url_with_headers(&url, headers) {
            let msg = format!("加载识矩应用失败: {error}");
            append_log(&log_path, &msg);
            show_error_dialog("VibeWindow WebView", &msg);
            return;
        }
    }

    let mut current_url = url;
    let mut browser_cleared = false;
    event_loop.run(move |event, _, control_flow| {
        // Keep the weakly referenced WebKit delegate alive for the event loop.
        #[cfg(target_os = "macos")]
        let _attachment_delegate = &attachment_delegate;
        if *control_flow != ControlFlow::Exit {
            *control_flow = ControlFlow::Wait;
        }

        match event {
            Event::UserEvent(UserEvent::Resize(w, h)) => {
                window.set_inner_size(tao::dpi::LogicalSize::new(w as f64, h as f64));
            }
            Event::UserEvent(UserEvent::Move(x, y)) => {
                window.set_outer_position(tao::dpi::LogicalPosition::new(x as f64, y as f64));
            }
            Event::UserEvent(UserEvent::OpenUrl(u)) => {
                if is_safe_navigation_url(&u) {
                    browser_cleared = false;
                    let _ = webview.load_url(&u);
                } else if let Some(target) = ExternalTarget::parse(&u) {
                    external.request(target, None, &proxy);
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(&current_url)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
            }
            Event::UserEvent(UserEvent::PopupClosed(id)) => popups.close(id),
            Event::UserEvent(UserEvent::OpenExternal(u)) => {
                if is_safe_navigation_url(&u) {
                    if open::that(u).is_err() {
                        webview_external_protocol::show_browser_open_error(&window);
                    }
                } else if let Some(target) = ExternalTarget::parse(&u) {
                    external.request(target, None, &proxy);
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(&current_url)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
            }
            Event::UserEvent(UserEvent::ConfirmExternal) => {
                if let Some((target, popup, confirmation)) = external.take() {
                    if let Some(id) = popup {
                        popups.show_external_confirmation(id, target, confirmation);
                    } else if let Some(parent) = webview_external_protocol::main_dialog_parent(&window) {
                        webview_external_protocol::show_confirmation(parent, target, confirmation);
                    }
                }
            }
            Event::UserEvent(UserEvent::NavigationChanged(u)) => {
                if browser_cleared {
                    current_url.clear();
                } else {
                    current_url.clone_from(&u);
                    if embedded_mode {
                        emit_navigation_url(&u);
                    }
                    if let Some(toolbar) = toolbar_webview.as_ref()
                        && let Ok(script) = toolbar_address_script(&u)
                    {
                        let _ = toolbar.evaluate_script(&script);
                    }
                }
                #[cfg(target_os = "macos")]
                webview_native_menu::set_current_url(&current_url);
            }
            Event::UserEvent(UserEvent::ToolbarReady) => {
                if let Some(toolbar) = toolbar_webview.as_ref()
                    && let Ok(script) = toolbar_address_script(&current_url)
                {
                    let _ = toolbar.evaluate_script(&script);
                }
            }
            Event::UserEvent(UserEvent::Back) => {
                if !browser_cleared {
                    let _ = webview.evaluate_script("history.back()");
                }
            }
            Event::UserEvent(UserEvent::Forward) => {
                if !browser_cleared {
                    let _ = webview.evaluate_script("history.forward()");
                }
            }
            Event::UserEvent(UserEvent::Reload) => {
                if !browser_cleared {
                    let _ = webview.reload();
                }
            }
            Event::UserEvent(UserEvent::SetAlwaysOnTop(always_on_top)) => {
                window.set_always_on_top(always_on_top);
            }
            Event::UserEvent(UserEvent::Show) => {
                window.set_visible(true);
            }
            Event::UserEvent(UserEvent::Hide) => {
                window.set_visible(false);
            }
            Event::UserEvent(UserEvent::Close) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::ClearBrowser) => {
                browser_cleared = true;
                current_url.clear();
                #[cfg(target_os = "macos")]
                webview_native_menu::set_current_url("");
                let _ = webview.load_url(DEFAULT_TARGET);
                if let Some(toolbar) = toolbar_webview.as_ref()
                    && let Ok(script) = toolbar_address_script("")
                {
                    let _ = toolbar.evaluate_script(&script);
                }
            }
            Event::WindowEvent { window_id, event: WindowEvent::Resized(size), .. }
                if window_id == window.id() && independent_mode =>
            {
                let size = size.to_logical::<u32>(window.scale_factor());
                let (toolbar_region, content_region) =
                    window_regions(size.width, size.height, true);
                if let (Some(toolbar), Some(toolbar_region)) =
                    (toolbar_webview.as_ref(), toolbar_region)
                {
                    let _ = toolbar.set_bounds(to_wry_rect(toolbar_region));
                }
                let _ = webview.set_bounds(to_wry_rect(content_region));
            }
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. }
                if window_id == window.id() =>
            {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}

#[cfg(any(target_arch = "wasm32", coverage))]
fn main() {}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;

#[cfg(test)]
#[path = "view_region_tests.rs"]
mod view_region_tests;
