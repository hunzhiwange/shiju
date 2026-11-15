//! view_tests.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use std::path::Path;

/// 重新导出 use super::{BadgeTone, preview_breadcrumb_segments}，让上层模块通过稳定路径访问。
use super::preview_breadcrumb_segments;
#[cfg(not(target_arch = "wasm32"))]
use super::BadgeTone;
/// 重新导出 use iced::Theme，让上层模块通过稳定路径访问。
use iced::Theme;

/// 重新导出 use super::{，让上层模块通过稳定路径访问。
#[cfg(not(target_arch = "wasm32"))]
use super::{
    active_preview_lsp_badge_colors, active_preview_lsp_badge_dot_color,
    active_preview_lsp_badge_state,
};

/// 验证 preview breadcrumb segments use relative path inside project 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn preview_breadcrumb_segments_use_relative_path_inside_project() {
    let segments =
        preview_breadcrumb_segments(Path::new("/tmp/demo/src/main.rs"), Some("/tmp/demo"));

    assert_eq!(segments, vec!["src".to_string(), "main.rs".to_string()]);
}

/// 验证 preview breadcrumb segments fall back to absolute path outside project 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn preview_breadcrumb_segments_fall_back_to_absolute_path_outside_project() {
    let segments =
        preview_breadcrumb_segments(Path::new("/tmp/other/notes/todo.md"), Some("/tmp/demo"));

    assert_eq!(segments, vec!["/tmp/other/notes/todo.md".to_string()]);
}

/// 验证 active preview lsp badge state keeps label short for ready server 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_keeps_label_short_for_ready_server() {
    let (label, tone) = active_preview_lsp_badge_state(Some("rust-analyzer"), false, true);

    assert_eq!(label, "LSP");
    assert_eq!(tone, BadgeTone::Ready);
}

/// 验证 active preview lsp badge state marks syncing servers with working tone 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_marks_syncing_servers_with_working_tone() {
    let (label, tone) = active_preview_lsp_badge_state(Some("rust-analyzer"), true, true);

    assert_eq!(label, "LSP");
    assert_eq!(tone, BadgeTone::Working);
}

/// 验证 active preview lsp badge state uses lsp label for unavailable language service 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_uses_lsp_label_for_unavailable_language_service() {
    let (label, tone) = active_preview_lsp_badge_state(None, false, true);

    assert_eq!(label, "LSP");
    assert_eq!(tone, BadgeTone::Error);
}

/// 验证 active preview lsp badge state marks plain text without language service 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_marks_plain_text_without_language_service() {
    let (label, tone) = active_preview_lsp_badge_state(None, false, false);

    assert_eq!(label, "Plain Text");
    assert_eq!(tone, BadgeTone::Neutral);
}

/// 验证 active preview lsp badge uses editor theme palette 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_uses_editor_theme_palette() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let (background, text_color, border_color) =
        active_preview_lsp_badge_colors(&theme, BadgeTone::Ready);

    assert_eq!(background, palette.success.base.color.scale_alpha(0.12));
    assert_eq!(text_color, palette.success.base.color);
    assert_eq!(border_color, palette.success.base.color.scale_alpha(0.32));
}

/// 验证 active preview lsp badge dot uses editor theme palette 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_dot_uses_editor_theme_palette() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(
        active_preview_lsp_badge_dot_color(&theme, BadgeTone::Neutral),
        palette.background.strong.color
    );
}
