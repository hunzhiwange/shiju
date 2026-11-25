//! 窗格布局及其标签归属。PTY 实例由 TerminalState 持有。


use iced::widget::Id;
use iced::widget::pane_grid::{Axis, Direction, Node, Pane, Region, ResizeEvent, State, Target};


pub const PANE_SPACING: f32 = 1.0;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalPane {
    pub tabs: Vec<u64>,
    pub active_id: Option<u64>,
}

impl TerminalPane {
    fn remove(&mut self, id: u64) {
        self.tabs.retain(|tab| *tab != id);
        if self.active_id == Some(id) {
            self.active_id = self.tabs.first().copied();
        }
    }
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

    pub fn place(&mut self, pane: Pane, id: u64) {
        if self.grid.get(pane).is_none() {
            return;
        }
        if self.owner(id) == Some(pane) {
            self.select(id);
            return;
        }
        self.remove(id);
        self.add(pane, id);
        self.select(id);
    }

    pub fn remove(&mut self, id: u64) -> Option<u64> {
        if let Some(pane) = self.owner(id) {
            let state = self.grid.get_mut(pane).expect("tab owner");
            state.remove(id);
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

    pub fn split(&mut self, pane: Pane, direction: Direction) -> Option<Pane> {
        split_grid(&mut self.grid, pane, direction)
    }

    pub fn resize(&mut self, event: ResizeEvent) {
        if event.ratio.is_finite() {
            self.grid.resize(event.split, event.ratio.clamp(0.0, 1.0));
        }
    }

    pub fn dock_tab(&mut self, pane: Pane, tab: u64, target: Target) -> bool {
        let Some(source) = self.grid.get(pane) else { return false };
        if self.grid.maximized().is_some() || !source.tabs.contains(&tab)
            || matches!(target, Target::Pane(id, _) if self.grid.get(id).is_none())
        {
            return false;
        }
        if let Target::Pane(destination, Region::Center) = target {
            if pane == destination {
                return false;
            }
            self.place(destination, tab);
            self.split_menu = None;
            return true;
        }

        let mut candidate = self.grid.clone();
        let moving = if source.tabs.len() == 1 {
            if self.grid.len() < 2 || matches!(target, Target::Pane(id, _) if id == pane) {
                return false;
            }
            pane
        } else {
            // Isolate the tab in a leaf so Iced can dock it without moving its siblings.
            let state = TerminalPane { tabs: vec![tab], active_id: Some(tab) };
            let Some((new, _)) = candidate.split(Axis::Vertical, pane, state) else {
                return false;
            };
            candidate.get_mut(pane).expect("source pane exists").remove(tab);
            new
        };
        candidate.drop(moving, target);
        if candidate.get(moving).is_none() || same_arrangement(candidate.layout(), self.grid.layout()) {
            return false;
        }
        self.grid = candidate;
        self.focused = moving;
        self.split_menu = None;
        true
    }
}

pub(crate) fn split_grid(grid: &mut State<TerminalPane>, pane: Pane, direction: Direction) -> Option<Pane> {
    grid.get(pane)?.active_id?;
    let axis = match direction {
        Direction::Left | Direction::Right => Axis::Vertical,
        Direction::Up | Direction::Down => Axis::Horizontal,
    };
    let (new_pane, _) = grid.split(axis, pane, TerminalPane::default())?;
    if matches!(direction, Direction::Left | Direction::Up) {
        grid.swap(pane, new_pane);
    }
    Some(new_pane)
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
