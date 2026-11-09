//! Dedicated ephemeral MCP Apps surface; ordinary browser navigation is never enabled.
use serde_json::{Value, json};
use std::cell::Cell;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use url::Url;
use vw_api_types::mcp_app_network::McpAppNetworkPolicy;
use wry::{WebViewBuilder, http};

#[path = "mcp_app_form.rs"]
mod form;

enum AppEvent {
    Deliver(Value),
    PopupBlocked,
    Close,
}

// A line distinct from the page's RPC envelope, emitted only by trusted helper code.
pub(super) fn revoke_document() {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "mcp-app-closed").and_then(|_| stdout.flush());
}

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const WRAPPER_URL: &str = "mcp-app://localhost/index.html";
const APP_URL: &str = "mcp-app://localhost/app.html";
#[cfg(target_os = "windows")]
const ENGINE_WRAPPER_URL: &str = "http://mcp-app.localhost/index.html";
#[cfg(not(target_os = "windows"))]
const ENGINE_WRAPPER_URL: &str = WRAPPER_URL;
#[cfg(target_os = "windows")]
const ENGINE_APP_URL: &str = "http://mcp-app.localhost/app.html";
#[cfg(not(target_os = "windows"))]
const ENGINE_APP_URL: &str = APP_URL;

const WRAPPER_CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; frame-src 'self'; base-uri 'none'; form-action 'none'";

/// Load host supplied HTML from the pipe, without granting a local-directory protocol.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut line = String::new();
    std::io::Read::take(io::stdin().lock(), 8 * 1024 * 1024 + 1).read_line(&mut line)?;
    if line.len() > 8 * 1024 * 1024 {
        return Err("MCP app resource too large".into());
    }
    let input: Value = serde_json::from_str(&line)?;
    let nonce = input["nonce"]
        .as_str()
        .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("missing host nonce")?
        .to_owned();
    let html = input["html"].as_str().ok_or("missing app HTML")?;
    let policy = McpAppNetworkPolicy::parse(input.get("csp"))?;
    let form_origins = form::allowed_origins(&policy);
    let form_enabled = !form_origins.is_empty();
    let app_csp = app_csp(&policy);
    let wrapper_csp = wrapper_csp(&form_origins);
    let document_nonce = format!("{:016x}{:016x}", getrandom::u64()?, getrandom::u64()?);
    let wrapper = render_wrapper(&nonce, &document_nonce, form_enabled).into_bytes();
    let app = Mutex::new(Some(html.as_bytes().to_vec()));
    let bootstrap = format!(
        "(() => {{
            if (window === top || parent !== top || location.href !== '{ENGINE_APP_URL}') return;
            const documentNonce = '{document_nonce}';
            {}
        }})();",
        include_str!("mcp_app_document.js"),
    );
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let popup_proxy = proxy.clone();
    let active = Arc::new(AtomicBool::new(true));
    let pipe_active = active.clone();
    std::thread::spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            let mut line = String::new();
            match std::io::Read::take(&mut input, (MAX_MESSAGE_BYTES + 1) as u64)
                .read_line(&mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(_) if line.len() > MAX_MESSAGE_BYTES => break,
                Ok(_) => {}
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line == "close" {
                break;
            }
            if let Ok(value) = serde_json::from_str(line) {
                let _ = proxy.send_event(AppEvent::Deliver(value));
            }
        }
        pipe_active.store(false, Ordering::Release);
        let _ = proxy.send_event(AppEvent::Close);
    });
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
            form::FormPolicy {
                parent: crate::external_protocol::main_dialog_parent(&window),
                frame_domains: policy.frame_domains,
                origins: form_origins,
                active: active.clone(),
                submitted: submitted.clone(),
                pending: Rc::new(Cell::new(false)),
            },
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

fn render_wrapper(host_nonce: &str, document_nonce: &str, form_enabled: bool) -> String {
    let forms = if form_enabled { " allow-forms" } else { "" };
    format!(
        r#"<!doctype html>
<meta charset="utf-8">
<style>
html, body, iframe {{ margin: 0; width: 100%; height: 100%; border: 0; }}
::-webkit-scrollbar {{ width: 4px; }}
:root {{ color-scheme: light dark; }}
#link-status {{ position: fixed; inset: auto 12px 12px; padding: 8px 12px; border-radius: 8px;
    color: CanvasText; background: Canvas; border: 1px solid GrayText; font: menu; pointer-events: none; }}
</style>
<iframe sandbox="allow-scripts{forms}"></iframe>
<div id="link-status" role="status" hidden></div>
<script>
(() => {{
    const hostNonce = "{host_nonce}";
    const documentNonce = "{document_nonce}";
    const maxMessageBytes = {MAX_MESSAGE_BYTES};
    {}
}})();
</script>"#,
        include_str!("mcp_app_host.js"),
    )
}

fn app_csp(policy: &McpAppNetworkPolicy) -> String {
    fn sources(origins: &[String]) -> String {
        if origins.is_empty() { "'none'".into() } else { origins.join(" ") }
    }
    let resources = policy.resource_domains.join(" ");
    let connect = sources(&policy.connect_domains);
    let frame = sources(&policy.frame_domains);
    let base = sources(&policy.base_uri_domains);
    let media = sources(&policy.resource_domains);
    let forms = sources(&form::allowed_origins(policy));
    format!(
        "default-src 'none'; script-src 'unsafe-inline' {resources}; style-src 'unsafe-inline' {resources}; font-src data: {resources}; img-src data: {resources}; media-src {media}; connect-src {connect}; frame-src {frame}; base-uri {base}; object-src 'none'; form-action {forms}"
    )
}

fn wrapper_csp(origins: &[String]) -> String {
    if origins.is_empty() {
        return WRAPPER_CSP.into();
    }
    WRAPPER_CSP.replace("frame-src 'self';", &format!("frame-src 'self' {};", origins.join(" ")))
}

fn internal_navigation_allowed(url: &str) -> bool {
    matches!(url, WRAPPER_URL | APP_URL | "about:blank" | "about:srcdoc")
        || url == ENGINE_WRAPPER_URL
        || url == ENGINE_APP_URL
}

fn https_origin(target: &str) -> Option<String> {
    let url = Url::parse(target).ok()?;
    (url.scheme() == "https" && url.username().is_empty() && url.password().is_none())
        .then(|| url.origin().ascii_serialization())
}

fn frame_origin_allowed(domains: &[String], target: &str) -> bool {
    https_origin(target).is_some_and(|origin| domains.contains(&origin))
}

#[cfg(test)]
#[path = "mcp_app_tests.rs"]
mod tests;
