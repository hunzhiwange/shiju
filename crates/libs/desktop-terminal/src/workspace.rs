//! Local project terminal layout. Restored tabs start interactive shells.
use crate::{TerminalState, panes::TerminalPane};
use iced::widget::pane_grid::{Axis, Configuration, Node, State};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Serialize, Deserialize)]
pub(crate) struct Workspace {
    visible: bool,
    height: f32,
    active_tab: Option<u64>,
    maximized_tab: Option<u64>,
    tabs: Vec<Tab>,
    layout: Layout,
}

#[derive(PartialEq, Serialize, Deserialize)]
struct Tab {
    id: u64,
    title: String,
    default_title: bool,
    shell: String,
    directory: Option<OsString>,
}

#[derive(PartialEq, Serialize, Deserialize)]
enum Layout {
    Pane { tabs: Vec<u64>, active_tab: Option<u64> },
    Split { vertical: bool, ratio: f32, first: Box<Self>, second: Box<Self> },
}

impl Layout {
    fn capture(grid: &State<TerminalPane>, node: &Node) -> Self {
        match node {
            Node::Pane(id) => {
                let pane = grid.get(*id).expect("pane in terminal layout");
                Self::Pane { tabs: pane.tabs.clone(), active_tab: pane.active_id }
            }
            Node::Split { axis, ratio, a, b, .. } => Self::Split {
                vertical: *axis == Axis::Vertical,
                ratio: *ratio,
                first: Box::new(Self::capture(grid, a)),
                second: Box::new(Self::capture(grid, b)),
            },
        }
    }

    fn collect_tabs(&self, ids: &mut HashSet<u64>) {
        match self {
            Self::Pane { tabs, .. } => ids.extend(tabs.iter().copied()),
            Self::Split { first, second, .. } => {
                first.collect_tabs(ids);
                second.collect_tabs(ids);
            }
        }
    }

    fn configuration(&self, remaining: &mut HashSet<u64>) -> Option<Configuration<TerminalPane>> {
        match self {
            Self::Pane { tabs, active_tab } => {
                let tabs: Vec<_> = tabs.iter().copied().filter(|id| remaining.remove(id)).collect();
                let first = tabs.first().copied()?;
                let active_id = active_tab.filter(|id| tabs.contains(id)).or(Some(first));
                Some(Configuration::Pane(TerminalPane { tabs, active_id }))
            }
            Self::Split { vertical, ratio, first, second } => {
                let first = first.configuration(remaining);
                let second = second.configuration(remaining);
                match (first, second) {
                    (Some(a), Some(b)) => Some(Configuration::Split {
                        axis: if *vertical { Axis::Vertical } else { Axis::Horizontal },
                        ratio: ratio.clamp(0.0, 1.0),
                        a: Box::new(a),
                        b: Box::new(b),
                    }),
                    (a, b) => a.or(b),
                }
            }
        }
    }
}

impl<S: Copy + Into<&'static str>> TerminalState<S> {
    pub fn has_saved_workspace(&self) -> bool {
        self.saved_workspace.is_some()
    }

    pub fn needs_workspace_checkpoint(&self) -> bool {
        !self.tabs.is_empty()
            || self.saved_workspace.as_ref().is_some_and(|workspace| !workspace.tabs.is_empty())
    }

    pub fn save_workspace(&mut self, path: &Path) -> io::Result<()> {
        if self.tabs.is_empty() && self.saved_workspace.is_none() {
            return Ok(());
        }
        let grid = &self.panes.grid;
        let workspace = Workspace {
            visible: self.is_visible,
            height: self.height,
            active_tab: self.active_id,
            maximized_tab: grid.maximized().and_then(|id| grid.get(id)?.active_id),
            tabs: self
                .tabs
                .iter()
                .map(|tab| Tab {
                    id: tab.id,
                    title: tab.title.clone(),
                    default_title: tab.default_title,
                    shell: tab.shell_program.to_owned(),
                    directory: tab.term.working_directory().map(PathBuf::into_os_string),
                })
                .collect(),
            layout: Layout::capture(grid, grid.layout()),
        };
        if self.saved_workspace.as_ref() != Some(&workspace) {
            write_json(path, &workspace)?;
            self.saved_workspace = Some(workspace);
        }
        Ok(())
    }

    /// Load onto a newly created project state before its default shell is started.
    pub fn restore_workspace(&mut self, path: &Path, project: &Path) -> io::Result<bool> {
        let Some(workspace): Option<Workspace> = read_json(path)? else {
            return Ok(false);
        };
        let mut placed = HashSet::new();
        workspace.layout.collect_tabs(&mut placed);

        for saved in &workspace.tabs {
            if saved.id == u64::MAX || !placed.remove(&saved.id) {
                continue;
            }
            let cwd = saved
                .directory
                .as_deref()
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute() && dir.is_dir())
                .unwrap_or_else(|| project.to_owned());
            let shell = match saved.shell.as_str() {
                "/bin/bash" => "/bin/bash",
                "/bin/zsh" => "/bin/zsh",
                _ => self.shell.into(),
            };
            let tab = self.spawn_interactive_tab(saved.id, shell, Some(cwd.clone()));
            let tab = tab.or_else(|| {
                let default_shell = self.shell.into();
                if shell == default_shell {
                    return None;
                }
                self.spawn_interactive_tab(saved.id, default_shell, Some(cwd))
            });
            if let Some(mut tab) = tab {
                tab.title.clone_from(&saved.title);
                tab.default_title = saved.default_title;
                self.next_id = self.next_id.max(tab.id + 1);
                self.tabs.push(tab);
            }
        }

        let mut remaining = self.tabs.iter().map(|tab| tab.id).collect();
        if let Some(config) = workspace.layout.configuration(&mut remaining) {
            let mut grid = State::with_configuration(config);
            let owner =
                |id| grid.iter().find(|(_, pane)| pane.tabs.contains(&id)).map(|(key, _)| *key);
            let maximized = workspace.maximized_tab.and_then(owner);
            let focused = workspace
                .active_tab
                .and_then(owner)
                .or(maximized)
                .or_else(|| grid.iter().next().map(|(id, _)| *id))
                .expect("terminal layout has at least one pane");
            self.panes.focused = focused;
            self.active_id = grid.get(focused).and_then(|pane| pane.active_id);
            if let Some(id) = maximized {
                grid.maximize(id);
            }
            self.panes.grid = grid;
        }
        self.set_visible(workspace.visible);
        if workspace.height.is_finite() && workspace.height > 0.0 {
            self.height = workspace.height;
        }
        self.saved_workspace = Some(workspace);
        Ok(true)
    }
}

pub fn workspace_has_tabs(path: &Path) -> io::Result<bool> {
    Ok(read_json::<Workspace>(path)?.is_some_and(|workspace| !workspace.tabs.is_empty()))
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    match std::fs::File::open(path) {
        Ok(file) => serde_json::from_reader(io::BufReader::new(file)).map(Some).map_err(Into::into),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = io::BufWriter::new(file.as_file_mut());
        serde_json::to_writer(&mut writer, value)?;
        writer.flush()?;
    }
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "workspace_tests.rs"]
mod tests;
