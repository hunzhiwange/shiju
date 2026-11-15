//! 预览编辑器的标签状态与局部行为。
use crate::components::editor::Editor;
use iced::widget::Id;
pub mod editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;
pub mod snapshot;

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
pub struct LspHoverPending {
    /// 悬停目标文件路径
    pub path: String,
    /// LSP 文本位置（行、列）
    pub position: iced_code_editor::LspPosition,
    /// 屏幕鼠标位置
    pub point: iced::Point,
    /// 请求创建时间
    pub ready_at: std::time::Instant,
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
pub struct LspProgress {
    /// 进度标题
    pub title: String,
    /// 详细消息
    pub message: Option<String>,
    /// 完成百分比
    pub percentage: Option<u32>,
}

impl std::fmt::Debug for PreviewTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 仅调试输出关键字段，避免大量内容污染日志
        f.debug_struct("PreviewTab").field("path", &self.path).field("title", &self.title).finish()
    }
}

pub mod message;

pub mod selection;

pub mod tab;
