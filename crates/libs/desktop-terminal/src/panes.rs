//! 窗格布局及其标签归属。PTY 实例由 TerminalState 持有。


use iced::widget::Id;
use iced::widget::pane_grid::{Axis, Direction, Node, Pane, ResizeEvent, State, Target};


pub const PANE_SPACING: f32 = 1.0;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalPane {
    pub tabs: Vec<u64>,
    pub active_id: Option<u64>,
}

pub struct TerminalPanes {
    pub instance: Id,
    pub grid: State<TerminalPane>,
    pub focused: Pane,
    pub split_menu: Option<Pane>,
}

impl Default for TerminalPanes {
    fn default() -> Self {
        let (grid, focused) = State::new(TerminalPane::default());
        Self { instance: Id::unique(), grid, focused, split_menu: None }
    }
}

impl TerminalPanes {
    pub fn add(&mut self, pane: Pane, id: u64) {
        let pane_state = self.grid.get_mut(pane).expect("existing terminal pane");
        pane_state.tabs.push(id);
        pane_state.active_id = Some(id);
        self.focused = pane;
    }

    fn owner(&self, id: u64) -> Option<Pane> {
        self.grid.iter().find_map(|(pane, state)| state.tabs.contains(&id).then_some(*pane))
    }

    pub fn select(&mut self, id: u64) -> bool {
        let Some(pane) = self.owner(id) else {
            return false;
        };
        if self.grid.maximized().is_some_and(|maximized| maximized != pane) {
            self.grid.restore();
        }
        self.grid.get_mut(pane).expect("tab owner").active_id = Some(id);
        self.focused = pane;
        true
    }

    pub fn remove(&mut self, id: u64) -> Option<u64> {
        if let Some(pane) = self.owner(id) {
            let state = self.grid.get_mut(pane).expect("tab owner");
            state.tabs.retain(|tab| *tab != id);
            if state.active_id == Some(id) {
                state.active_id = state.tabs.first().copied();
            }
            if state.tabs.is_empty()
                && let Some((_, sibling)) = self.grid.close(pane)
            {
                if self.focused == pane {
                    self.focused = sibling;
                }
                if self.split_menu == Some(pane) {
                    self.split_menu = None;
                }
            }
        }
        self.grid.get(self.focused).and_then(|pane| pane.active_id)
    }

    pub fn is_visible_tab(&self, id: u64) -> bool {
        self.grid.iter().any(|(key, pane)| {
            self.grid.maximized().is_none_or(|maximized| maximized == *key)
                && pane.active_id == Some(id)
        })
    }

    pub fn resize(&mut self, event: ResizeEvent) {
        if event.ratio.is_finite() {
            self.grid.resize(event.split, event.ratio.clamp(0.0, 1.0));
        }
    }

    pub fn dock(&mut self, pane: Pane, target: Target) {
        if self.grid.len() < 2
            || self.grid.get(pane).is_none()
            || matches!(target, Target::Pane(id, _) if id == pane || self.grid.get(id).is_none())
        {
            return;
        }

        let mut candidate = self.grid.clone();
        candidate.drop(pane, target);
        if candidate.panes == self.grid.panes
            && !same_arrangement(candidate.layout(), self.grid.layout())
        {
            self.grid = candidate;
        }
    }
}

// Re-docking a pane on its existing edge must preserve the user's split ratios.
fn same_arrangement(first: &Node, second: &Node) -> bool {
    match (first, second) {
        (Node::Pane(first), Node::Pane(second)) => first == second,
        (
            Node::Split { axis, a, b, .. },
            Node::Split { axis: other_axis, a: other_a, b: other_b, .. },
        ) => axis == other_axis && same_arrangement(a, other_a) && same_arrangement(b, other_b),
        _ => false,
    }
}

pub use crate::state::SplitError;
