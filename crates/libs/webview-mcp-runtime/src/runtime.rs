//! Dedicated ephemeral MCP Apps surface; ordinary browser navigation is never enabled.
use crate::document::{Document, read_document, revoke_document};
use serde_json::{Value, json};
use std::cell::Cell;
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use webview_mcp_policy::{
    APP_URL, ENGINE_WRAPPER_URL, MAX_MESSAGE_BYTES, WRAPPER_URL,
    frame_origin_allowed, internal_navigation_allowed,
};
use wry::{WebViewBuilder, http};

#[cfg(all(not(coverage), unix))]
use webview_mcp_form as form;

pub(crate) enum AppEvent {
    Deliver(Value),
    PopupBlocked,
    Close,
}

/// Load host supplied HTML from the pipe, without granting a local-directory protocol.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Document {
        nonce,
        policy,
        form_origins,
        form_enabled,
        app_csp,
        wrapper_csp,
        wrapper,
        app,
        bootstrap,
    } = read_document()?;
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let popup_proxy = proxy.clone();
    let active = Arc::new(AtomicBool::new(true));
    crate::stdin_bridge::spawn(proxy, active.clone());
    let window = WindowBuilder::new().with_title("ShiJu MCP App").build(&event_loop)?;
    let submitted = Rc::new(Cell::new(false));
    let ipc_submitted = submitted.clone();
    let ipc_active = active.clone();
    let builder = WebViewBuilder::new()
        .with_custom_protocol("mcp-app".into(), move |_, request| {
            let uri = request.uri();
            let resource = if request.method() == http::Method::GET {
                match uri.to_string().as_str() {
                    WRAPPER_URL => Some((wrapper.clone(), wrapper_csp.as_str())),
                    APP_URL => app
                        .lock()
                        .expect("app resource lock")
                        .take()
                        .map(|html| (html, app_csp.as_str())),
                    _ => None,
                }
            } else {
                None
            };
            let (status, body, csp) = match resource {
                Some((body, csp)) => (200, body, csp),
                None => (403, Vec::new(), "default-src 'none'"),
            };
            http::Response::builder()
                .status(status)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Content-Security-Policy", csp)
                .header("Referrer-Policy", "no-referrer")
                .header("Cache-Control", "no-store")
                .body(std::borrow::Cow::Owned(body))
                .expect("validated response")
        })
        .with_initialization_script_for_main_only(bootstrap, false)
        .with_incognito(true)
        .with_devtools(false)
        .with_clipboard(false)
        .with_new_window_req_handler(move |_, _| {
            let _ = popup_proxy.send_event(AppEvent::PopupBlocked);
            wry::NewWindowResponse::Deny
        })
        .with_ipc_handler(move |request| {
            if !ipc_active.load(Ordering::Acquire)
                || ipc_submitted.get()
                || request.uri() != ENGINE_WRAPPER_URL
                || request.body().len() > MAX_MESSAGE_BYTES
            {
                return;
            }
            let Ok(envelope) = serde_json::from_str::<Value>(request.body()) else {
                return;
            };
            if envelope["nonce"].as_str() != Some(nonce.as_str()) {
                return;
            }
            if envelope["closed"] == true {
                revoke_document();
                return;
            }
            if let Some(directive) = envelope["blocked"].as_str() {
                if matches!(
                    directive,
                    "base-uri"
                        | "connect-src"
                        | "default-src"
                        | "font-src"
                        | "form-action"
                        | "frame-src"
                        | "img-src"
                        | "media-src"
                        | "object-src"
                        | "script-src"
                        | "script-src-elem"
                        | "style-src"
                        | "style-src-attr"
                        | "style-src-elem"
                ) {
                    eprintln!("MCP app CSP blocked {directive}");
                    if cfg!(target_os = "windows") && directive == "form-action" {
                        eprintln!("Native MCP app forms are unsupported on Windows");
                    }
                }
                return;
            }
            let mut stdout = io::stdout().lock();
            let message = json!({"origin":"null","message":envelope["message"]});
            let line = format!("mcp-app {message}\n");
            if line.len() <= MAX_MESSAGE_BYTES {
                let _ = stdout.write_all(line.as_bytes());
                let _ = stdout.flush();
            }
        });
    // Install the native form policy before any application HTML can run.
    let builder = if form_enabled {
        builder.with_download_started_handler(|_, _| false)
    } else {
        let frame_domains = policy.frame_domains.clone();
        builder.with_url(WRAPPER_URL).with_navigation_handler(move |url| {
            internal_navigation_allowed(&url)
                // The wrapper's CSP only permits internal direct children.
                || (!cfg!(target_os = "windows") && frame_origin_allowed(&frame_domains, &url))
        })
    };
    let webview = builder.build(&window)?;
    #[cfg(all(not(coverage), unix))]
    let _form_delegate = if form_enabled {
        let delegate = form::install(
            &webview,
            form::FormPolicy::new(
                webview_external_protocol::main_dialog_parent(&window),
                policy.frame_domains,
                form_origins,
                active.clone(),
                submitted.clone(),
                revoke_document,
            ),
        )?;
        webview.load_url(WRAPPER_URL)?;
        Some(delegate)
    } else {
        None
    };
    event_loop.run(move |event, _, flow| {
        let _ = window.id();
        #[cfg(all(not(coverage), target_os = "macos"))]
        let _ = &_form_delegate;
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(AppEvent::Deliver(value))
                if active.load(Ordering::Acquire) && !submitted.get() =>
            {
                // JSON is encoded as a JS string, then parsed: no page text becomes source.
                if let Ok(encoded) = serde_json::to_string(&value.to_string()) {
                    let _ = webview
                        .evaluate_script(&format!("window.__mcpDeliver?.(JSON.parse({encoded}))"));
                }
            }
            Event::UserEvent(AppEvent::PopupBlocked) => {
                let _ = webview.evaluate_script("window.__mcpPopupBlocked?.()");
            }
            Event::UserEvent(AppEvent::Close)
            | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                active.store(false, Ordering::Release);
                *flow = ControlFlow::Exit
            }
            _ => {}
        }
    });
}
