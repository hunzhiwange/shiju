//! 定义调度器与工作节点控制器之间的生命周期请求、状态快照及适配器契约。

use anyhow::{Result, bail};
use async_trait::async_trait;

use crate::agent_scheduler::domain::{TeamId, WorkerName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerLifecycleState {
    Running,
    Sleeping,
    Stopped,
}

impl WorkerLifecycleState {
    pub fn as_hiclaw_spec_state(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Sleeping => "Sleeping",
            Self::Stopped => "Stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureWorkerReadyRequest {
    pub worker_name: WorkerName,
    pub runtime: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
}

impl From<EnsureWorkerReadyRequest> for WorkerLifecycleRequest {
    fn from(value: EnsureWorkerReadyRequest) -> Self {
        Self {
            worker_name: value.worker_name,
            desired_state: WorkerLifecycleState::Running,
            runtime: value.runtime,
            team_id: value.team_id,
            worker_role: value.worker_role,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerLifecycleRequest {
    pub worker_name: WorkerName,
    pub desired_state: WorkerLifecycleState,
    pub runtime: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerStatusRequest {
    pub worker_name: WorkerName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerControllerSnapshot {
    pub worker_name: WorkerName,
    pub desired_state: WorkerLifecycleState,
    pub runtime: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
    pub room_id: Option<String>,
    pub container_state: Option<String>,
}

#[async_trait]
pub trait ControllerAdapter: Send + Sync {
    async fn ensure_worker_ready(
        &self,
        request: EnsureWorkerReadyRequest,
    ) -> Result<WorkerControllerSnapshot>;

    async fn request_worker_sleep(
        &self,
        request: WorkerLifecycleRequest,
    ) -> Result<WorkerControllerSnapshot>;

    async fn request_worker_stop(
        &self,
        request: WorkerLifecycleRequest,
    ) -> Result<WorkerControllerSnapshot>;

    async fn worker_status(
        &self,
        request: WorkerStatusRequest,
    ) -> Result<Option<WorkerControllerSnapshot>>;
}

fn validate_lifecycle_request(
    request: &WorkerLifecycleRequest,
    expected: WorkerLifecycleState,
    operation: &str,
) -> Result<()> {
    if request.desired_state != expected {
        bail!(
            "{operation} requires desired state {}, got {}",
            expected.as_hiclaw_spec_state(),
            request.desired_state.as_hiclaw_spec_state()
        );
    }
    Ok(())
}

#[cfg(test)]
mod fake {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, Clone, Default)]
    pub struct FakeControllerAdapter {
        snapshots: Arc<Mutex<HashMap<String, WorkerControllerSnapshot>>>,
        lifecycle_requests: Arc<Mutex<Vec<WorkerLifecycleRequest>>>,
        status_requests: Arc<Mutex<Vec<WorkerStatusRequest>>>,
    }

    impl FakeControllerAdapter {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set_snapshot(&self, snapshot: WorkerControllerSnapshot) {
            self.snapshots
                .lock()
                .expect("fake controller snapshots lock poisoned")
                .insert(snapshot.worker_name.as_str().to_string(), snapshot);
        }

        pub fn recorded_lifecycle_requests(&self) -> Vec<WorkerLifecycleRequest> {
            self.lifecycle_requests
                .lock()
                .expect("fake controller lifecycle requests lock poisoned")
                .clone()
        }

        pub fn recorded_status_requests(&self) -> Vec<WorkerStatusRequest> {
            self.status_requests
                .lock()
                .expect("fake controller status requests lock poisoned")
                .clone()
        }

        fn record_lifecycle(&self, request: WorkerLifecycleRequest) -> WorkerControllerSnapshot {
            self.lifecycle_requests
                .lock()
                .expect("fake controller lifecycle requests lock poisoned")
                .push(request.clone());
            self.snapshot_for_lifecycle(&request)
        }

        fn snapshot_for_lifecycle(
            &self,
            request: &WorkerLifecycleRequest,
        ) -> WorkerControllerSnapshot {
            let mut snapshot = self
                .snapshots
                .lock()
                .expect("fake controller snapshots lock poisoned")
                .get(request.worker_name.as_str())
                .cloned()
                .unwrap_or_else(|| WorkerControllerSnapshot {
                    worker_name: request.worker_name.clone(),
                    desired_state: request.desired_state,
                    runtime: request.runtime.clone(),
                    team_id: request.team_id.clone(),
                    worker_role: request.worker_role.clone(),
                    room_id: None,
                    container_state: None,
                });

            snapshot.desired_state = request.desired_state;
            if snapshot.runtime.is_none() {
                snapshot.runtime = request.runtime.clone();
            }
            if snapshot.team_id.is_none() {
                snapshot.team_id = request.team_id.clone();
            }
            if snapshot.worker_role.is_none() {
                snapshot.worker_role = request.worker_role.clone();
            }
            snapshot
        }
    }

    #[async_trait]
    impl ControllerAdapter for FakeControllerAdapter {
        async fn ensure_worker_ready(
            &self,
            request: EnsureWorkerReadyRequest,
        ) -> Result<WorkerControllerSnapshot> {
            Ok(self.record_lifecycle(request.into()))
        }

        async fn request_worker_sleep(
            &self,
            request: WorkerLifecycleRequest,
        ) -> Result<WorkerControllerSnapshot> {
            validate_lifecycle_request(
                &request,
                WorkerLifecycleState::Sleeping,
                "request worker sleep",
            )?;
            Ok(self.record_lifecycle(request))
        }

        async fn request_worker_stop(
            &self,
            request: WorkerLifecycleRequest,
        ) -> Result<WorkerControllerSnapshot> {
            validate_lifecycle_request(
                &request,
                WorkerLifecycleState::Stopped,
                "request worker stop",
            )?;
            Ok(self.record_lifecycle(request))
        }

        async fn worker_status(
            &self,
            request: WorkerStatusRequest,
        ) -> Result<Option<WorkerControllerSnapshot>> {
            self.status_requests
                .lock()
                .expect("fake controller status requests lock poisoned")
                .push(request.clone());
            Ok(self
                .snapshots
                .lock()
                .expect("fake controller snapshots lock poisoned")
                .get(request.worker_name.as_str())
                .cloned())
        }
    }
}

#[cfg(test)]
pub use fake::FakeControllerAdapter;
