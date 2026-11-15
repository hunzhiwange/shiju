//! 协作消息仅为数据；身份来自运行时上下文，不接受模型指定发送者。
#![allow(dead_code)] // 后续协作工具与模型循环消费此运行时边界。
use super::*;
use std::collections::VecDeque;

pub use session_port::child_agent::{TeamMessageKind, TeamMessage, TeamActivity};

#[derive(Debug, Default)]
pub(super) struct Mailbox {
    pub(super) pending: VecDeque<TeamMessage>,
    pub(super) cursor: u64,
    pub(super) result_cursor: u64,
}

// 后续工具协议与模型安全边界连线使用这些运行时接口。
#[allow(dead_code)]
impl ChildAgentCoordinator {
    pub fn team_snapshots(&self, receiver: &str) -> Result<Vec<ChildAgentTaskSnapshot>> {
        let state = self.lock_state();
        let root = team_root(&state, receiver)?;
        let mut tasks = Vec::new();
        for entry in state.tasks.values() {
            if team_root(&state, &entry.snapshot.child_session_id)? == root {
                tasks.push(entry.snapshot.clone());
            }
        }
        tasks.sort_by(|a, b| a.child_session_id.cmp(&b.child_session_id));
        Ok(tasks)
    }

    pub fn enqueue_mail(
        &self,
        sender: &str,
        target: &str,
        turn: u64,
        kind: TeamMessageKind,
        payload: String,
    ) -> Result<TeamMessage> {
        if payload.len() > 64 * 1024 {
            bail!("team message exceeds 64 KiB");
        }
        let (message, version) = {
            let mut state = self.lock_state();
            if team_root(&state, sender)? != team_root(&state, target)? {
                bail!("team message crosses team boundary");
            }
            for session in [sender, target] {
                if state.tasks.get(session).is_some_and(|e| {
                    e.cancellation.is_cancelled()
                        || (e.snapshot.state.is_terminal()
                            && !state.continuations.contains_key(session))
                }) {
                    bail!("team session is closed");
                }
            }
            if state.mailboxes.get(target).is_some_and(|m| m.pending.len() >= 128) {
                bail!("team mailbox has 128 unread messages");
            }
            state.version = state
                .version
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("team sequence exhausted"))?;
            let sequence = state.version;
            let message = TeamMessage {
                id: format!("mail-{sequence}"),
                sender_session: sender.into(),
                recipient_session: target.into(),
                turn,
                kind,
                payload,
                sequence,
            };
            state.mailboxes.entry(target.into()).or_default().pending.push_back(message.clone());
            (message, sequence)
        };
        self.publish_change(version);
        Ok(message)
    }

    pub async fn wait_mail(
        &self,
        receiver: &str,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<TeamActivity> {
        let mut changed = self.inner.changed.subscribe();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let state = self.lock_state();
                team_root(&state, receiver)?;
                if cancellation.is_cancelled()
                    || state.tasks.get(receiver).is_some_and(|e| e.cancellation.is_cancelled())
                {
                    return Ok(TeamActivity::Cancelled);
                }
                if state.mailboxes.get(receiver).is_some_and(|m| !m.pending.is_empty()) {
                    return Ok(TeamActivity::Message);
                }
                let cursor = state.mailboxes.get(receiver).map_or(0, |m| m.result_cursor);
                if state.tasks.values().any(|e| {
                    e.snapshot.state.is_terminal()
                        && e.sequence > cursor
                        && e.snapshot.parent_session_id == receiver
                }) {
                    return Ok(TeamActivity::Result);
                }
            }
            tokio::select! {
                _ = cancellation.cancelled() => return Ok(TeamActivity::Cancelled),
                _ = tokio::time::sleep_until(deadline) => return Ok(TeamActivity::TimedOut),
                result = changed.changed() => { result?; }
            }
        }
    }

    pub fn take_mail(&self, receiver: &str) -> Result<Vec<TeamMessage>> {
        let mut state = self.lock_state();
        team_root(&state, receiver)?;
        let mailbox = state.mailboxes.entry(receiver.to_string()).or_default();
        let messages: Vec<_> = mailbox.pending.drain(..).collect();
        if let Some(last) = messages.last() {
            mailbox.cursor = last.sequence;
        }
        Ok(messages)
    }
}

fn team_root<'a>(state: &'a CoordinatorState, session: &'a str) -> Result<&'a str> {
    let mut current = session;
    let mut seen = HashSet::new();
    while let Some(entry) = state.tasks.get(current) {
        if !seen.insert(current) {
            bail!("team ancestor cycle");
        }
        current = &entry.snapshot.parent_session_id;
    }
    if current == session && !state.tasks.values().any(|e| e.snapshot.parent_session_id == session)
    {
        bail!("unknown team session");
    }
    Ok(current)
}

#[cfg(test)]
#[path = "mailbox_tests.rs"]
mod tests;
