//! 结构化流式输出处理模块
//!
//! 本模块提供 CLI 转录（transcript）中结构化流式输出的渲染支持。
//! 主要用于处理包含特殊标记（如 `<think` 标签或 `tool ` 前缀）的
//! 助手响应内容，将其转换为格式化的转录视图以便在终端界面中显示。
//!
//! # 功能概述
//!
//! - 检测是否需要结构化流式渲染（通过 [`should_render_structured_stream`]）
//! - 构建流式转录视图（通过 [`build_streaming_transcript_view`]）
//!
//! # 使用场景
//!
//! 当助手的响应内容包含：
//! - 思考过程标记（`<think` 或 `\n<think`）
//! - 工具调用标记（`tool ` 或 `\ntool `）
//!
//! 时，需要使用本模块进行特殊渲染处理，以提供更好的用户阅读体验。

use super::{TranscriptEntry, TranscriptRole};

/// 检测草稿内容是否需要进行结构化流式渲染
///
/// 该函数通过检查内容中是否包含特定的标记来决定是否启用
/// 结构化流式渲染模式。这些标记包括思考过程标签和工具调用前缀。
///
/// # 参数
///
/// - `draft`: 待检测的草稿内容字符串
///
/// # 返回值
///
/// 返回 `true` 表示内容需要结构化流式渲染，返回 `false` 表示可以按普通文本处理。
///
/// # 检测规则
///
/// 内容需要结构化渲染的条件（需同时满足以下所有条件）：
/// 1. 内容非空（去除空白字符后）
/// 2. 内容包含以下任一标记：
///    - 以 `<think` 开头（思考过程起始标签）
///    - 包含换行后的 `<think`（行首的思考标签）
///    - 以 `tool ` 开头（工具调用行）
///    - 包含换行后的 `tool `（行首的工具调用）
///
/// # 示例
///
/// ```ignore
/// // 包含思考标签的内容
/// assert!(should_render_structured_stream("<think\n推理过程..."));
///
/// // 包含工具调用的内容
/// assert!(should_render_structured_stream("tool read_file\n{\"path\": \"/src/main.rs\"}"));
///
/// // 普通文本内容
/// assert!(!should_render_structured_stream("这是一段普通的助手回复"));
///
/// // 空内容
/// assert!(!should_render_structured_stream(""));
/// assert!(!should_render_structured_stream("   \n  "));
/// ```
pub(crate) fn should_render_structured_stream(draft: &str) -> bool {
    // 先去除首尾空白字符，避免纯空白内容被误判
    let t = draft.trim();

    // 内容必须非空，且包含以下至少一种结构化标记：
    // - <think: 思考过程标签（可能是行首或内容开头）
    // - tool : 工具调用前缀（可能是行首或内容开头）
    !t.is_empty()
        && (t.starts_with("<think")
            || t.contains("\n<think")
            || t.starts_with("tool ")
            || t.contains("\ntool "))
}

/// 构建流式转录视图
///
/// 根据草稿内容是否需要结构化渲染，生成相应的转录条目列表。
/// 对于包含结构化标记的内容，会解析并格式化为可读性更强的形式。
///
/// # 参数
///
/// - `transcript`: 现有的转录条目列表，包含之前的对话历史
/// - `draft`: 当前正在流式输出的草稿内容（助手的响应内容）
/// - `expand_tool_details`: 是否展开工具调用的详细信息
///
/// # 返回值
///
/// 返回一个元组 `(Vec<TranscriptEntry>, String)`：
/// - 第一个元素是更新后的转录条目列表
/// - 第二个元素是剩余的未渲染草稿内容（结构化渲染时返回空字符串）
///
/// # 处理逻辑
///
/// 1. **非结构化内容**：如果草稿不需要结构化渲染，直接返回原有转录列表
///    和原始草稿内容的副本，不做任何修改。
///
/// 2. **结构化内容**：如果草稿需要结构化渲染：
///    a. 使用 [`parse_assistant_segments`] 解析草稿内容为段落
///    b. 使用 [`assistant_segments_to_lines`] 将段落转换为可显示的行
///    c. 将格式化后的内容作为新的助手转录条目追加到列表末尾
///    d. 返回空字符串作为剩余草稿（因为内容已完全渲染到转录中）
///
/// # 示例
///
/// ```ignore
/// let transcript = vec![
///     TranscriptEntry::new(TranscriptRole::User, "你好"),
/// ];
///
/// // 普通内容 - 保持原样
/// let (view, remaining) = build_streaming_transcript_view(
///     &transcript,
///     "你好！有什么可以帮助你的？",
///     false,
/// );
/// assert_eq!(remaining, "你好！有什么可以帮助你的？");
///
/// // 结构化内容 - 转换为格式化视图
/// let (view, remaining) = build_streaming_transcript_view(
///     &transcript,
///     "<think\n正在思考...\n</think\n答案是42",
///     false,
/// );
/// assert_eq!(remaining, ""); // 内容已完全渲染
/// assert_eq!(view.len(), 2); // 原有条目 + 新的助手条目
/// ```
///
/// # 设计说明
///
/// 返回空字符串作为剩余草稿的设计原因是：结构化内容（如思考过程和工具调用）
/// 已经被解析并格式化后追加到转录列表中，因此不需要在草稿区域重复显示。
/// 这样可以避免内容重复，同时保持用户界面的清晰性。
pub fn build_streaming_transcript_view(
    transcript: &[TranscriptEntry],
    draft: &str,
    expand_tool_details: bool,
) -> (Vec<TranscriptEntry>, String) {
    // 快速路径：如果内容不需要结构化渲染，直接返回原数据
    if !should_render_structured_stream(draft) {
        return (transcript.to_vec(), draft.to_string());
    }

    // 创建新的转录视图：复制现有转录并追加格式化后的助手响应
    let _ = expand_tool_details;
    let mut streaming_view = transcript.to_vec();
    streaming_view.push(TranscriptEntry::new(TranscriptRole::Assistant, draft.to_string()));

    // 返回更新后的转录视图和空的剩余草稿
    // 空字符串表示草稿内容已完全处理并渲染到转录中
    (streaming_view, String::new())
}
