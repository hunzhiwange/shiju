//! Session projection state and pin persistence.
use std::{cell::RefCell, collections::HashSet, sync::Arc};
pub struct SessionUiProjection<Agent, Sidebar> {
    pub hover_details: std::collections::HashMap<String, super::hover_details::SessionHoverDetails>,
    pub hover_details_loading: HashSet<String>,
    pub agent_sessions: RefCell<Option<Arc<Vec<Agent>>>>,
    pub agent_session_key: RefCell<(Option<String>, usize)>,
    pub sidebar: RefCell<Option<Arc<Sidebar>>>,
    pub pinned: HashSet<(String, String)>,
    pub pending_pins: HashSet<(String, String)>,
    pub pins_generation: u64,
}

impl<Agent, Sidebar> Default for SessionUiProjection<Agent, Sidebar> {
    fn default() -> Self {
        Self {
            hover_details: Default::default(),
            hover_details_loading: Default::default(),
            agent_sessions: Default::default(),
            agent_session_key: Default::default(),
            sidebar: Default::default(),
            pinned: Default::default(),
            pending_pins: Default::default(),
            pins_generation: 0,
        }
    }
}

impl<Agent, Sidebar> SessionUiProjection<Agent, Sidebar> {
    pub fn apply_pin(&mut self, key: (String, String), pinned: bool) {
        if pinned {
            self.pinned.insert(key);
        } else {
            self.pinned.remove(&key);
        }
        self.sidebar.get_mut().take();
    }
    pub fn apply_pins(
        &mut self,
        generation: u64,
        result: Result<HashSet<(String, String)>, String>,
    ) {
        if generation != self.pins_generation {
            return;
        }
        match result {
            Ok(pinned) => {
                if pinned != self.pinned {
                    self.pinned = pinned;
                    self.sidebar.get_mut().take();
                }
            }
            Err(error) => tracing::warn!(%error, "Failed to refresh pinned sessions"),
        }
    }
}
pub fn write_pin(project_id: &str, id: &str, pinned: bool) -> Result<bool, String> {
    let data_dir = pins_data_dir()?;
    let scope = pins_scope(project_id);
    session_artifact_store::set_thread_pinned(&data_dir, &scope, id, pinned)
        .map_err(|error| error.to_string())?;
    Ok(pinned)
}

pub fn pins_data_dir() -> Result<std::path::PathBuf, String> {
    let home = directories::BaseDirs::new().ok_or_else(|| "无法定位置顶会话目录".to_string())?;
    Ok(config_runtime_paths::home_config_dir(home.home_dir()))
}

pub fn pins_scope(project: &str) -> String {
    let tenant = vw_api_transport::TenantScope::default();
    session_artifact_store::pinned_threads_scope(
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
        project,
    )
}

pub fn load_pins(projects: HashSet<String>) -> Result<HashSet<(String, String)>, String> {
    let data_dir = pins_data_dir()?;
    let mut pinned = HashSet::new();
    for project in projects {
        let scope = pins_scope(&project);
        let ids = session_artifact_store::load_pinned_threads(&data_dir, &scope)
            .map_err(|error| error.to_string())?;
        pinned.extend(ids.into_iter().map(|id| (project.clone(), id)));
    }
    Ok(pinned)
}

#[cfg(test)]
#[path = "ui_projection_tests.rs"]
mod tests;

/// Host message classification determines which session projection became stale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionChange {
    Preserve,
    Agent,
    All,
}
impl<Agent, Sidebar> SessionUiProjection<Agent, Sidebar> {
    /// Returns whether a cached sidebar was discarded, for host diagnostics.
    pub fn prepare(&mut self, change: ProjectionChange) -> bool {
        match change {
            ProjectionChange::Preserve => false,
            ProjectionChange::Agent => {
                self.agent_sessions.get_mut().take();
                false
            }
            ProjectionChange::All => {
                self.agent_sessions.get_mut().take();
                self.sidebar.get_mut().take().is_some()
            }
        }
    }
    pub fn begin_pin(&mut self, key: (String, String)) -> bool {
        self.pending_pins.insert(key)
    }
    pub fn begin_pins_refresh(&mut self) -> u64 {
        self.pins_generation = self.pins_generation.wrapping_add(1);
        self.pins_generation
    }
}
