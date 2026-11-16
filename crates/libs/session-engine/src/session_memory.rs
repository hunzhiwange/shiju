//! In-memory session conversation owned by the session runtime.

/// 内存中的会话对象
///
/// 表示一个活跃的会话，包含会话 ID 和消息历史。
/// 主要用于运行时消息管理和上下文维护。
#[derive(Debug, Clone)]
pub struct Session {
    /// 会话唯一标识符（降序时间戳格式）
    pub id: String,
    /// 消息历史列表
    pub messages: Vec<Message>,
}

/// 消息对象
///
/// 表示会话中的一条消息，包含角色和内容。
#[derive(Debug, Clone, Hash)]
pub struct Message {
    /// 消息角色（用户、助手、系统、工具）
    pub role: Role,
    /// 消息文本内容
    pub content: String,
}

/// 消息角色枚举
///
/// 定义消息的可能角色类型，用于区分消息来源和用途。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// 用户消息
    User,
    /// 助手（AI）消息
    Assistant,
    /// 系统消息
    System,
    /// 开发者指令消息
    Developer,
    /// 工具调用消息
    Tool,
}

impl Session {
    /// 创建新的会话实例
    ///
    /// # 参数
    ///
    /// - `id`: 会话唯一标识符
    ///
    /// # 返回
    ///
    /// 返回一个空的会话实例，消息列表初始化为空
    ///
    /// # 示例
    ///
    /// ```
    /// use session_engine::session_memory::{Session, Role};
    ///
    /// let session = Session::new("session-123".to_string());
    /// assert!(session.messages.is_empty());
    /// ```
    pub fn new(id: String) -> Self {
        Self { id, messages: Vec::new() }
    }

    /// 向会话添加消息
    ///
    /// # 参数
    ///
    /// - `role`: 消息角色
    /// - `content`: 消息内容
    ///
    /// # 示例
    ///
    /// ```
    /// use session_engine::session_memory::{Session, Role};
    ///
    /// let mut session = Session::new("session-123".to_string());
    /// session.push(Role::User, "你好".to_string());
    /// assert_eq!(session.messages.len(), 1);
    /// ```
    pub fn push(&mut self, role: Role, content: String) {
        self.messages.push(Message { role, content });
    }
}
