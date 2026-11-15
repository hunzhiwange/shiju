//! 聊天流协议兼容导出。

pub use vw_api_chat::{
    ChatRole, MessageStatus, MessagePartDto,
    MessageDto, InputMessageDto, ChatOptionsDto,
    ChatContextDto, ChatStreamRequest, GatewayChatStreamRequest,
    GatewayChatStreamEvent, SessionStartedEvent, AssistantDeltaEvent,
    AssistantMessageCompletedEvent, ToolStartedEvent, ToolUpdatedEvent,
    ToolCompletedEvent, TodoUpdatedEvent, QuestionRaisedEvent,
    QuestionResolvedEvent, UsageUpdatedEvent, TitleUpdatedEvent,
    SessionUpdatedEvent, ChatErrorEvent, DoneEvent,
    ChatEvent, ReasoningEffortDto, SessionAgentIdentity,
};
