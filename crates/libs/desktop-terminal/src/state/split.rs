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
        let mut grid = self.panes.grid.clone();
        let new_pane = crate::panes::split_grid(&mut grid, pane, direction)
            .ok_or(SplitError::MissingPane)?;

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
