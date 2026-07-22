//! CLI 会话摘要转写渲染模块
//!
//! 本模块提供将转写条目（transcript entries）转换为可渲染的 TUI 行的功能。
//! 主要用于在命令行界面中以格式化的方式显示代理与用户之间的对话历史。
//!
//! # 核心功能
//!
//! - 将转写条目转换为带有角色前缀和样式的显示行
//! - 支持工具块（tool blocks）的折叠/展开控制
//! - 支持思考块（think blocks）的按需展开
//! - 管理思考块 ID 映射，用于用户交互（如点击展开）
//! - 处理草稿内容的实时显示
//!
//! # 架构位置
//!
//! 该模块位于 CLI 转写系统的摘要视图层，负责最终的行渲染逻辑。
//! 它依赖于上游的解析器（`parse_assistant_segments`）和样式工具
//!（`transcript_prefix_style`）来完成完整的内容渲染。

use super::super::theme::TEXT_MUTED;
use super::{
    ThinkBlockMeta, TranscriptEntry, TranscriptRole, assistant_segments_to_lines_with_meta,
    default_empty_transcript_line, parse_assistant_segments, transcript_prefix_style,
};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::collections::BTreeSet;

fn assistant_think_id_salt(visible_entry_idx: usize) -> u64 {
    u64::try_from(visible_entry_idx).unwrap_or(u64::MAX)
}

/// 将转写条目列表转换为可渲染的 TUI 行
///
/// 该函数遍历所有转写条目，根据角色类型和展开状态，
/// 生成带有样式前缀的显示行，并维护思考块 ID 到行索引的映射。
///
/// # 参数
///
/// * `transcript` - 转写条目切片，包含完整的对话历史
/// * `expand_tool_blocks` - 是否展开所有工具块（显示完整内容）
/// * `expand_think_all` - 是否展开所有思考块（忽略 `expanded_think_blocks` 集合）
/// * `think_detail_overrides` - 已被手动切换的思考块 ID 集合，用于覆盖默认展开状态
/// * `draft` - 当前草稿内容，将作为灰色文本附加到末尾
///
/// # 返回值
///
/// 返回一个元组：
/// - `Vec<Line<'a>>`: 可渲染的行列表，包含文本和样式信息
/// - `Vec<Option<ThinkBlockMeta>>`: 思考块元数据映射，每行对应一个可选的思考块元数据
///   - `Some(meta)`: 该行属于指定的思考块，可用于交互和状态推导
///   - `None`: 该行不属于任何思考块或为非助手内容
///
/// # 渲染规则
///
/// ## 角色前缀
///
/// - 助手消息：带样式的角色前缀，内容支持多行
/// - 用户消息：带样式的角色前缀
/// - 系统消息：带样式的角色前缀
/// - 进度/错误消息：跳过不渲染（这些由其他组件处理）
///
/// ## 助手内容处理
///
/// 助手消息需要特殊处理，因为可能包含：
/// - 嵌入的工具调用块
/// - 思考块（可折叠）
/// - 多格式文本
///
/// 只有首行添加角色前缀，后续行保持对齐。
///
/// # 示例
///
/// ```ignore
/// use std::collections::BTreeSet;
///
/// let transcript = vec![
///     TranscriptEntry {
///         role: TranscriptRole::User,
///         text: "Hello".to_string(),
///         // ... 其他字段
///     },
///     TranscriptEntry {
///         role: TranscriptRole::Assistant,
///         text: "Hi there!".to_string(),
///         // ... 其他字段
///     },
/// ];
///
/// let expanded = BTreeSet::new();
/// let (lines, think_map) = transcript_to_lines(
///     &transcript,
///     false,  // 不展开工具块
///     false,  // 不展开所有思考块
///     &expanded,
///     "",     // 无草稿
/// );
///
/// assert_eq!(lines.len(), 2);
/// assert!(think_map.iter().all(|m| m.is_none())); // 无思考块
/// ```
///
/// # 性能考虑
///
/// - 预分配向量容量以减少重新分配
/// - 使用迭代器避免中间集合
/// - 仅在必要时克隆或转换数据
pub(crate) fn transcript_to_lines<'a>(
    transcript: &'a [TranscriptEntry],
    expand_tool_blocks: bool,
    expand_think_all: bool,
    think_detail_overrides: &BTreeSet<u64>,
    draft: &'a str,
) -> (Vec<Line<'a>>, Vec<Option<ThinkBlockMeta>>) {
    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut think_map: Vec<Option<ThinkBlockMeta>> = Vec::new();
    let mut last_visible_entry_at: Option<chrono::DateTime<chrono::Local>> = None;
    let mut has_visible_entry = false;
    let mut visible_entry_idx = 0usize;

    if transcript.is_empty() {
        lines.push(default_empty_transcript_line());
        think_map.push(None);
    } else {
        for entry in transcript {
            if matches!(entry.role, TranscriptRole::Progress | TranscriptRole::Error) {
                continue;
            }
            if has_visible_entry {
                lines.push(Line::from(Span::raw("")));
                think_map.push(None);
            }

            let (prefix, _color, prefix_style) = transcript_prefix_style(entry.role);

            if matches!(entry.role, TranscriptRole::Assistant) {
                let think_duration_secs = last_visible_entry_at.map(|at| {
                    entry.at.signed_duration_since(at).num_seconds().max(0).cast_unsigned()
                });
                let (mut rendered, mut rendered_meta) = assistant_segments_to_lines_with_meta(
                    parse_assistant_segments(&entry.text),
                    expand_tool_blocks,
                    expand_think_all,
                    think_detail_overrides,
                    think_duration_secs,
                    assistant_think_id_salt(visible_entry_idx),
                );

                if rendered.is_empty() {
                    rendered.push(Line::from(Span::raw("")));
                    rendered_meta.push(None);
                }

                for (idx, line) in rendered.into_iter().enumerate() {
                    let mut spans = line.spans;

                    if idx == 0 && !prefix.is_empty() {
                        let mut prefixed = Vec::with_capacity(spans.len().saturating_add(1));
                        prefixed.push(Span::styled(prefix.to_string(), prefix_style));
                        prefixed.append(&mut spans);
                        lines.push(Line::from(prefixed));
                    } else {
                        lines.push(Line::from(spans));
                    }

                    let meta = rendered_meta.get(idx).and_then(|v| *v);
                    think_map.push(meta);
                }
            } else {
                for (idx, raw_line) in entry.text.lines().enumerate() {
                    if idx == 0 {
                        if prefix.is_empty() {
                            lines.push(Line::from(Span::raw(raw_line)));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled(prefix.to_string(), prefix_style),
                                Span::raw(raw_line),
                            ]));
                        }
                    } else {
                        lines.push(Line::from(Span::raw(raw_line)));
                    }
                    think_map.push(None);
                }
            }
            last_visible_entry_at = Some(entry.at);
            has_visible_entry = true;
            visible_entry_idx = visible_entry_idx.saturating_add(1);
        }
    }

    if !draft.trim().is_empty() {
        lines.push(Line::from(vec![Span::styled(draft, Style::default().fg(TEXT_MUTED))]));
        think_map.push(None);
    }

    (lines, think_map)
}
