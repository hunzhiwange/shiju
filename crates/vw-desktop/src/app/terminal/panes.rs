//! 窗格布局及其标签归属。PTY 实例由 TerminalState 持有。

use std::path::PathBuf;

use iced::Size;
use iced::widget::Id;
use iced::widget::pane_grid::{Axis, Direction, Node, Pane, ResizeEvent, State, Target};

use super::TerminalState;

pub(crate) const PANE_SPACING: f32 = 4.0;
const MIN_PANE_WIDTH: f32 = 160.0;
const MIN_PANE_HEIGHT: f32 = 88.0;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TerminalPane {
    pub(crate) tabs: Vec<u64>,
    pub(crate) active_id: Option<u64>,
}

pub(crate) struct TerminalPanes {
    pub(crate) instance: Id,
    pub(crate) grid: State<TerminalPane>,
    pub(crate) focused: Pane,
    pub(crate) split_menu: Option<Pane>,
}

impl Default for TerminalPanes {
    fn default() -> Self {
        let (grid, focused) = State::new(TerminalPane::default());
        Self { instance: Id::unique(), grid, focused, split_menu: None }
    }
}

impl TerminalPanes {
    pub(super) fn add(&mut self, pane: Pane, id: u64) {
        let pane_state = self.grid.get_mut(pane).expect("existing terminal pane");
        pane_state.tabs.push(id);
        pane_state.active_id = Some(id);
        self.focused = pane;
    }

    fn owner(&self, id: u64) -> Option<Pane> {
        self.grid.iter().find_map(|(pane, state)| state.tabs.contains(&id).then_some(*pane))
    }

    pub(super) fn select(&mut self, id: u64) -> bool {
        let Some(pane) = self.owner(id) else {
            return false;
        };
        self.grid.get_mut(pane).expect("tab owner").active_id = Some(id);
        self.focused = pane;
        true
    }

    pub(super) fn remove(&mut self, id: u64) -> Option<u64> {
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

    pub(crate) fn is_visible_tab(&self, id: u64) -> bool {
        self.grid.iter().any(|(_, pane)| pane.active_id == Some(id))
    }

    pub(crate) fn resize(&mut self, event: ResizeEvent, size: Size) {
        if !event.ratio.is_finite() {
            return;
        }
        let splits = self.grid.layout().split_regions(PANE_SPACING, 0.0, size);
        let Some(&(_, _, current)) = splits.get(&event.split) else {
            return;
        };
        let mut candidate = self.grid.clone();
        let requested = event.ratio.clamp(0.0, 1.0);
        candidate.resize(event.split, requested);
        if fits(&candidate, size) {
            self.grid.resize(event.split, requested);
        } else if fits(&self.grid, size) {
            // A child can contain more splits. Use the rendered leaf bounds to
            // find the nearest legal divider position, including Iced's rounding.
            let (mut valid, mut invalid) = (current, requested);
            for _ in 0..16 {
                let ratio = (valid + invalid) / 2.0;
                candidate.resize(event.split, ratio);
                if fits(&candidate, size) {
                    valid = ratio;
                } else {
                    invalid = ratio;
                }
            }
            self.grid.resize(event.split, valid);
        }
    }

    pub(crate) fn dock(&mut self, pane: Pane, target: Target, size: Size) {
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
            && fits(&candidate, size)
        {
            self.grid = candidate;
        }
    }
}

fn fits(grid: &State<TerminalPane>, size: Size) -> bool {
    let regions = grid.layout().pane_regions(PANE_SPACING, 0.0, size);
    regions.keys().eq(grid.panes.keys())
        && regions.values().all(|region| {
            region.width.is_finite()
                && region.height.is_finite()
                && region.width >= MIN_PANE_WIDTH
                && region.height >= MIN_PANE_HEIGHT
        })
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SplitError {
    MissingPane,
    InsufficientSpace,
    Spawn,
}

impl TerminalState {
    pub(crate) fn split_terminal(
        &mut self,
        pane: Pane,
        direction: Direction,
        size: Size,
        cwd: Option<PathBuf>,
    ) -> Result<(), SplitError> {
        let region = self
            .panes
            .grid
            .layout()
            .pane_regions(PANE_SPACING, 0.0, size)
            .remove(&pane)
            .filter(|_| self.panes.grid.get(pane).is_some_and(|p| p.active_id.is_some()))
            .ok_or(SplitError::MissingPane)?;
        let axis = match direction {
            Direction::Left | Direction::Right => Axis::Vertical,
            Direction::Up | Direction::Down => Axis::Horizontal,
        };
        let (min_width, min_height) = match axis {
            Axis::Vertical => (2.0 * MIN_PANE_WIDTH + PANE_SPACING, MIN_PANE_HEIGHT),
            Axis::Horizontal => (MIN_PANE_WIDTH, 2.0 * MIN_PANE_HEIGHT + PANE_SPACING),
        };
        if !(region.width >= min_width && region.height >= min_height) {
            return Err(SplitError::InsufficientSpace);
        }

        let mut grid = self.panes.grid.clone();
        let (new_pane, _) =
            grid.split(axis, pane, TerminalPane::default()).ok_or(SplitError::MissingPane)?;
        if matches!(direction, Direction::Left | Direction::Up) {
            grid.swap(pane, new_pane);
        }

        // Alacritty can silently ignore an invalid working directory on Unix.
        if cwd.as_ref().is_some_and(|path| !path.is_dir()) {
            return Err(SplitError::Spawn);
        }
        let tab = self.spawn_interactive_terminal(cwd).ok_or(SplitError::Spawn)?;
        self.next_id = self.next_id.wrapping_add(1);
        self.panes.grid = grid;
        self.push_tab(new_pane, tab);
        Ok(())
    }
}
