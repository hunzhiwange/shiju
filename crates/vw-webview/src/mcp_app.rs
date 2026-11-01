//! Dedicated ephemeral MCP Apps surface; ordinary browser navigation is never enabled.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

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
    let mut resource_domains = String::new();
    let mut connect_domains = String::new();
    let mut base_domains = String::new();
    if let Some(policy) = input.get("csp") {
        let object = policy.as_object().ok_or("invalid CSP")?;
        for (key, values) in object {
            if !matches!(
                key.as_str(),
                "resourceDomains" | "connectDomains" | "baseUriDomains" | "redirectDomains"
            ) {
                return Err("unsupported CSP directive".into());
            }
            let values =
                values.as_array().filter(|v| v.len() <= 32).ok_or("invalid CSP domains")?;
            for value in values {
                let origin = value.as_str().ok_or("invalid origin")?;
                let authority = origin.strip_prefix("https://").ok_or("HTTPS origin required")?;
                if authority.is_empty()
                    || authority.len() > 253
                    || !authority.contains('.')
                    || !authority
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
                    || authority.starts_with('.')
                    || authority.ends_with('.')
                    || authority.split(':').count() > 2
                {
                    return Err("exact origin required".into());
                }
                if let Some((_, port)) = authority.split_once(':')
                    && port.parse::<u16>().ok().is_none_or(|p| p == 0)
                {
                    return Err("invalid origin port".into());
                }
                let target = match key.as_str() {
                    "resourceDomains" => &mut resource_domains,
                    "connectDomains" => &mut connect_domains,
                    "baseUriDomains" => &mut base_domains,
                    _ => continue,
                };
                target.push(' ');
                target.push_str(origin);
            }
        }
    }
    let connect_policy = if connect_domains.is_empty() { "'none'" } else { connect_domains.trim() };
    let base_policy = if base_domains.is_empty() { "'none'" } else { base_domains.trim() };
    // An opaque-origin iframe has no filesystem, native IPC, cookies, or host DOM access.
    // Deny network by default; unsupported declarations must be rejected by the host.
    let document = format!(
        r#"<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'{resource_domains}; style-src 'unsafe-inline'{resource_domains}; font-src data:{resource_domains}; frame-src 'self' about:; img-src data:{resource_domains}; connect-src {connect_policy}; base-uri {base_policy}; form-action 'none'"><style>html,body,iframe{{margin:0;width:100%;height:100%;border:0}}::-webkit-scrollbar{{width:4px}}</style><iframe sandbox="allow-scripts" srcdoc="{}"></iframe><script>const f=document.querySelector('iframe');window.addEventListener('message',e=>{{if(e.source===f.contentWindow&&e.origin==='null')window.ipc.postMessage(JSON.stringify({{nonce:"{nonce}",message:e.data}}));}});window.__mcpDeliver=(v)=>f.contentWindow.postMessage(v,'*');</script>"#,
        html.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
    );
    let event_loop = EventLoopBuilder::<Option<Value>>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            let mut line = String::new();
            match std::io::Read::take(&mut input, 1024 * 1024 + 1).read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) if line.len() > 1024 * 1024 => break,
                Ok(_) => {}
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line == "close" {
                break;
            }
            if line.len() <= 1024 * 1024
                && let Ok(value) = serde_json::from_str(line)
            {
                let _ = proxy.send_event(Some(value));
            }
        }
        let _ = proxy.send_event(None);
    });
    let window = WindowBuilder::new().with_title("ShiJu MCP App").build(&event_loop)?;
    let webview = WebViewBuilder::new()
        .with_custom_protocol("mcp-app".into(), move |_, request| {
            let allowed = request.uri().path() == "/index.html";
            wry::http::Response::builder()
                .status(if allowed { 200 } else { 403 })
                .header("Content-Type", "text/html; charset=utf-8")
                .body(std::borrow::Cow::Owned(if allowed {
                    document.as_bytes().to_vec()
                } else {
                    Vec::new()
                }))
                .expect("static response")
        })
        .with_url("mcp-app://localhost/index.html")
        .with_incognito(true)
        .with_devtools(false)
        .with_clipboard(false)
        .with_navigation_handler(|url| {
            matches!(
                url.as_str(),
                "mcp-app://localhost/index.html" | "about:blank" | "about:srcdoc"
            )
        })
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_ipc_handler(move |request| {
            if request.body().len() > 1024 * 1024 {
                return;
            }
            if let Ok(envelope) = serde_json::from_str::<Value>(request.body()) {
                if envelope["nonce"].as_str() != Some(nonce.as_str()) {
                    return;
                }
                let message = &envelope["message"];
                let mut stdout = io::stdout().lock();
                let _ = writeln!(stdout, "mcp-app {}", json!({"origin":"null","message":message}));
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
                        .evaluate_script(&format!("window.__mcpDeliver(JSON.parse({encoded}))"));
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
