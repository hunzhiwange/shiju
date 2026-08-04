//! HTTP 工具共享的内容协商请求头。

use reqwest::header::{ACCEPT_LANGUAGE, HeaderMap, HeaderValue};

pub(super) const ACCEPT_LANGUAGE_VALUE: &str = "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7";

/// 返回适用于网页与 JSON API 请求的通用内容协商头。
pub(super) fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static(ACCEPT_LANGUAGE_VALUE));
    headers
}

#[cfg(test)]
#[path = "http_headers_tests.rs"]
mod tests;
