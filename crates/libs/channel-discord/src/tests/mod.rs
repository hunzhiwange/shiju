//! Discord 频道模块的单元测试
//!
//! 本模块包含 DiscordChannel 实现的全面测试套件，涵盖以下方面：
//! - 频道标识与命名验证
//! - Bot 用户 ID 提取与解码
//! - 用户权限控制（白名单机制）
//! - 消息内容规范化与提及处理
//! - 消息分割（遵守 Discord 2000 字符限制）
//! - 正在输入状态管理
//! - 表情符号编码与反应
//! - 附件处理（图片、音频、转录）
//! - 工作区路径安全与遍历防护
//!
//! 这些测试确保 Discord 集成的安全性、正确性和稳健性。

use super::*;

mod attachments_incoming;
mod attachments_outgoing;
mod basics;
mod message_ids;
mod message_split;
mod reactions;
mod typing;
