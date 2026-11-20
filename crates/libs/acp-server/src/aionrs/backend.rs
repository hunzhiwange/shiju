use super::process::{Process, failure};
use crate::server::*;
use agent_client_protocol::{ModelInfo, SessionModelState};
use async_trait::async_trait;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::sync::{Mutex, mpsc, oneshot, watch};

pub(super) fn workspace_tool(workspace: &Path, name: &str, args: &serde_json::Value) -> bool {
    let value = match name {
        "Read" | "Write" | "Edit" => args.get("file_path").and_then(serde_json::Value::as_str),
        "Glob" => {
            let Some(pattern) = args.get("pattern").and_then(serde_json::Value::as_str) else {
                return false;
            };
            if Path::new(pattern).is_absolute()
                || Path::new(pattern).components().any(|part| part == Component::ParentDir)
            {
                return false;
            }
            args.get("path").map_or(Some("."), serde_json::Value::as_str)
        }
        _ => return false,
    };
    let Some(value) = value.filter(|value| !value.is_empty()) else { return false };
    let target = workspace.join(value);
    if !target.starts_with(workspace)
        || target.components().any(|part| part == Component::ParentDir)
    {
        return false;
    }
    // New files can have missing parent components. A dangling symlink is a present
    // entry and must still resolve successfully before the call is approved.
    for existing in target.ancestors() {
        match existing.symlink_metadata() {
            Ok(_) => return existing.canonicalize().is_ok_and(|path| path.starts_with(workspace)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    false
}

fn managed_skill_tool(process: &Process, name: &str, args: &serde_json::Value) -> bool {
    if !process.managed_skills {
        return false;
    }
    if name == "Skill" {
        return args.as_object().is_some_and(|args| {
            args.len() == 1
                && args
                    .get("skill")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|skill| !skill.is_empty() && skill.len() <= 256)
        });
    }
    if name != "Read" {
        return false;
    }
    let Some(root) = &process.skill_root else {
        return false;
    };
    let Some(target) = args
        .get("file_path")
        .and_then(serde_json::Value::as_str)
        .map(Path::new)
    else {
        return false;
    };
    if !target.starts_with(root) || target.components().any(|part| part == Component::ParentDir) {
        return false;
    }
    target
        .canonicalize()
        .is_ok_and(|file| file.starts_with(&process.workspace) && file.is_file())
}

fn verify_projected_skills(
    root: &Path,
    workspace: &Path,
    selection: &[acp_types::SessionSkillMount],
) -> Result<Vec<String>, BackendError> {
    let invalid = || BackendError::InvalidRequest(
        "aionrs skill mounts do not match the workspace-bound launch selection".into(),
    );
    let root = root.join(".aionrs/skills");
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && selection.is_empty() => {
            return Ok(Vec::new());
        }
        Err(_) => return Err(invalid()),
    };
    let entries = entries.collect::<std::io::Result<Vec<_>>>().map_err(|_| invalid())?;
    if entries.len() != selection.len() {
        return Err(invalid());
    }
    selection
        .iter()
        .map(|mount| {
            let projected = root.join(&mount.name);
            let expected = std::fs::read_link(&projected).map_err(|_| invalid())?;
            // The wire selection only verifies the host projection. Neither its
            // path nor its name can add an entry to the engine's scanner.
            if expected == workspace || !expected.starts_with(workspace)
                || projected.canonicalize().ok().as_ref() != Some(&expected)
                || Path::new(&mount.path).canonicalize().ok().as_ref() != Some(&expected)
                || !expected.is_dir()
            {
                return Err(invalid());
            }
            Ok(mount.name.clone())
        })
        .collect()
}

#[derive(Default)]
pub struct AionrsBackend {
    sessions: Mutex<HashMap<String, Arc<Mutex<Process>>>>,
    sequence: AtomicU64,
}
struct Cancel {
    sender: watch::Sender<bool>,
    cancelled: AtomicBool,
}
impl BackendCancelHandle for Cancel {
    fn cancel(&self) -> bool {
        if self.cancelled.swap(true, Ordering::SeqCst) {
            return false;
        }
        let _ = self.sender.send(true);
        true
    }
}
impl AionrsBackend {
    async fn insert_session(&self, cwd: PathBuf, process: Process) -> BackendSession {
        let id = format!("aionrs-{}", self.sequence.fetch_add(1, Ordering::Relaxed));
        self.sessions.lock().await.insert(id.clone(), Arc::new(Mutex::new(process)));
        BackendSession { id, cwd, history: vec![] }
    }
}

#[async_trait]
impl SessionBackend for AionrsBackend {
    fn supports_session_load(&self) -> bool {
        false
    }

    fn supports_mcp_servers(&self) -> bool {
        false
    }

    fn supports_skill_mounts(&self) -> bool {
        std::env::var_os("SHIJU_AIONRS_SANDBOX").is_some()
            && std::env::var_os("SHIJU_AIONRS_SKILL_ROOT").is_some()
    }

    async fn create_session_with_mounts(
        &self,
        cwd: PathBuf,
        request: BackendMountRequest,
    ) -> Result<BackendMountedSession, BackendError> {
        let mut mounts = request.legacy_response()?;
        if let Some(root) = std::env::var_os("SHIJU_AIONRS_SKILL_ROOT")
            .filter(|_| self.supports_skill_mounts())
        {
            let skills = request.skills.as_ref().map_or(&[][..], |selection| &selection.skills);
            mounts.skills = verify_projected_skills(Path::new(&root), &cwd, skills)?;
        } else {
            request.reject_unsupported_skills()?;
        }
        let session = self.create_session_with_mcp(cwd, request.mcp_servers).await?;
        Ok(BackendMountedSession { session, mounts })
    }

    async fn session_models(&self, id: &str) -> Result<Option<SessionModelState>, BackendError> {
        if !self.sessions.lock().await.contains_key(id) {
            return Err(BackendError::NotFound(id.into()));
        }
        Ok(std::env::var_os("SHIJU_AIONRS_SANDBOX").and_then(|_| std::env::var("MODEL").ok()).map(
            |model| {
                SessionModelState::new(model.clone(), vec![ModelInfo::new(model.clone(), model)])
            },
        ))
    }

    async fn set_session_model(
        &self,
        id: &str,
        model: &str,
    ) -> Result<SessionModelState, BackendError> {
        self.session_models(id)
            .await?
            .filter(|state| state.current_model_id.0.as_ref() == model)
            .ok_or_else(|| {
                BackendError::InvalidRequest("aionrs session is bound to its launch model".into())
            })
    }

    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        let process = Process::spawn(&cwd).await?;
        Ok(self.insert_session(cwd, process).await)
    }

    async fn create_session_with_mcp(
        &self,
        cwd: PathBuf,
        servers: Vec<agent_client_protocol::McpServer>,
    ) -> Result<BackendSession, BackendError> {
        if servers.is_empty() {
            return self.create_session(cwd).await;
        }
        #[cfg(unix)]
        {
            if std::env::var_os("SHIJU_AIONRS_SANDBOX").is_none() {
                return Err(BackendError::InvalidRequest("aionrs MCP requires a sandboxed engine".into()));
            }
            let mut process = Process::spawn(&cwd).await?;
            if let Err(error) = process.attach_mcp(servers).await {
                process.kill().await;
                return Err(error);
            }
            Ok(self.insert_session(cwd, process).await)
        }
        #[cfg(not(unix))]
        Err(BackendError::InvalidRequest("aionrs MCP requires the managed model relay".into()))
    }

    async fn load_session(&self, _: &str, _: PathBuf) -> Result<BackendSession, BackendError> {
        Err(BackendError::InvalidRequest("aionrs session restore is not supported".into()))
    }
    async fn start_prompt(
        &self,
        id: &str,
        prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        let text = prompt
            .into_iter()
            .map(|p| match p {
                PromptContent::Text(t) => Ok(t),
                _ => Err(BackendError::InvalidRequest("aionrs accepts text only".into())),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        let process = self
            .sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| BackendError::NotFound(id.into()))?;
        let mut process = process
            .try_lock_owned()
            .map_err(|_| BackendError::InvalidRequest("aionrs session is busy".into()))?;
        let msg = format!("turn-{}", self.sequence.fetch_add(1, Ordering::Relaxed));
        process.send(json!({"type":"message","msg_id":msg,"content":text})).await?;
        let (events, receiver) = mpsc::unbounded_channel();
        let (permissions, requests) = mpsc::unbounded_channel();
        let (sender, mut cancel) = watch::channel(false);
        tokio::spawn(async move {
            let result = async {
                let mut seen = HashSet::new();
                let mut approved = HashSet::new();
                let mut mcp_titles = HashMap::new();
                let mut pending = HashMap::<String, (String, serde_json::Value, bool)>::new();
                let (answers, mut responses) = mpsc::unbounded_channel::<(String, bool)>();
                loop {
                    tokio::select! {
                        _ = cancel.changed() => {
                            for call in pending.keys() { process.send(json!({"type":"tool_deny","call_id":call,"reason":"Cancelled"})).await?; }
                            process.send(json!({"type":"stop"})).await?;
                            // Retire the process: no old turn can ever enter a later prompt.
                            process.kill().await;
                            let _ = events.send(BackendEvent::Terminal(BackendTermination::Cancelled));
                            return Ok::<(), BackendError>(());
                        }
                        answer = responses.recv() => {
                            let Some((call, allow)) = answer else { return Err(failure("approval channel")); };
                            let Some((name, arguments, is_mcp)) = pending.remove(&call) else {
                                return Err(failure("approval identity"));
                            };
                            if allow && is_mcp {
                                #[cfg(unix)]
                                process.mcp.as_ref().ok_or_else(|| failure("missing MCP broker"))?
                                    .approve(name, arguments)?;
                            }
                            if allow { approved.insert(call.clone()); }
                            process.send(if allow { json!({"type":"tool_approve","call_id":call,"scope":"once"}) }
                                else { json!({"type":"tool_deny","call_id":call,"reason":"Host denied"}) }).await?;
                        }
                        frame = process.read() => {
                            let frame = frame?;
                            // The upstream output sink emits connection-level tool notices with
                            // an empty turn ID. Discard them before validating data-bearing events.
                            if frame.kind == "info" && frame.msg_id.as_deref() == Some("") {
                                continue;
                            }
                            if frame.msg_id.as_deref() != Some(msg.as_str()) { return Err(failure("turn identity")); }
                            match frame.kind.as_str() {
                                "stream_start" => {},
                                "text_delta" => { let _ = events.send(BackendEvent::Text(frame.text.ok_or_else(|| failure("text"))?)); },
                                "thinking" => { let _ = events.send(BackendEvent::Status(frame.text.ok_or_else(|| failure("thinking"))?)); },
                                "tool_request" => {
                                    let call = frame.call_id.ok_or_else(|| failure("call"))?;
                                    if !seen.insert(call.clone()) { return Err(failure("duplicate call")); }
                                    let tool = frame.tool.ok_or_else(|| failure("tool"))?;
                                    let is_mcp = tool.category.as_deref() == Some("mcp");
                                    #[cfg(unix)]
                                    let mounted = is_mcp && process.mcp.as_ref().is_some_and(|broker| broker.contains(&tool.name));
                                    #[cfg(not(unix))]
                                    let mounted = false;
                                    if !mounted && (is_mcp
                                        || (!workspace_tool(&process.workspace, &tool.name, &tool.args)
                                            && !managed_skill_tool(&process, &tool.name, &tool.args)))
                                    {
                                        process.send(json!({"type":"tool_deny","call_id":call,"reason":"Only workspace file tools are supported"})).await?;
                                        continue;
                                    }
                                    let title = tool.name;
                                    if mounted { mcp_titles.insert(call.clone(), title.clone()); }
                                    let raw_input = if title == "Skill" { tool.args.clone() } else { json!({}) };
                                    pending.insert(call.clone(), (title.clone(), tool.args, mounted));
                                    let (response, answer) = oneshot::channel();
                                    let request = BackendPermissionRequest { request_id: call.clone(), call_id: call.clone(), title,
                                        raw_input, options: vec![
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
                                    let title = mcp_titles.get(&call).cloned().unwrap_or_else(|| "aionrs tool".into());
                                    let _ = events.send(BackendEvent::ToolStart { call_id: call, title, raw_input: json!({}) });
                                }
                                "tool_result" | "tool_cancelled" => {
                                    let call = frame.call_id.ok_or_else(|| failure("call"))?;
                                    if !seen.contains(&call) { return Err(failure("unknown call")); }
                                    let status = if frame.status.as_deref() == Some("success") { BackendToolStatus::Completed } else { BackendToolStatus::Failed };
                                    let content = mcp_titles.remove(&call).and(frame.output);
                                    let _ = events.send(BackendEvent::ToolComplete { call_id: call, status, content, raw_output: None });
                                }
                                "stream_end" if pending.is_empty() => {
                                    #[cfg(unix)]
                                    if let Some(broker) = process.mcp.as_ref() { broker.clear_approvals(); }
                                    let _ = events.send(BackendEvent::Terminal(BackendTermination::EndTurn)); return Ok(());
                                }
                                "info" => {},
                                _ => return Err(failure("unexpected event")),
                            }
                        }
                    }
                }
            }.await;
            if result.is_err() {
                process.kill().await;
                let _ = events.send(BackendEvent::Error(
                    "aionrs session terminated after a protocol failure".into(),
                ));
            }
        });
        Ok(BackendPrompt::new(
            receiver,
            Arc::new(Cancel { sender, cancelled: AtomicBool::new(false) }),
        )
        .with_permissions(requests))
    }
}
