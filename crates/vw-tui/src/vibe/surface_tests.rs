//! 验证隔离 v3 界面只暴露复制渲染层所需的最小公开表面。
//! 选择浮层清单还作为非安全覆盖层新增变体的显式审计点。

// VibeWindow adaptation: verifies the isolated v3 surface shell.
// VibeWindow 适配：验证隔离后的 v3 界面外壳。

use super::surface::V3Surface;
use crate::bottom_pane::list_selection_view::ListSelectionView;
use crate::vibe::action::{V3_NON_SECURITY_SELECTION_SURFACES, V3Action, V3SelectionItem};
use crossterm::event::{KeyCode, KeyEvent};

#[test]
// 复制 crate 应能在不引入 Codex 运行时的情况下构造并渲染 v3 根界面。
fn copied_crate_exposes_v3_surface_without_codex_runtime() {
    let _surface = V3Surface::new();
}

#[test]
// 每个非安全选择浮层都必须进入穷举清单，并映射为稳定动作。
fn v3_non_security_overlays_are_exhaustive() {
    assert_eq!(
        V3_NON_SECURITY_SELECTION_SURFACES,
        [
            "search",
            "todo",
            "task",
            "command_palette",
            "report",
            "mcp",
            "memory",
            "settings",
            "permissions",
            "integration",
            "background_tasks",
            "agents",
            "workflows",
            "schedules",
        ],
    );
    for surface in V3_NON_SECURITY_SELECTION_SURFACES {
        let mut view = ListSelectionView::from_v3(
            *surface,
            *surface,
            vec![V3SelectionItem::new("provided-row", "Provided row")],
        );
        assert_eq!(
            view.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select {
                surface: (*surface).to_string(),
                item_id: "provided-row".to_string(),
            },
            "overlay {surface} must use the shared V3 selection action boundary",
        );
    }
}
