//! WebView download handlers and macOS attachment navigation support.

#![cfg(not(any(target_arch = "wasm32", coverage)))]

#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use webview_storage::append_log;
#[cfg(unix)]
use wry::WebViewBuilder;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{AttachmentNavigationDelegate, enable_attachments};

#[cfg(unix)]
pub fn with_downloads<'a>(
    builder: WebViewBuilder<'a>,
    log_path: &Path,
) -> WebViewBuilder<'a> {
    let log_path = log_path.to_owned();
    builder.with_download_started_handler(|_, _| true).with_download_completed_handler(
        move |_, _, success| {
            if !success {
                append_log(&log_path, "WebView download failed.");
            }
        },
    )
}
