//! MCP Apps wrapper page and document bridge resources.
use webview_mcp_policy::MAX_MESSAGE_BYTES;

/// Bridge injected into the application document by the native host.
pub const DOCUMENT_SCRIPT: &str = include_str!("mcp_app_document.js");

pub fn render_wrapper(i18n: desktop_i18n::I18n, host_nonce: &str, document_nonce: &str, form_enabled: bool) -> String {
    let ui_copy = locale_copy(i18n);
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
    const uiCopy = {ui_copy};
    const hostNonce = "{host_nonce}";
    const documentNonce = "{document_nonce}";
    const maxMessageBytes = {MAX_MESSAGE_BYTES};
    {}
}})();
</script>"#,
        include_str!("mcp_app_host.js"),
    )
}

/// Only host-owned labels enter this trusted wrapper script.
pub fn locale_copy(i18n: desktop_i18n::I18n) -> String {
    serde_json::json!({
        "popupBlocked": i18n.__("MCP 应用无法打开此弹窗，请使用外部链接。"),
        "linkDenied": i18n.__("未打开外部链接。"),
    }).to_string().replace('<', "\\u003c")
}

#[cfg(test)]
mod wrapper_tests;
