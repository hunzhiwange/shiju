//! Desktop message adapter for the workflow video template modal.

#[cfg(test)]
pub(super) fn modal(state: &WorkflowState) -> Element<'_, Message> {
    crate::view::video_edit::modal(state).map(Message::WorkflowTool)
}

#[cfg(test)]
use super::*;
