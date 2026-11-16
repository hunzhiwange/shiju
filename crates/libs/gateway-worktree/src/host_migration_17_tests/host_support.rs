//! Real library adapters for domain tests; no App/AppState or simulated process lifetime.
//! Session and process actions supplied to worktree-core by the application.
use std::collections::{HashMap, HashSet};
use vw_api_transport::{ProjectScope, TenantScope};
use worktree_core::{Error, host::{Host, ProcessGuard, ProtectionSnapshot}};

pub struct WorktreeHost;
struct Guard { _tree: tool_shell::shell::processes::termination::ProcessTree }
impl ProcessGuard for Guard {}
impl Host for WorktreeHost {
    fn with_tenant_scope<'a>(&self, tenant: TenantScope, future: worktree_core::host::ScopedFuture<'a>) -> worktree_core::host::ScopedFuture<'a> {
        Box::pin(session_store::ui_store::with_tenant_scope(tenant, future))
    }
    fn current_tenant_scope(&self) -> TenantScope {
        session_store::ui_store::current_tenant_scope()
    }
    fn read_snapshot(&self, scope: &ProjectScope) -> Result<ProtectionSnapshot, Error> {
        let invalid = |error: String| Error::Invalid(error);
        let (sessions, foreign_directories) = session_store::ui_store::load_worktree_protection_sessions(scope.tenant_scope())
            .map_err(|error| invalid(error.to_string()))?;
        let mut projects: HashSet<String> = sessions.iter().map(|session| session.project_id.clone()).collect();
        projects.insert(scope.project_id().as_ref().to_owned());
        let pinned = projects.into_iter().map(|project| {
            let pins = session_store::ui_store::load_pinned_threads(scope.tenant_scope(), &project)
                .map_err(|error| invalid(error.to_string()))?;
            Ok((project, pins))
        }).collect::<Result<HashMap<_, _>, Error>>()?;
        let busy = busy_sessions(session_engine::status::cleanup_snapshot().map_err(|error| invalid(error.to_string()))?);
        Ok(ProtectionSnapshot { sessions, foreign_directories, pinned, busy })
    }
    fn spawn(&self, command: &mut tokio::process::Command)
        -> anyhow::Result<(tokio::process::Child, Box<dyn ProcessGuard>)> {
        let (child, guard) = tool_shell::shell::processes::termination::spawn(command)?;
        Ok((child, Box::new(Guard { _tree: guard })))
    }
}

static INITIALIZED: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    worktree_core::host::install(std::sync::Arc::new(WorktreeHost))
        .expect("worktree host is installed only by application composition");
});
pub fn initialize_host() {
    std::sync::LazyLock::force(&INITIALIZED);
}

pub(super) fn busy_sessions(statuses: HashMap<String, session_engine::status::Info>) -> HashSet<String> {
    statuses.into_iter().filter_map(|(id, status)| matches!(status,
        session_engine::status::Info::Busy | session_engine::status::Info::Retry { .. }).then_some(id)).collect()
}
