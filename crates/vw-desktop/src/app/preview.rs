//! 文件预览模块
//!
//! 本模块提供文件内容预览功能，包括：
//! - 安全读取和显示文件内容（自动截断过大的文件）
//! - 清理 ANSI 转义序列和控制字符
//! - 长行自动换行处理
//! - 搜索匹配高亮支持
//! - LSP（语言服务器协议）集成支持（非 WASM 平台）
//!
//! # 主要组件
//!
//! - [`PreviewTab`]: 预览标签页结构体，存储单个文件的预览状态
//! - [`LspHoverPending`]: LSP 悬停请求的待处理状态
//! - [`LspProgress`]: LSP 进度通知
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::preview::{PreviewTab, safe_preview};
//!
//! // 安全预览文件内容
//! let (content, truncated) = safe_preview("/path/to/file.rs");
//! println!("内容: {}", content);
//! println!("是否被截断: {}", truncated);
//! ```

pub use preview_content::{compute_search_matches, format_text_preview_content};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use preview_content::path_to_file_uri;
use preview_content::file_looks_like_text;

use crate::app::components::editor::Editor;
use iced::widget::Id;

/// 预览标签页
///
/// 存储单个文件在预览视图中的完整状态，包括：
/// - 文件路径和标题
/// - 文件内容（可能已截断）
/// - 编辑器组件状态
/// - 滚动位置标识
/// - LSP 相关信息（非 WASM 平台）
///
/// # 字段说明
///
/// - `path`: 文件的完整路径
/// - `title`: 显示在标签页上的标题
/// - `content`: 文件的实际内容
/// - `truncated`: 文件是否因过大而被截断
/// - `editor`: 编辑器组件实例
/// - `scroll_id`: 滚动容器的唯一标识符
/// - `lsp_server_key`: LSP 服务器标识键（非 WASM）
/// - `lsp_uri`: LSP 文件 URI（非 WASM）
/// - `lsp_language_id`: LSP 语言标识符（非 WASM）
pub struct PreviewTab {
    /// 文件的完整路径
    pub path: String,
    /// 标签页显示标题
    pub title: String,
    /// 文件内容
    pub content: String,
    /// 编辑器内容是否相对已保存版本发生变化
    pub is_dirty: bool,
    /// 文件是否被截断
    pub truncated: bool,
    /// 提交快照等非工作区内容只允许浏览。
    pub read_only: bool,
    /// 自动保存版本号，用于丢弃过期的延迟保存任务。
    pub auto_save_revision: u64,
    /// 编辑器组件
    pub editor: Editor,
    /// 滚动容器 ID
    pub scroll_id: Id,
    /// LSP 服务器键（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_server_key: Option<&'static str>,
    /// LSP 文件 URI（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_uri: Option<String>,
    /// LSP 语言标识符（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_language_id: Option<String>,
}

/// LSP 悬停请求待处理状态
///
/// 当用户在编辑器中悬停时，会创建此结构体来跟踪悬停请求的状态。
/// 包含请求的位置信息、屏幕坐标以及请求时间戳。
///
/// # 字段说明
///
/// - `path`: 悬停目标文件的路径
/// - `position`: LSP 协议中的文本位置（行、列）
/// - `point`: 屏幕上的鼠标位置（用于显示工具提示）
/// - `ready_at`: 请求创建的时间戳（用于超时检测）
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub(crate) struct LspHoverPending {
    /// 悬停目标文件路径
    pub(crate) path: String,
    /// LSP 文本位置（行、列）
    pub(crate) position: iced_code_editor::LspPosition,
    /// 屏幕鼠标位置
    pub(crate) point: iced::Point,
    /// 请求创建时间
    pub(crate) ready_at: std::time::Instant,
}

/// LSP 进度通知
///
/// 表示来自语言服务器的进度更新，通常用于长时间运行的操作
/// （如索引项目、分析代码等）。
///
/// # 字段说明
///
/// - `title`: 进度操作的标题
/// - `message`: 可选的详细消息
/// - `percentage`: 可选的完成百分比（0-100）
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub(crate) struct LspProgress {
    /// 进度标题
    pub(crate) title: String,
    /// 详细消息
    pub(crate) message: Option<String>,
    /// 完成百分比
    pub(crate) percentage: Option<u32>,
}

impl std::fmt::Debug for PreviewTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 仅调试输出关键字段，避免大量内容污染日志
        f.debug_struct("PreviewTab").field("path", &self.path).field("title", &self.title).finish()
    }
}

/// 读取并格式化文件文本，保留桌面截断提示。
pub fn safe_preview_text(path: &str) -> String {
    let (content, truncated) = preview_content::read_preview_text(path);
    if truncated {
        format!("文件较大，预览已截断。\n\n{content}")
    } else {
        content
    }
}

fn unsupported_preview_kind(path: &str) -> Option<&'static str> {
    preview_content::unsupported_preview_kind(path).map(|kind| match kind {
        preview_content::UnsupportedPreviewKind::Archive => "压缩包",
        preview_content::UnsupportedPreviewKind::Pdf => "PDF",
    })
}

pub fn preview_open_error(path: &str) -> Option<String> {
    preview_open_error_message(path).map(|message| {
        message
            .render(crate::app::i18n::I18n::new(
                config_ui::DesktopLocale::SimplifiedChinese,
            ))
            .expect("preview error template")
    })
}

pub fn preview_open_error_message(path: &str) -> Option<crate::app::i18n::format::Message> {
    use crate::app::i18n::format::Message;
    if let Some(kind) = unsupported_preview_kind(path) {
        let kind =
            if kind == "PDF" { Message::literal(kind.into()) } else { Message::new(kind, vec![]) };
        return Some(Message::nested("不支持展开该文件：%s 文件无法打开".into(), vec![kind]));
    }
    if !file_looks_like_text(path) {
        return Some(Message::new("不支持展开该文件：二进制文件无法打开", vec![]));
    }
    None
}

pub fn safe_preview(path: &str) -> (String, bool) {
    use preview_content::{FilePreview, UnsupportedPreviewKind};
    let kind = match preview_content::read_file_preview(path) {
        FilePreview::Text { content, truncated } => return (content, truncated),
        FilePreview::Image => "图片",
        FilePreview::Unsupported(UnsupportedPreviewKind::Archive) => "压缩包",
        FilePreview::Unsupported(UnsupportedPreviewKind::Pdf) => "PDF",
        FilePreview::Binary => "二进制文件",
    };
    let name = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
    let info = if kind == "图片" {
        format!("文件: {}\n类型: 图片\n", name)
    } else {
        format!("文件: {}\n类型: {}\n不支持文本预览\n", name, kind)
    };
    (info, false)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;

/// Semantic metadata for previews that cannot display editable text.
pub fn preview_description(path: &str) -> Option<crate::app::i18n::format::Message> {
    use crate::app::i18n::format::Message;
    let name =
        std::path::Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path);
    let kind = unsupported_preview_kind(path)?;
    let kind =
        if kind == "PDF" { Message::literal(kind.into()) } else { Message::new(kind, vec![]) };
    Some(Message::nested(
        "文件: %s\n类型: %s\n不支持文本预览\n".into(),
        vec![Message::literal(name.into()), kind],
    ))
}
