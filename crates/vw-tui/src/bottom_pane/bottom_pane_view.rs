//! 本模块集中实现 `bottom_pane_view` 在底部面板中的职责。


use crate::render::renderable::Renderable;
use crate::vibe::action::V3Action;
use crossterm::event::KeyEvent;

/// 本地底部面板视图。它只能发射 V3Action，无法访问运行时状态。
/// A local bottom-pane view.  It can emit only a V3Action and cannot access runtime state.
pub(crate) trait BottomPaneView: Renderable {
    fn handle_key_event(&mut self, _event: KeyEvent) -> V3Action {
        V3Action::None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewCompletion {
    Accepted,
    Cancelled,
}
