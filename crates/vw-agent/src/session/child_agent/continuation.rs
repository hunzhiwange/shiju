//! 同一存活运行时的串行续派入口。重启后不从磁盘推断权限。
#![allow(dead_code)] // 协作协议在后续任务接入。
use super::{ChildAgentCoordinator, ChildAgentTaskState};
use anyhow::{Result, bail};
use tokio::sync::mpsc;

impl ChildAgentCoordinator {
    pub(crate) fn attach_continuation(&self, child: &str) -> mpsc::Receiver<String> {
        let (sender, receiver) = mpsc::channel(128);
        self.lock_state().continuations.insert(child.into(), sender);
        receiver
    }

    pub(crate) fn followup_task(&self, parent: &str, child: &str, prompt: String) -> Result<()> {
        if prompt.trim().is_empty() || prompt.len() > 64 * 1024 {
            bail!("followup task must contain 1..65536 bytes");
        }
        let state = self.lock_state();
        let entry = state.tasks.get(child).ok_or_else(|| anyhow::anyhow!("unknown child agent"))?;
        if entry.snapshot.parent_session_id != parent {
            bail!("followup target does not belong to parent");
        }
        if entry.cancellation.is_cancelled()
            || entry.snapshot.state == ChildAgentTaskState::Cancelled
        {
            bail!("followup target was cancelled");
        }
        state
            .continuations
            .get(child)
            .ok_or_else(|| {
                anyhow::anyhow!("target runtime absent; effective permissions cannot be restored")
            })?
            .try_send(prompt)
            .map_err(|_| anyhow::anyhow!("target runtime is closed"))
    }

    pub(crate) fn turn_cancellation(
        &self,
        child: &str,
    ) -> Result<tokio_util::sync::CancellationToken> {
        self.lock_state()
            .tasks
            .get(child)
            .map(|entry| entry.turn_cancellation.clone())
            .ok_or_else(|| anyhow::anyhow!("unknown child agent"))
    }

    pub(crate) fn begin_followup(&self, parent: &str, child: &str, prompt: &str) -> Result<()> {
        let version = {
            let mut state = self.lock_state();
            let entry = super::task_for_parent_mut(&mut state, parent, child)?;
            if entry.cancellation.is_cancelled() {
                bail!("followup target was cancelled");
            }
            if !entry.snapshot.state.is_terminal() {
                bail!("previous child turn is still active");
            }
            entry.turn_cancellation.cancel();
            entry.turn_cancellation = entry.cancellation.child_token();
            entry.snapshot.turn = entry
                .snapshot
                .turn
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("child turn exhausted"))?;
            entry.snapshot.task = prompt.into();
            entry.snapshot.state = ChildAgentTaskState::Running;
            entry.snapshot.output = None;
            entry.snapshot.error = None;
            state.version += 1;
            state.version
        };
        self.publish_change(version);
        Ok(())
    }
}
