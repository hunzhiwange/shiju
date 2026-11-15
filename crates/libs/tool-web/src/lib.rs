//! HTTP 请求、代码检索与网页抓取工具。

pub mod codesearch;
pub mod http_headers;
pub mod http_request;
pub mod web_fetch;

pub use codesearch::CodeSearchTool;
pub use http_request::HttpRequestTool;
pub use web_fetch::WebFetchTool;

pub const WEB_FETCH_TOOL_ID: &str = "WebFetch";
pub const WEB_FETCH_TOOL_ALIAS: &str = "web_fetch";
