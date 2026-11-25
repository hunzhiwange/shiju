use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use desktop_terminal::workspace::{SavedLayout, read_json, write_json};
use gateway_endpoint::GatewayEndpoint;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_api_types::cli_workbench::CliWorkbenchSession;

use super::{Project, Surface, Workbench};

#[derive(Default)]
pub(super) struct Store {
    path: Option<PathBuf>,
    saved: Workspace,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
struct Workspace {
    selected: Option<String>,
    projects: BTreeMap<String, ProjectLayout>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ProjectLayout {
    active: u64,
    groups: BTreeMap<u64, SavedLayout<String>>,
    names: BTreeMap<u64, String>,
    hidden: BTreeSet<String>,
    tab_names: BTreeMap<String, String>,
}

impl Store {
    pub(super) fn open(endpoint: &GatewayEndpoint, surface: Surface) -> Self {
        if cfg!(test) {
            return Self::default();
        }
        let Some(dirs) = crate::app::project_dirs() else { return Self::default() };
        let tenant = endpoint.tenant_scope();
        // The key scopes view preferences without including authentication material.
        let scope = serde_json::json!([
            endpoint.normalized_host(),
            endpoint.port,
            tenant.platform_id(),
            tenant.company_id(),
            endpoint.user_id(),
        ]);
        let scope = hex::encode(Sha256::digest(scope.to_string().as_bytes()));
        let file = match surface {
            Surface::Workbench => "workbench.json",
            Surface::AppTerminal => "app-terminal.json",
            Surface::Terminal => "terminal-ssh.json",
        };
        let path = dirs.data_local_dir().join("workbench-layouts").join(scope).join(file);
        Self::load(path)
    }

    pub(super) fn load(path: PathBuf) -> Self {
        let saved = match read_json(&path) {
            Ok(saved) => saved.unwrap_or_default(),
            Err(error) => {
                tracing::warn!(kind = ?error.kind(), "failed to read workbench layout");
                Workspace::default()
            }
        };
        Self { path: Some(path), saved }
    }

    pub(super) fn selected(&self) -> Option<&str> {
        self.saved.selected.as_deref()
    }

    pub(super) fn merge_sessions(
        &self,
        id: &str,
        project: &mut Project,
        sessions: Vec<CliWorkbenchSession>,
    ) {
        project.merge_sessions(sessions);
        if !project.layout_loaded {
            if let Some(layout) = self.saved.projects.get(id) {
                layout.restore(project);
            }
            project.layout_loaded = true;
        }
    }
}

impl Workbench {
    pub(in crate::app) fn checkpoint_layout(&mut self) {
        let Some(path) = &self.workspace.path else { return };
        let mut next = self.workspace.saved.clone();
        if let Some(selected) = &self.selected {
            next.selected = Some(selected.clone());
        }
        for (id, project) in &self.projects {
            if project.layout_loaded {
                next.projects.insert(id.clone(), ProjectLayout::capture(project));
            }
        }
        if next != self.workspace.saved {
            match write_json(path, &next) {
                Ok(()) => self.workspace.saved = next,
                Err(error) => {
                    tracing::warn!(kind = ?error.kind(), "failed to save workbench layout")
                }
            }
        }
    }
}

impl ProjectLayout {
    fn capture(project: &Project) -> Self {
        let keys: HashMap<_, _> = project.tabs.iter().map(|(id, tab)| (tab.view_id, id)).collect();
        let capture = |panes| SavedLayout::capture(panes, |id| keys[&id].to_string());
        let mut groups: BTreeMap<_, _> =
            project.parked_layouts.iter().map(|(&id, panes)| (id, capture(panes))).collect();
        groups.insert(project.active_layout, capture(&project.panes));
        Self {
            active: project.active_layout,
            groups,
            names: project.layout_names.clone(),
            hidden: project
                .tabs
                .iter()
                .filter(|(_, tab)| tab.hidden)
                .map(|(id, _)| id.clone())
                .collect(),
            tab_names: project
                .tabs
                .iter()
                .filter_map(|(id, tab)| Some((id.clone(), tab.name.clone()?)))
                .collect(),
        }
    }

    fn restore(&self, project: &mut Project) {
        if !self.groups.contains_key(&self.active) || self.groups.keys().any(|&id| id == u64::MAX) {
            return;
        }
        for (id, tab) in &mut project.tabs {
            tab.hidden = self.hidden.contains(id);
            tab.name = self.tab_names.get(id).cloned();
        }
        let mut remaining: HashSet<_> =
            project.tabs.values().filter(|tab| !tab.hidden).map(|tab| tab.view_id).collect();
        let resolve = |id: &String| project.tabs.get(id).map(|tab| tab.view_id);
        project.panes = self.groups[&self.active].restore(resolve, &mut remaining);
        project.parked_layouts = self
            .groups
            .iter()
            .filter(|(id, _)| **id != self.active)
            .map(|(&id, group)| (id, group.restore(resolve, &mut remaining)))
            .collect();
        project.active_layout = self.active;
        project.next_layout = self.groups.keys().next_back().expect("active group exists") + 1;
        project.layout_names = self
            .names
            .iter()
            .filter(|(id, _)| self.groups.contains_key(*id))
            .map(|(&id, name)| (id, name.clone()))
            .collect();
        let unplaced: Vec<_> = project
            .tabs
            .values()
            .map(|tab| tab.view_id)
            .filter(|id| remaining.contains(id))
            .collect();
        for tab in unplaced {
            project.add_discovered_tab(tab);
        }
    }
}
