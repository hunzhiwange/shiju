use wry::WebViewBuilder;

/// Keep media prompts in WebKit's requesting-site context and deny other permissions.
pub fn with_permission_handler<'a>(builder: WebViewBuilder<'a>) -> WebViewBuilder<'a> {
    // Wry otherwise grants WKWebView media requests. WebKit's prompt uses
    // the requesting site's security context and preserves system consent.
    builder.with_permission_handler(|kind| match kind {
        wry::PermissionKind::Camera | wry::PermissionKind::Microphone => {
            wry::PermissionResponse::Default
        }
        _ => wry::PermissionResponse::Deny,
    })
}
