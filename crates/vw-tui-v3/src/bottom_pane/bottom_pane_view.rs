use crate::render::renderable::Renderable;
use crate::vibe::action::V3Action;
use crossterm::event::KeyEvent;

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
