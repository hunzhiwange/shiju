use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}};
use tokio::sync::{Mutex, mpsc, oneshot, watch};
use async_trait::async_trait;
use serde_json::json;
use crate::server::*;
use super::process::{Process, failure};

#[derive(Default)]
pub struct AionrsBackend {
    sessions: Mutex<HashMap<String, Arc<Mutex<Process>>>>,
    sequence: AtomicU64,
}
struct Cancel { sender: watch::Sender<bool>, cancelled: AtomicBool }
impl BackendCancelHandle for Cancel {
    fn cancel(&self) -> bool {
        if self.cancelled.swap(true, Ordering::SeqCst) { return false; }
        let _ = self.sender.send(true); true
    }
}
#[async_trait]
impl SessionBackend for AionrsBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        let process = Process::spawn(&cwd).await?;
        let id = format!("aionrs-{}", self.sequence.fetch_add(1, Ordering::Relaxed));
        self.sessions.lock().await.insert(id.clone(), Arc::new(Mutex::new(process)));
        Ok(BackendSession { id, cwd, history: vec![] })
    }
    async fn load_session(&self, _: &str, _: PathBuf) -> Result<BackendSession, BackendError> {
        Err(BackendError::InvalidRequest("aionrs session restore is not supported".into()))
    }
    async fn start_prompt(&self, id: &str, prompt: Vec<PromptContent>) -> Result<BackendPrompt, BackendError> {
        let text = prompt.into_iter().map(|p| match p {
            PromptContent::Text(t) => Ok(t),
            _ => Err(BackendError::InvalidRequest("aionrs accepts text only".into())),
        }).collect::<Result<Vec<_>, _>>()?.join("\n");
        let process = self.sessions.lock().await.get(id).cloned().ok_or_else(|| BackendError::NotFound(id.into()))?;
        let mut process = process.try_lock_owned().map_err(|_| BackendError::InvalidRequest("aionrs session is busy".into()))?;
        let msg = format!("turn-{}", self.sequence.fetch_add(1, Ordering::Relaxed));
        process.send(json!({"type":"message","msg_id":msg,"content":text})).await?;
        let (events, receiver) = mpsc::unbounded_channel();
        let (permissions, requests) = mpsc::unbounded_channel();
        let (sender, mut cancel) = watch::channel(false);
        tokio::spawn(async move {
            let result = async {
                let mut seen = HashSet::new();
                let mut approved = HashSet::new();
                let mut pending = HashSet::new();
                let (answers, mut responses) = mpsc::unbounded_channel::<(String, bool)>();
                loop {
                    tokio::select! {
                        _ = cancel.changed() => {
                            for call in &pending { process.send(json!({"type":"tool_deny","call_id":call,"reason":"Cancelled"})).await?; }
                            process.send(json!({"type":"stop"})).await?;
                            // Retire the process: no old turn can ever enter a later prompt.
                            process.kill().await;
                            let _ = events.send(BackendEvent::Terminal(BackendTermination::Cancelled));
                            return Ok::<(), BackendError>(());
                        }
                        answer = responses.recv() => {
                            let Some((call, allow)) = answer else { return Err(failure("approval channel")); };
                            if !pending.remove(&call) { return Err(failure("approval identity")); }
                            if allow { approved.insert(call.clone()); }
                            process.send(if allow { json!({"type":"tool_approve","call_id":call,"scope":"once"}) }
                                else { json!({"type":"tool_deny","call_id":call,"reason":"Host denied"}) }).await?;
                        }
                        frame = process.read() => {
                            let frame = frame?;
                            if frame.msg_id.as_deref() != Some(msg.as_str()) { return Err(failure("turn identity")); }
                            match frame.kind.as_str() {
                                "stream_start" => {},
                                "text_delta" => { let _ = events.send(BackendEvent::Text(frame.text.ok_or_else(|| failure("text"))?)); },
                                "thinking" => { let _ = events.send(BackendEvent::Status(frame.text.ok_or_else(|| failure("thinking"))?)); },
                                "tool_request" => {
                                    let call = frame.call_id.ok_or_else(|| failure("call"))?;
                                    if !seen.insert(call.clone()) { return Err(failure("duplicate call")); }
                                    let title = frame.tool.ok_or_else(|| failure("tool"))?.name;
                                    // Spawn may grant tools inside an unmediated nested engine.
                                    if title == "Spawn" {
                                        process.send(json!({"type":"tool_deny","call_id":call,"reason":"Nested agents are unsupported"})).await?;
                                        continue;
                                    }
                                    pending.insert(call.clone());
                                    let (response, answer) = oneshot::channel();
                                    let request = BackendPermissionRequest { request_id: call.clone(), call_id: call.clone(), title,
                                        raw_input: json!({}), options: vec![
                                            BackendPermissionOption { id: "allow".into(), name: "Allow once".into(), kind: BackendPermissionOptionKind::AllowOnce },
                                            BackendPermissionOption { id: "deny".into(), name: "Deny".into(), kind: BackendPermissionOptionKind::RejectOnce }], response };
                                    let _ = permissions.send(request);
                                    let answers = answers.clone();
                                    tokio::spawn(async move {
                                        let allow = tokio::time::timeout(std::time::Duration::from_secs(120), answer).await
                                            .ok().and_then(Result::ok).flatten().as_deref() == Some("allow");
                                        let _ = answers.send((call, allow));
                                    });
                                }
                                "tool_running" => {
                                    let call = frame.call_id.ok_or_else(|| failure("call"))?;
                                    if !approved.contains(&call) { return Err(failure("unapproved execution")); }
                                    let _ = events.send(BackendEvent::ToolStart { call_id: call, title: "aionrs tool".into(), raw_input: json!({}) });
                                }
                                "tool_result" | "tool_cancelled" => {
                                    let call = frame.call_id.ok_or_else(|| failure("call"))?;
                                    if !seen.contains(&call) { return Err(failure("unknown call")); }
                                    let status = if frame.status.as_deref() == Some("success") { BackendToolStatus::Completed } else { BackendToolStatus::Failed };
                                    let _ = events.send(BackendEvent::ToolComplete { call_id: call, status, content: None, raw_output: None });
                                }
                                "stream_end" if pending.is_empty() => {
                                    let _ = events.send(BackendEvent::Terminal(BackendTermination::EndTurn)); return Ok(());
                                }
                                "info" => {},
                                _ => return Err(failure("unexpected event")),
                            }
                        }
                    }
                }
            }.await;
            if result.is_err() { process.kill().await; let _ = events.send(BackendEvent::Error("aionrs session terminated after a protocol failure".into())); }
        });
        Ok(BackendPrompt::new(receiver, Arc::new(Cancel { sender, cancelled: AtomicBool::new(false) })).with_permissions(requests))
    }
}
