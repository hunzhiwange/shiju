//! MCP Apps wrapper page and document bridge resources.
use webview_mcp_policy::MAX_MESSAGE_BYTES;

/// Bridge injected into the application document by the native host.
pub const DOCUMENT_SCRIPT: &str = include_str!("mcp_app_document.js");

pub fn render_wrapper(host_nonce: &str, document_nonce: &str, form_enabled: bool) -> String {
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

#[cfg(test)]
mod wrapper_tests;
