//! 网关聊天事件、SSE 解析与网络订阅。

mod types;
mod normalize;
mod sse;
mod transport;

pub use types::*;
pub use normalize::{normalize_chat_stream_event, normalize_chat_tool_event};
pub use sse::{GatewaySseFrame, parse_sse_frame, take_next_sse_event};
pub use transport::{apply_stream_resume, stream_chat, stream_chat_resumable, stream_json_sse};
#[cfg(not(target_arch = "wasm32"))]
pub use transport::{stream_chat_blocking, stream_chat_blocking_with_idle_timeout, stream_chat_with_idle_timeout};

#[cfg(test)]
use sse::{event_name, is_terminal_stream_event, parse_stream_event};
#[cfg(test)]
use transport::stream_http_failure_message;
#[cfg(test)]
mod stream_tests;
