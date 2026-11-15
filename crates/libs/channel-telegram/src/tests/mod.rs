//! Telegram 通道测试模块
//!
//! 本模块整合了 Telegram 通道实现的所有测试用例，采用模块化组织方式，
//! 将不同功能领域的测试分离到独立的子模块中，便于维护和扩展。
//!
//! # 测试子模块说明
//!
//! - `ack_reaction`: 消息确认反应机制测试，验证消息已读/处理状态的反应功能
//! - `approval_callback`: 审批回调测试，验证需要用户确认的操作的回调处理
//! - `attachment_format`: 附件格式测试，验证各类文件附件的格式化与传输
//! - `attachments`: 附件处理测试，验证图片、文档、视频等附件的上传与下载
//! - `channel_basic`: 通道基础功能测试，验证通道的初始化、连接、断开等基本操作
//! - `file_sending`: 文件发送测试，验证文件传输的可靠性与错误处理
//! - `helpers`: 测试辅助工具，提供测试用例共用的 mock、fixture 和工具函数
//! - `markdown_html`: Markdown 与 HTML 转换测试，验证消息格式的正确渲染
//! - `mention_handling`: @提及处理测试，验证用户提及的解析与通知机制
//! - `message_parsing`: 消息解析测试，验证接收消息的结构化解析逻辑
//! - `message_splitting`: 消息分割测试，验证超长消息的分段发送策略
//! - `pairing_bind`: 配对绑定测试，验证设备/会话配对与绑定流程
//! - `streaming_drafts`: 流式草稿测试，验证消息编辑与实时更新功能
//! - `tool_call_tags`: 工具调用标签测试，验证工具执行结果的标签化展示
//! - `user_allowlist`: 用户白名单测试，验证访问控制与权限过滤

use super::attachments::{
    IncomingAttachmentKind, TELEGRAM_MAX_FILE_DOWNLOAD_BYTES, TelegramAttachmentKind,
    format_attachment_content, infer_attachment_kind_from_target, is_image_extension,
    parse_attachment_markers, parse_path_only_attachment, resolve_workspace_attachment_output_path,
    resolve_workspace_attachment_path, sanitize_attachment_filename,
};
use super::message_utils::{
    TELEGRAM_ACK_REACTIONS, TELEGRAM_MAX_MESSAGE_LENGTH, build_telegram_ack_reaction_request,
    random_telegram_ack_reaction, split_message_for_telegram,
};
use super::tool_tags::strip_tool_call_tags;
use super::*;

mod ack_reaction;
mod approval_callback;
mod attachment_format;
mod attachments;
mod channel_basic;
mod file_sending;
mod helpers;
mod markdown_html;
mod mention_handling;
mod message_parsing;
mod message_splitting;
mod pairing_bind;
mod streaming_drafts;
mod tool_call_tags;
mod user_allowlist;
