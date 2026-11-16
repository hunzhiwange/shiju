//! 消息格式转换由会话引擎提供。

pub(crate) use session_engine::runner_messages::{
    assistant_message_with_reasoning, now_ms, tool_calls_to_assistant_message,
    tool_result_to_message,
};
