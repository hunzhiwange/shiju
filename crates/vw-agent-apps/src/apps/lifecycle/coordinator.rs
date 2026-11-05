//! Serializes application writes in the authorized execution-host process.
//!
//! Authorization and operation ownership remain with the domain caller. Every
//! tenant uses this same host-local coordinator, never a tenant-keyed write lock.
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use vw_api_types::agent_apps::AppId;

struct HostWrites {
    tools: HashMap<AppId, Arc<Mutex<()>>>,
    execution: Arc<Mutex<()>>,
}

fn host_writes() -> &'static HostWrites {
    static WRITES: OnceLock<HostWrites> = OnceLock::new();
    WRITES.get_or_init(|| HostWrites {
        tools: super::super::catalog::catalog()
            .into_iter()
            .filter(|app| app.metadata.cli_command.is_some())
            .map(|app| (app.metadata.id, Arc::new(Mutex::new(()))))
            .collect(),
        execution: Arc::new(Mutex::new(())),
    })
}

/// All handles share host locks, including independently constructed handles.
#[derive(Default)]
pub struct ToolLifecycleCoordinator;

impl ToolLifecycleCoordinator {
    /// Reserves tools before queueing and runs the entire write under one lock.
    ///
    /// The caller must revalidate its plan inside `operation` and wait for every
    /// child process before returning (including error/unwind cleanup). Dropping
    /// the awaiting request does not abort the blocking operation. Dropping a
    /// queued request releases its reservation without starting a write.
    /// This is not a remote executor, cancellation API, or cross-process lock.
    pub async fn run<T, F>(&self, tools: Vec<AppId>, operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        if tools.is_empty() {
            return Err("lifecycle action requires a tool".into());
        }
        let host = host_writes();
        let mut guards = Vec::with_capacity(tools.len());
        for (index, tool) in tools.iter().enumerate() {
            if tools[..index].contains(tool) {
                return Err("duplicate lifecycle action target".into());
            }
            let lock = host.tools.get(tool).ok_or("unsupported lifecycle action target")?;
            guards.push(lock.clone().try_lock_owned().map_err(|_| "TOOL_ACTION_IN_PROGRESS")?);
        }
        let execution = host.execution.clone().lock_owned().await;
        // No await between acquiring execution and handing both guards to the
        // non-abortable worker. The HTTP future never owns a running write lock.
        tokio::task::spawn_blocking(move || {
            let _tools = guards;
            let _execution = execution;
            operation()
        })
        .await
        .map_err(|_| "tool lifecycle task join error".to_owned())?
    }
}
