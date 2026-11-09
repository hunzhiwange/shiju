//! Dedicated ephemeral MCP Apps surface; ordinary browser navigation is never enabled.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::Mutex;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use url::Url;
use vw_api_types::mcp_app_network::McpAppNetworkPolicy;
use wry::{WebViewBuilder, http};

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
    let app_csp = app_csp(&policy);
    let document_nonce = format!("{:016x}{:016x}", getrandom::u64()?, getrandom::u64()?);
    let wrapper = render_wrapper(&nonce, &document_nonce).into_bytes();
    let app = Mutex::new(Some(html.as_bytes().to_vec()));
    let bootstrap = format!(
        "(() => {{
            if (window === top || parent !== top || location.href !== '{ENGINE_APP_URL}') return;
            const documentNonce = '{document_nonce}';
            {}
        }})();",
        include_str!("mcp_app_document.js"),
    );
    let event_loop = EventLoopBuilder::<Option<Value>>::with_user_event().build();
    let proxy = event_loop.create_proxy();
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
                let _ = proxy.send_event(Some(value));
            }
        }
        let _ = proxy.send_event(None);
    });
    let window = WindowBuilder::new().with_title("ShiJu MCP App").build(&event_loop)?;
    let webview = WebViewBuilder::new()
        .with_custom_protocol("mcp-app".into(), move |_, request| {
            let uri = request.uri();
            let resource = if request.method() == http::Method::GET {
                match uri.to_string().as_str() {
                    WRAPPER_URL => Some((wrapper.clone(), WRAPPER_CSP)),
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
        .with_url(WRAPPER_URL)
        .with_initialization_script_for_main_only(bootstrap, false)
        .with_incognito(true)
        .with_devtools(false)
        .with_clipboard(false)
        .with_navigation_handler(move |url| {
            matches!(url.as_str(), WRAPPER_URL | APP_URL | "about:blank" | "about:srcdoc")
                || url == ENGINE_WRAPPER_URL
                || url == ENGINE_APP_URL
                // WebKit also calls this handler for frame navigations. The wrapper's
                // frame-src only permits its internal app document as a direct child.
                || (!cfg!(target_os = "windows") && frame_origin_allowed(&policy, &url))
        })
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_ipc_handler(move |request| {
            if request.uri() != ENGINE_WRAPPER_URL || request.body().len() > MAX_MESSAGE_BYTES {
                return;
            }
            let Ok(envelope) = serde_json::from_str::<Value>(request.body()) else {
                return;
            };
            if envelope["nonce"].as_str() != Some(nonce.as_str()) {
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
        })
        .build(&window)?;
    event_loop.run(move |event, _, flow| {
        let _ = window.id();
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Some(value)) => {
                // JSON is encoded as a JS string, then parsed: no page text becomes source.
                if let Ok(encoded) = serde_json::to_string(&value.to_string()) {
                    let _ = webview
                        .evaluate_script(&format!("window.__mcpDeliver?.(JSON.parse({encoded}))"));
                }
            }
            Event::UserEvent(None)
            | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *flow = ControlFlow::Exit
            }
            _ => {}
        }
    });
}

fn render_wrapper(host_nonce: &str, document_nonce: &str) -> String {
    format!(
        r#"<!doctype html>
<meta charset="utf-8">
<style>
html, body, iframe {{ margin: 0; width: 100%; height: 100%; border: 0; }}
::-webkit-scrollbar {{ width: 4px; }}
</style>
<iframe sandbox="allow-scripts"></iframe>
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
    format!(
        "default-src 'none'; script-src 'unsafe-inline' {resources}; style-src 'unsafe-inline' {resources}; font-src data: {resources}; img-src data: {resources}; media-src {media}; connect-src {connect}; frame-src {frame}; base-uri {base}; object-src 'none'; form-action 'none'"
    )
}

fn frame_origin_allowed(policy: &McpAppNetworkPolicy, target: &str) -> bool {
    Url::parse(target).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && policy.frame_domains.contains(&url.origin().ascii_serialization())
    })
}

#[cfg(test)]
#[path = "mcp_app_tests.rs"]
mod tests;
