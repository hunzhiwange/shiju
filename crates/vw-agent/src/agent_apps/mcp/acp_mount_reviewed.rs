use super::*;
use crate::model_relay::RelayBinding;
use acp_client::ApprovalValidity;
use agent_acp::mount_store::MountRecord;
use agent_app_services::mcp::{McpCatalogStore, reviewed_stdio::ReviewedProjectStdio};
use agent_database::run_sync_database;
use std::{path::Path, sync::Arc, time::Instant};
use vw_api_transport::ProjectScope;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReviewedStdioSource {
    Project,
    Mounted,
}

pub(crate) struct ReviewedStdioMount {
    pub(crate) source: ReviewedStdioSource,
    pub(crate) server: McpServer,
    pub(crate) review: Arc<ReviewedProjectStdio>,
    // Share the current check across discovery, approval issuance and dispatch.
    pub(crate) current: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl ReviewedStdioMount {
    pub(crate) fn guard(&self, approval: ApprovalValidity) -> ApprovalValidity {
        let current = self.current.clone();
        Box::new(move || current() && approval())
    }
}

const MAX_REVIEWED_STDIO_SERVICES: usize = 16;

#[derive(Clone, Copy)]
enum SelectionMode {
    Single,
    List,
}

impl SelectionMode {
    fn limit(self) -> usize {
        match self {
            Self::Single => 1,
            Self::List => MAX_REVIEWED_STDIO_SERVICES,
        }
    }
}

/// Resolve the current session's committed reference or an explicit project selection.
pub(crate) async fn reviewed_linux_stdio(
    launch: Arc<RelayBinding>,
    cwd: &Path,
    project_selected: &[McpServer],
) -> anyhow::Result<Option<ReviewedStdioMount>> {
    let mut services =
        resolve_for_session(launch, cwd, project_selected, SelectionMode::Single).await?;
    Ok(services.pop())
}

/// Resolve project and session selections for one projection. Every returned
/// service shares a validity check for the full set of reviewed revisions.
pub(crate) async fn reviewed_linux_stdio_list(
    launch: Arc<RelayBinding>,
    cwd: &Path,
    project_selected: &[McpServer],
) -> anyhow::Result<Vec<ReviewedStdioMount>> {
    resolve_for_session(launch, cwd, project_selected, SelectionMode::List).await
}

async fn resolve_for_session(
    launch: Arc<RelayBinding>,
    cwd: &Path,
    project_selected: &[McpServer],
    mode: SelectionMode,
) -> anyhow::Result<Vec<ReviewedStdioMount>> {
    let identity = launch.identity();
    let owner = ProjectScope::new(identity.tenant.clone(), identity.project.as_str().into())?;
    let database = || GlobalDatabase::for_home(&crate::global::paths().home);
    let Some(context) = crate::tools::current_tool_use_context() else {
        return resolve(launch, owner, database, project_selected, mode, || Ok(None));
    };
    let session = context.session();
    let Some((session_scope, adapter)) = identity.session.rsplit_once(':') else {
        anyhow::bail!("Invalid reviewed MCP relay owner");
    };
    anyhow::ensure!(
        !adapter.is_empty()
            && session_scope
                .rsplit_once(":acp-source-")
                .is_some_and(|(bound, generation)| bound == session && !generation.is_empty())
            && context.tenant_scope() == identity.tenant,
        "Reviewed MCP relay belongs to another session"
    );
    let scope = SessionScope::new(identity.tenant.clone(), cwd)?;
    let record_scope = run_sync_database(|| {
        session_store::ui_store::load_agent_session_strict(
            owner.tenant_scope(),
            owner.project_id().as_ref(),
            session,
        )
    })?;
    if let Some(record) = &record_scope {
        anyhow::ensure!(
            record.id == session
                && record.project_id == identity.project
                && Path::new(&record.directory).canonicalize()? == scope.directory(),
            "Reviewed MCP session ownership mismatch"
        );
    }
    let read = agent_acp::mount_store::reader(&scope, session).await;
    // A project-only launch can precede publication of the native session.
    let owns_session = record_scope.is_some();
    resolve(launch, owner, database, project_selected, mode, move || {
        let record = read()?;
        if !owns_session && record.is_some() {
            return Err(BackendError::InvalidRequest(
                "Cannot establish reviewed MCP session ownership".into(),
            ));
        }
        Ok(record)
    })
}

struct ReviewedSelection {
    server: McpServer,
    proof: Arc<ReviewedProjectStdio>,
    mount: Option<McpCatalogMount>,
}

fn resolve(
    launch: Arc<RelayBinding>,
    owner: ProjectScope,
    database: impl FnOnce() -> anyhow::Result<GlobalDatabase>,
    project_selected: &[McpServer],
    mode: SelectionMode,
    read: impl Fn() -> Result<Option<MountRecord>, BackendError> + Send + Sync + 'static,
) -> anyhow::Result<Vec<ReviewedStdioMount>> {
    anyhow::ensure!(
        project_selected.len() <= mode.limit(),
        "Too many reviewed project MCP selections"
    );
    let committed = read()?;
    let refs = committed.as_ref().map(|record| record.mcp_catalog.as_slice()).unwrap_or_default();
    if refs.is_empty() && project_selected.is_empty() {
        return Ok(Vec::new());
    }
    let mut names = HashSet::new();
    for server in project_selected {
        let name = mcp_client::server_name(server);
        anyhow::ensure!(
            names.insert(name)
                && committed.as_ref().is_none_or(|record| !record.has_mcp_name(name)),
            "Reviewed MCP name conflicts across project and session selections"
        );
    }
    for mount in refs {
        anyhow::ensure!(names.insert(mount.record.id.0.as_str()), "Duplicate reviewed MCP name");
    }
    let database = database()?;
    let load = |error| anyhow::anyhow!("reviewed MCP selection: {error:?}");
    let store = run_sync_database(|| McpCatalogStore::new(database).map_err(load))?;
    let mut selections = Vec::new();
    for mount in refs {
        let reviewed = run_sync_database(|| {
            store.reviewed_mounted_stdio(&owner, mount).map_or_else(
                |error| match error {
                    vw_api_mcp_management::McpFailureCode::NotFound
                    | vw_api_mcp_management::McpFailureCode::PermissionDenied
                    | vw_api_mcp_management::McpFailureCode::StaleRevision => Ok(None),
                    error => Err(load(error)),
                },
                |service| Ok(Some(service)),
            )
        })?;
        if let Some((proof, server)) = reviewed {
            anyhow::ensure!(
                selections.len() + project_selected.len() < mode.limit(),
                "Too many reviewed MCP selections"
            );
            selections.push(ReviewedSelection {
                server: McpServer::Stdio(server),
                proof: Arc::new(proof),
                mount: Some(mount.clone()),
            });
        }
    }
    for server in project_selected {
        let proof = run_sync_database(|| {
            store.reviewed_project_stdio(&owner, std::slice::from_ref(server)).map_err(load)
        })?;
        selections.push(ReviewedSelection {
            server: server.clone(),
            proof: Arc::new(proof),
            mount: None,
        });
    }
    if selections.is_empty() {
        return Ok(Vec::new());
    }

    let mount_revision = committed.as_ref().map(|record| record.revision);
    let pin_mount_revision = matches!(mode, SelectionMode::List)
        || selections.iter().any(|selection| selection.mount.is_some());
    let pins: Vec<_> = selections
        .iter()
        .map(|selection| {
            (
                mcp_client::server_name(&selection.server).to_owned(),
                selection.mount.clone(),
                selection.proof.clone(),
            )
        })
        .collect();
    let current = Arc::new(move || {
        if launch.cancelled().is_cancelled()
            || launch.expires().is_some_and(|expiry| Instant::now() >= expiry)
        {
            return false;
        }
        let Ok(record) = read() else { return false };
        if pin_mount_revision && record.as_ref().map(|record| record.revision) != mount_revision {
            return false;
        }
        let same_selections = pins.iter().all(|(name, mount, _)| match mount {
            Some(mount) => record.as_ref().is_some_and(|record| {
                record.mcp_catalog.iter().filter(|entry| &entry.record.id.0 == name).eq([mount])
            }),
            None => record.as_ref().is_none_or(|record| !record.has_mcp_name(name)),
        });
        same_selections
            && run_sync_database(|| {
                Ok(pins
                    .iter()
                    .all(|(_, _, proof)| proof.is_current(&store, &owner).unwrap_or(false)))
            })
            .unwrap_or(false)
    });
    anyhow::ensure!(current(), "Reviewed MCP selection was revoked");
    Ok(selections
        .into_iter()
        .map(|selection| ReviewedStdioMount {
            source: if selection.mount.is_some() {
                ReviewedStdioSource::Mounted
            } else {
                ReviewedStdioSource::Project
            },
            server: selection.server,
            review: selection.proof,
            current: current.clone(),
        })
        .collect())
}

#[cfg(test)]
#[path = "acp_mount_reviewed_tests.rs"]
mod tests;
