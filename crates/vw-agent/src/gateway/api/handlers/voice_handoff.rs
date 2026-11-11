//! Connection-owned work admission and result projection. No permission overrides.
use std::collections::HashMap;
use vw_api_types::voice::{HandoffStatus, VoiceCounter, VoiceErrorCode, VoiceId};

pub(crate) use crate::provider::voice::realtime_handoff::HandoffCall;
#[derive(Default)]
pub(crate) struct Admissions {
    calls: HashMap<VoiceId, HandoffCall>,
}
impl Admissions {
    pub(crate) fn admit(
        &mut self,
        call: &HandoffCall,
        generation: VoiceCounter,
    ) -> Result<bool, VoiceErrorCode> {
        if call.generation != generation {
            return Ok(false);
        }
        if let Some(existing) = self.calls.get(&call.call_id) {
            if existing != call {
                return Err(VoiceErrorCode::ProtocolError);
            }
            return Ok(false);
        }
        if self.calls.len() >= 256 {
            return Err(VoiceErrorCode::Backpressure);
        }
        self.calls.insert(call.call_id.clone(), call.clone());
        Ok(true)
    }
}
#[derive(Default)]
pub(crate) struct Outcome {
    denied: bool,
    failed: bool,
}
impl Outcome {
    pub(crate) fn observe(&mut self, payload: &serde_json::Value) {
        if payload["type"] == "chat.tool" {
            match payload["status"].as_str() {
                Some("denied") => self.denied = true,
                Some("failed" | "cancelled") => self.failed = true,
                _ => {}
            }
        }
    }
    pub(crate) fn finish(&self, text: String) -> (HandoffStatus, String) {
        if self.denied {
            return (HandoffStatus::Failed, "工作审批已拒绝，未获准的操作没有执行。".into());
        }
        if self.failed {
            return (HandoffStatus::Failed, "工作未能完成，请在聊天中查看结果。".into());
        }
        (HandoffStatus::Completed, bounded_text(text))
    }
}
pub(crate) fn bounded_text(mut text: String) -> String {
    // Keep the serialized handoff frame below 64 KiB even for control-heavy tool output.
    text.retain(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'));
    if text.len() > 16 * 1024 {
        let mut end = 16 * 1024 - "…".len();
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push('…');
    }
    text
}

#[derive(Clone)]
pub(crate) struct Binding {
    pub tenant: vw_api_types::id::TenantScope,
    pub chat_session: VoiceId,
    pub directory: String,
}
pub(crate) struct Update {
    pub call: HandoffCall,
    pub status: HandoffStatus,
    pub text: String,
}
/// One owner per authenticated socket; dropping it cancels the existing session collector.
pub(crate) struct Handoffs {
    binding: Binding,
    admissions: Admissions,
    active: Option<(HandoffCall, tokio::task::JoinHandle<()>)>,
    tx: tokio::sync::mpsc::Sender<Update>,
    pub updates: tokio::sync::mpsc::Receiver<Update>,
}
impl Handoffs {
    pub(crate) fn new(binding: Binding) -> Self {
        let (tx, updates) = tokio::sync::mpsc::channel(8);
        Self { binding, admissions: Admissions::default(), active: None, tx, updates }
    }
    pub(crate) fn submit(
        &mut self,
        call: HandoffCall,
        generation: VoiceCounter,
    ) -> Result<(), VoiceErrorCode> {
        if !self.admissions.admit(&call, generation)? {
            return Ok(());
        }
        if self.active.is_some() {
            return Err(VoiceErrorCode::Conflict);
        }
        let binding = self.binding.clone();
        let worker_call = call.clone();
        let tx = self.tx.clone();
        let task = tokio::spawn(async move {
            crate::session::ui_store::with_tenant_scope(binding.tenant.clone(), async move {
                run(binding, worker_call, tx).await;
            })
            .await;
        });
        self.active = Some((call, task));
        Ok(())
    }
    pub(crate) fn generation_changed(&mut self) -> Option<Update> {
        let (call, task) = self.active.take()?;
        task.abort();
        Some(Update {
            call,
            status: HandoffStatus::Cancelled,
            text: "已请求取消语音工作；已发生的操作不会自动回滚。".into(),
        })
    }
    pub(crate) fn finish(&mut self, call: &HandoffCall) {
        if self.active.as_ref().is_some_and(|(active, _)| active == call) {
            self.active.take();
        }
    }
}
impl Drop for Handoffs {
    fn drop(&mut self) {
        if let Some((_, task)) = self.active.take() {
            task.abort();
        }
    }
}
async fn run(binding: Binding, call: HandoffCall, tx: tokio::sync::mpsc::Sender<Update>) {
    use super::session::stream::collect_gateway_chat_stream_with_observer;
    use std::sync::{Arc, Mutex};
    use vw_api_types::chat::GatewayChatStreamRequest;
    let send = |status, text: &str| {
        tx.try_send(Update { call: call.clone(), status, text: text.to_owned() }).is_ok()
    };
    if !send(HandoffStatus::Accepted, "已交给当前 ShiJu 聊天会话。") {
        return;
    }
    // The binding was fixed by authenticated upgrade, never supplied by service arguments.
    let scope = binding.tenant.clone();
    let session = binding.chat_session.as_ref().to_owned();
    let owned = crate::global_database::run_isolated(move || {
        let Some(info) =
            crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&scope, &session)?
        else {
            return Ok::<_, anyhow::Error>(None);
        };
        let snapshot = crate::session::ui_store::load_session_scoped_for_tenant_fallible(
            &scope,
            &session,
            Some(&info.project_id),
        )?;
        Ok(Some((info, snapshot)))
    })
    .await;
    let snapshot = match owned {
        Ok(Ok(Some((info, snapshot)))) if info.directory == binding.directory => snapshot,
        _ => {
            let _ = send(HandoffStatus::Failed, "聊天会话已不可用，工作没有启动。");
            return;
        }
    };
    let directory = binding.directory.clone();
    let manager = crate::session::ui_store::with_tenant_scope(
        binding.tenant.clone(),
        crate::gateway::instance::with_instance(directory, || {
            Box::pin(async {
                Ok::<_, crate::gateway::ApiError>(
                    crate::gateway::approval_state::approval_manager_for_current_instance().await,
                )
            })
        }),
    )
    .await;
    let Ok(manager) = manager else {
        let _ = send(HandoffStatus::Failed, "工作审批服务不可用。");
        return;
    };
    // No full_access, tool allowlist or background permission overrides. Session binding
    // and the standard producer select the existing agent/model/approval policy.
    let mut messages: Vec<_> = snapshot
        .into_iter()
        .flat_map(|snapshot| snapshot.messages)
        .map(|message| {
            let role = match message.role {
                crate::session::ui_types::ChatRole::User => "user",
                crate::session::ui_types::ChatRole::Assistant => "assistant",
                crate::session::ui_types::ChatRole::System => "system",
                crate::session::ui_types::ChatRole::Developer => "developer",
                crate::session::ui_types::ChatRole::Tool => "tool",
            };
            serde_json::json!({"role":role,"content":message.content})
        })
        .collect();
    messages.push(serde_json::json!({"role":"user","content":call.instruction}));
    let request = GatewayChatStreamRequest {
        session_id: Some(binding.chat_session.as_ref().to_owned().into()),
        messages,
        ..Default::default()
    };
    let outcome = Arc::new(Mutex::new(Outcome::default()));
    let observed = outcome.clone();
    let work =
        collect_gateway_chat_stream_with_observer(binding.directory, request, move |payload| {
            observed.lock().unwrap_or_else(|e| e.into_inner()).observe(payload);
        });
    tokio::pin!(work);
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut waiting = false;
    if !send(HandoffStatus::Running, "ShiJu 正在处理工作。") {
        return;
    }
    let result = loop {
        tokio::select! { biased;
            result = &mut work => break result,
            _ = interval.tick() => {
                let pending = !manager.list_non_cli_pending_requests(None, None, Some(binding.chat_session.as_ref())).is_empty();
                if pending != waiting {
                    waiting = pending;
                    let (status, text) = if waiting { (HandoffStatus::AwaitingApproval, "工作等待审批，请在原桌面审批界面确认或拒绝。") } else { (HandoffStatus::Running, "ShiJu 正在处理工作。") };
                    if !send(status, text) { return; }
                }
            }
        }
    };
    let (status, text) = match result {
        Ok(result) => outcome.lock().unwrap_or_else(|e| e.into_inner()).finish(result.output),
        Err(_) => (HandoffStatus::Failed, "工作失败，请在聊天中查看详情。".into()),
    };
    let _ = send(status, &text);
}

#[cfg(test)]
#[path = "voice_handoff_tests.rs"]
mod tests;
