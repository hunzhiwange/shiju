//! 队列所有者进程与 IPC 通道的健康检查。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::queue_ipc_transport::connect_to_queue_owner;
use crate::queue_lease_store::{read_default_queue_owner_record, read_default_queue_owner_status};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOwnerHealth {
    pub session_id: String,
    pub has_lease: bool,
    pub healthy: bool,
    pub socket_reachable: bool,
    pub pid_alive: bool,
    pub pid: Option<u32>,
    pub socket_path: Option<PathBuf>,
    pub owner_generation: Option<u64>,
    pub queue_depth: Option<u64>,
}

pub async fn probe_queue_owner_health(session_id: &str) -> QueueOwnerHealth {
    let Some(owner_record) = read_default_queue_owner_record(session_id).await else {
        return QueueOwnerHealth {
            session_id: session_id.to_string(),
            has_lease: false,
            healthy: false,
            socket_reachable: false,
            pid_alive: false,
            pid: None,
            socket_path: None,
            owner_generation: None,
            queue_depth: None,
        };
    };

    let Some(owner) = read_default_queue_owner_status(session_id).await.ok().flatten() else {
        return QueueOwnerHealth {
            session_id: session_id.to_string(),
            has_lease: false,
            healthy: false,
            socket_reachable: false,
            pid_alive: false,
            pid: None,
            socket_path: None,
            owner_generation: None,
            queue_depth: None,
        };
    };

    let pid_alive = owner.alive;
    let socket_reachable =
        matches!(connect_to_queue_owner(&owner_record, Some(2)).await, Ok(Some(_)));

    QueueOwnerHealth {
        session_id: session_id.to_string(),
        has_lease: true,
        healthy: socket_reachable,
        socket_reachable,
        pid_alive,
        pid: Some(owner.pid),
        socket_path: Some(owner.socket_path),
        owner_generation: Some(owner.owner_generation),
        queue_depth: Some(owner.queue_depth),
    }
}

#[cfg(test)]
#[path = "queue_ipc_health_tests.rs"]
mod queue_ipc_health_tests;
