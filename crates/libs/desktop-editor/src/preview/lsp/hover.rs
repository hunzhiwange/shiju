//! 悬停清理、防抖与延迟请求。
use super::PreviewTab;
use std::time::{Duration, Instant};
pub fn clear_lsp_hover(
    lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    lsp_hover_hide_deadline: &mut Option<Instant>,
    lsp_hover_pending: &mut Option<super::LspHoverPending>,
    lsp_overlay: &mut iced_code_editor::LspOverlayState,
    lsp_overlay_path: &mut Option<String>,
) {
    // 清除悬停覆盖层的内容
    lsp_overlay.clear_hover();

    // 清除悬停锚点（用于防抖）
    (*lsp_hover_anchor) = None;

    // 清除待处理的悬停请求
    (*lsp_hover_pending) = None;

    // 清除悬停隐藏计时器
    (*lsp_hover_hide_deadline) = None;

    // 如果没有显示自动补全窗口，同时清除 LSP 路径
    // 这样可以避免在没有 LSP 功能时保持不必要的状态
    if !lsp_overlay.completion_visible {
        (*lsp_overlay_path) = None;
    }
}

pub fn process_lsp_hover_timers(
    lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    lsp_hover_hide_deadline: &mut Option<Instant>,
    lsp_hover_pending: &mut Option<super::LspHoverPending>,
    lsp_overlay: &mut iced_code_editor::LspOverlayState,
    lsp_overlay_path: &mut Option<String>,
    preview_tabs: &mut Vec<PreviewTab>,
) {
    // 获取当前时间，用于所有定时器检查
    let now = Instant::now();

    // 一致性检查：如果悬停窗口可见但没有关联的 LSP 路径，说明状态不一致，立即清除
    if lsp_overlay.hover_visible && lsp_overlay_path.is_none() {
        tracing::debug!("[LSP HOVER] clear_lsp_hover: hover_visible=true but path is None");
        clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
    }

    // 处理待处理的悬停请求
    if let Some(pending) = lsp_hover_pending.take() {
        tracing::debug!("[LSP HOVER] pending ready_at={:?}, now={:?}", pending.ready_at, now);
        // 检查是否达到延迟时间
        if now >= pending.ready_at {
            // 延迟时间已到，发送实际的悬停请求
            let request_sent =
                if let Some(tab) = preview_tabs.iter_mut().find(|t| t.path == pending.path) {
                    // 先刷新编辑器中的待处理更改，确保 LSP 看到最新内容
                    tab.editor.lsp_flush_pending_changes();
                    // 发送悬停请求到 LSP 服务器
                    let result = tab.editor.lsp_request_hover_at_position(pending.position);
                    tracing::debug!("[LSP HOVER] request_hover_at_position returned {:?}", result);
                    result
                } else {
                    // 找不到对应的标签页，无法发送请求
                    tracing::debug!("[LSP HOVER] tab not found for path {:?}", pending.path);
                    false
                };

            if request_sent {
                // 请求成功发送，设置悬停窗口的显示位置
                lsp_overlay.set_hover_position(pending.point);
                // 记录当前 LSP 路径
                (*lsp_overlay_path) = Some(pending.path);
                tracing::debug!(
                    "[LSP HOVER] request sent, set_hover_position at {:?}",
                    pending.point
                );
            } else {
                // 请求发送失败，清除悬停锚点
                (*lsp_hover_anchor) = None;
                tracing::debug!("[LSP HOVER] request NOT sent, cleared anchor");
            }
        } else {
            // 延迟时间未到，放回待处理队列等待下次检查
            (*lsp_hover_pending) = Some(pending);
        }
    }

    // 检查悬停窗口是否需要自动隐藏
    if let Some(deadline) = (*lsp_hover_hide_deadline)
        && now >= deadline // 已超过隐藏截止时间
        && !lsp_overlay.hover_interactive
    // 用户未在与悬停窗口交互
    {
        clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
    }
}

pub fn handle_lsp_hover_from_mouse(
    active_preview_path: &mut Option<String>,
    lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    lsp_hover_hide_deadline: &mut Option<Instant>,
    lsp_hover_pending: &mut Option<super::LspHoverPending>,
    lsp_overlay: &mut iced_code_editor::LspOverlayState,
    lsp_overlay_path: &mut Option<String>,
    preview_nav_popup: &mut Option<(String, f32, f32, Vec<(String, bool)>)>,
    preview_tabs: &mut Vec<PreviewTab>,
    show_preview_context_menu: &mut bool,
    point: iced::Point,
) {
    // 如果显示了上下文菜单或导航弹窗，不触发悬停提示
    if (*show_preview_context_menu) || preview_nav_popup.is_some() {
        return;
    }

    // 获取当前活动的预览路径，没有则直接返回
    let Some(path) = active_preview_path.clone() else {
        tracing::debug!("[LSP HOVER] handle_lsp_hover_from_mouse: no active_preview_path");
        return;
    };

    // 处理用户正在与悬停窗口交互的情况
    if lsp_overlay.hover_interactive {
        // 如果悬停窗口不可见或路径不匹配，解除交互锁定
        if !lsp_overlay.hover_visible || lsp_overlay_path.as_ref() != Some(&path) {
            lsp_overlay.hover_interactive = false;
            (*lsp_hover_hide_deadline) = None;
            (*lsp_hover_pending) = None;
        } else {
            // 用户正在交互，保持显示，不处理新的悬停请求
            tracing::debug!("[LSP HOVER] hover_interactive=true, skipping");
            return;
        }
    }

    // 根据鼠标位置查找可悬停的代码元素锚点
    let anchor =
        if let Some(tab) = preview_tabs.iter().find(|t| t.path.as_str() == path.as_str()) {
            tab.editor.lsp_hover_anchor_at_point(point)
        } else {
            None
        };

    // 如果没有找到锚点，处理隐藏逻辑
    let Some((position, anchor_point)) = anchor else {
        tracing::debug!("[LSP HOVER] no anchor at point {:?}", point);
        // 如果当前路径的悬停窗口仍然可见，保持显示（可能鼠标还在同一个元素上）
        if lsp_overlay.hover_visible && lsp_overlay_path.as_ref() == Some(&path) {
            return;
        }
        // 如果悬停窗口可见但鼠标已移到无效位置，设置延迟隐藏
        if lsp_overlay.hover_visible {
            (*lsp_hover_hide_deadline) = Some(Instant::now() + Duration::from_millis(400));
        }
        return;
    };

    tracing::debug!(
        "[LSP HOVER] got anchor: position={:?}, anchor_point={:?}",
        position,
        anchor_point
    );

    // 鼠标在同一符号内轻微移动时保持已有防抖计时，避免请求被无限推迟。
    if let Some((last_path, last_position)) = lsp_hover_anchor.as_ref()
        && last_path == &path
        && last_position.line == position.line
        && last_position.character == position.character
    {
        return;
    }

    // 更新悬停锚点（用于防抖，避免重复查询）
    (*lsp_hover_anchor) = Some((path.clone(), position));
    lsp_overlay.hover_interactive = false;

    // 创建待处理的悬停请求，设置 400 毫秒的延迟
    (*lsp_hover_pending) = Some(super::LspHoverPending {
        path,
        position,
        point: anchor_point,
        ready_at: Instant::now() + Duration::from_millis(400),
    });

    // 清除之前的隐藏计时器（因为有了新的悬停请求）
    (*lsp_hover_hide_deadline) = None;
}
