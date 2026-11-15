use super::*;
#[derive(Debug, PartialEq, Eq)]
pub enum SplitError {
    MissingPane,
    Spawn,
}

impl<S: Copy + Into<&'static str>> TerminalState<S> {
    pub fn split_terminal(
        &mut self,
        pane: Pane,
        direction: Direction,
        cwd: Option<PathBuf>,
    ) -> Result<(), SplitError> {
        if self.panes.grid.get(pane).and_then(|pane| pane.active_id).is_none() {
            return Err(SplitError::MissingPane);
        }
        let axis = match direction {
            Direction::Left | Direction::Right => Axis::Vertical,
            Direction::Up | Direction::Down => Axis::Horizontal,
        };
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
