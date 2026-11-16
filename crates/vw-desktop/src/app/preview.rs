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

pub use desktop_editor::preview::PreviewTab;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_editor::preview::{LspHoverPending, LspProgress};

pub use desktop_editor::preview::file_validation::{safe_preview_text, preview_open_error, preview_open_error_message};

pub use desktop_editor::preview::tab::safe_preview;


/// Semantic metadata for previews that cannot display editable text.
pub use desktop_editor::preview::tab::preview_description;
