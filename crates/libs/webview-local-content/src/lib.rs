//! WebView 本地路径解析与内容协议响应。

#![cfg(not(target_arch = "wasm32"))]

mod paths;
mod response;

pub use paths::{StartTarget, normalize_url, percent_encode_url_path, resolve_target};
pub use response::local_protocol_response;

#[cfg(test)]
mod test_support;
