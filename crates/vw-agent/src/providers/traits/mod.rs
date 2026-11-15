//! 模型调用契约由 `model-types` 统一持有。

pub use model_types::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider, ProviderBounds,
    ProviderCapabilities, ProviderCapabilityError, StreamChunk, StreamError, StreamOptions,
    StreamResult, TokenUsage, ToolCall, ToolCallKind, ToolResultMessage, ToolsPayload,
    build_tool_instructions_text,
};
