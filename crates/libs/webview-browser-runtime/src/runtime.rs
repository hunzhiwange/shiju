use webview_external_protocol::dialog::show_error_dialog;
use webview_browser_views::{build_content_view, build_toolbar_view};

use webview_local_content::{
    StartTarget, normalize_url, percent_encode_url_path, resolve_target,
};
use webview_launch::{DEFAULT_TARGET, launch_log_summary, parse_launch_options};
use webview_core::events::UserEvent;
use webview_init_script::build_init_script;

use webview_storage::{append_log, webview_data_dir};

use webview_runtime_cookie::{
    read_shiju_app_set_cookie_for_target, runtime_cookie, runtime_cookie_headers,
};

#[cfg(unix)]
const DEFAULT_TITLE: &str = "Vibe Window WebView";
pub(crate) const SAFARI_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Safari/605.1.15";

pub fn run(args: Vec<String>) {
    #[cfg(all(unix, not(target_os = "macos")))]
    use gtk::prelude::*;
    use std::io;
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    #[cfg(all(unix, not(target_os = "macos")))]
    use tao::platform::unix::WindowExtUnix;
    use wry::WebContext;

    use webview_external_protocol::ExternalProtocols;
    use webview_toolbar::window_regions;

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
    crate::control_pipe::spawn(&proxy, embedded_mode);

    let wb = crate::launch::window_builder(&options, embedded_mode);

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

    let toolbar_webview = if let Some(region) = toolbar_region {
        let toolbar_result = build_toolbar_view(
            &window, region, options.shiju_app_set_cookie_stdin, &proxy,
        );

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
    let webview_result = build_content_view(
        &window,
        &mut web_context,
        initial_url,
        &init_script,
        #[cfg(target_os = "macos")]
        SAFARI_USER_AGENT,
        #[cfg(unix)]
        &log_path,
        local_root.clone(),
        independent_mode,
        content_region,
        &proxy,
        &popups,
    );
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

    let mut events = crate::window_events::BrowserEvents {
        window, webview, toolbar_webview, popups, external, proxy,
        embedded_mode, independent_mode, current_url: url, browser_cleared: false,
    };
    event_loop.run(move |event, _, control_flow| {
        let _web_context = &web_context;
        // Keep the weakly referenced WebKit delegate alive for the event loop.
        #[cfg(target_os = "macos")]
        let _attachment_delegate = &attachment_delegate;
        if *control_flow != ControlFlow::Exit {
            *control_flow = ControlFlow::Wait;
        }

        events.handle(event, control_flow);
    });
}
