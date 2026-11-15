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

pub use desktop_editor::preview::PreviewTab;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_editor::preview::{LspHoverPending, LspProgress};

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

pub use desktop_editor::preview::tab::safe_preview;

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;

/// Semantic metadata for previews that cannot display editable text.
pub use desktop_editor::preview::tab::preview_description;
