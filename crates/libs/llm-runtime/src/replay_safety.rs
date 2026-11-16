//! Preserve the single replay-safety boundary for partial model output.
use message_types as message;
use std::collections::HashMap;

/// 标记无法安全重放的响应，让上层压缩与 ACP 恢复也遵守同一边界。
pub fn prevent_error_replay(error: &mut message::AssistantError) {
    if let message::AssistantError::Unknown { message }
    | message::AssistantError::ContextOverflowError { message, .. } = error
    {
        *error = message::AssistantError::APIError {
            message: message.clone(),
            status_code: None,
            is_retryable: false,
            response_headers: None,
            response_body: None,
            metadata: None,
        };
    }
    if let message::AssistantError::APIError { is_retryable, metadata, .. } = error {
        *is_retryable = false;
        metadata.get_or_insert_with(HashMap::new).insert("replay_safe".into(), "false".into());
    }
}
