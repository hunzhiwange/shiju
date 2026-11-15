use clap::Subcommand;
/// 通道管理子命令枚举。
///
/// 与 CLI 枚举镜像，但避免 clap 依赖。
/// 用于运行时动态处理通道相关的管理命令。
#[derive(Debug, Clone, Subcommand)]
pub enum ChannelCommands {
    /// 列出所有已配置的通道
    List,
    /// 启动通道服务
    Start,
    /// 诊断通道配置问题
    Doctor,
    /// 添加新通道
    ///
    /// # 参数
    /// - `channel_type`: 通道类型标识符（如 "telegram"、"discord"）
    /// - `config`: 通道配置（JSON 格式字符串）
    Add { channel_type: String, config: String },
    /// 移除指定通道
    ///
    /// # 参数
    /// - `name`: 要移除的通道名称
    Remove { name: String },
    /// 绑定 Telegram 身份
    ///
    /// # 参数
    /// - `identity`: 身份标识符
    BindTelegram { identity: String },
}
