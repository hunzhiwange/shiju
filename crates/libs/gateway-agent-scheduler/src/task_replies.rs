use super::*;
use std::collections::VecDeque;
use task_scheduler::runner::gateway_request::gateway_local_agent_stream_request;
use task_scheduler::runner::local::{
    ExecutionShutdownUnconfirmed, LocalAgentExecutionRequest, LocalAgentWorkspace,
    execution_metadata_string,
};
use tokio_util::sync::CancellationToken;
use vw_api_agent_scheduler::{
    AgentSchedulerScope, AgentSchedulerTaskAgentTurnDto as Turn,
    AgentSchedulerTaskAgentTurnStatus as Status, AgentSchedulerTaskDetailChange as Change,
    AgentSchedulerTaskDetailDto as Detail, AgentSchedulerTaskReplyMode as Mode,
    AgentSchedulerTaskReplySession as Session,
};

type ReplyKey = (AgentSchedulerScope, TaskId, String);

#[derive(Clone, Default)]
pub(super) struct ReplyQueues(Arc<Mutex<HashMap<ReplyKey, ReplyQueue>>>);

#[derive(Default)]
struct ReplyQueue {
    active: Option<(String, CancellationToken)>,
    waiting: VecDeque<String>,
    shutdown_unconfirmed: bool,
}

impl ReplyQueues {
    pub(super) fn project(
        &self,
        context: &AgentSchedulerContext,
        id: &TaskId,
        detail: &mut Detail,
    ) {
        let queues = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let scope = context.scope().project_scope();
        let author = scope.user_id().as_ref();
        for comment in &mut detail.collaboration.comments {
            let Some(turn) = comment.agent_turn.as_mut() else { continue };
            if !matches!(turn.status, Status::Queued | Status::Running) {
                continue;
            }
            let live = queues.iter().any(|((owner, task_id, agent), queue)| {
                let owner = owner.project_scope();
                task_id == id
                    && agent == &turn.agent
                    && owner.platform_id() == scope.platform_id()
                    && owner.company_id() == scope.company_id()
                    && owner.project_id() == scope.project_id()
                    && owner.user_id().as_ref() == comment.author
                    && (queue.active.as_ref().is_some_and(|(id, _)| id == &comment.id)
                        || queue.waiting.contains(&comment.id))
            });
            if !live {
                turn.status = Status::Failed;
            }
        }
        if let Some(agent) = detail.reply_agent.as_ref() {
            let key = (context.scope().clone(), id.clone(), agent.clone());
            detail.reply_session =
                if queues.get(&key).is_some_and(|queue| queue.shutdown_unconfirmed) {
                    Session::Unavailable
                } else if let Some(queue) = queues.get(&key) {
                    detail.reply_active_turn_id = queue.active.as_ref().map(|(id, _)| id.clone());
                    Session::Busy
                } else if detail.collaboration.comments.iter().any(|comment| {
                    comment.author == author
                        && comment.agent_turn.as_ref().is_some_and(|turn| {
                            turn.agent == *agent
                                && matches!(turn.status, Status::Completed | Status::Stopped)
                        })
                }) {
                    Session::Idle
                } else {
                    Session::Missing
                };
        }
    }
}

impl<H: GatewayExecutorHost> StoreBackedAgentSchedulerGateway<H> {
    // The caller holds the project lock across validation, persistence and queue admission.
    pub(super) fn save_detail_change(
        &self,
        context: AgentSchedulerContext,
        task_id: TaskId,
        change: Change,
    ) -> Result<Detail, ApiError> {
        let author = context.scope().project_scope().user_id().as_ref();
        let service = AgentTaskBoardService::new(&self.store);
        if let Change::StopReply { id } = &change {
            let mut detail = service
                .task_detail(&context, &task_id)
                .map_err(map_scheduler_error)?
                .ok_or_else(|| ApiError::not_found("task not found"))?;
            let turn = detail
                .collaboration
                .comments
                .iter()
                .find(|comment| comment.id == *id && comment.author == author)
                .and_then(|comment| comment.agent_turn.as_ref())
                .ok_or_else(|| ApiError::not_found("ACP reply not found"))?;
            let key = (context.scope().clone(), task_id.clone(), turn.agent.clone());
            let queues = self.reply_queues.0.lock().unwrap_or_else(|error| error.into_inner());
            if let Some((_, cancel)) = queues
                .get(&key)
                .and_then(|queue| queue.active.as_ref())
                .filter(|(active, _)| active == id)
            {
                cancel.cancel();
            }
            drop(queues);
            self.reply_queues.project(&context, &task_id, &mut detail);
            return Ok(detail);
        }
        let mut turn = None;
        let mut dispatch = None;
        if let Change::Comment { id, delivery: Some(delivery), .. } = &change {
            let detail = service
                .task_detail(&context, &task_id)
                .map_err(map_scheduler_error)?
                .ok_or_else(|| ApiError::not_found("task not found"))?;
            if !detail.collaboration.comments.iter().any(|comment| comment.id == *id) {
                let agent = detail
                    .reply_agent
                    .filter(|agent| agent == &delivery.agent)
                    .ok_or_else(|| ApiError::bad_request("the task's ACP agent has changed"))?;
                let key = (context.scope().clone(), task_id.clone(), agent.clone());
                let queues = self.reply_queues.0.lock().unwrap_or_else(|error| error.into_inner());
                let queue = queues.get(&key);
                if delivery.mode == Mode::Restart
                    && !queue
                        .and_then(|queue| queue.active.as_ref())
                        .is_some_and(|(id, _)| delivery.active_turn_id.as_ref() == Some(id))
                {
                    return Err(ApiError::bad_request("the active ACP reply has changed"));
                }
                if let Some(queue) = queue {
                    if queue.shutdown_unconfirmed {
                        return Err(ApiError::bad_request(
                            "ACP session shutdown could not be confirmed",
                        ));
                    }
                    if delivery.mode == Mode::Start {
                        return Err(ApiError::bad_request("an ACP reply is already running"));
                    }
                    if queue.waiting.len() >= 16 {
                        return Err(ApiError::bad_request("the ACP reply queue is full"));
                    }
                }
                let session_id = detail
                    .collaboration
                    .comments
                    .iter()
                    .rev()
                    .filter(|comment| comment.author == author)
                    .filter_map(|comment| comment.agent_turn.as_ref())
                    .find(|turn| turn.agent == agent)
                    .map_or_else(
                        || format!("task-reply-{}", uuid::Uuid::new_v4().simple()),
                        |turn| turn.session_id.clone(),
                    );
                turn = Some(Turn {
                    agent,
                    session_id,
                    status: Status::Queued,
                    response: String::new(),
                });
                dispatch = Some((key, id.clone(), delivery.mode));
            }
        }
        let (mut detail, inserted) = service
            .change_task_detail(&context, &task_id, author, change, turn, current_timestamp_ms())
            .map_err(map_task_board_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        if let Some((key, id, mode)) = dispatch.filter(|_| inserted) {
            let mut queues = self.reply_queues.0.lock().unwrap_or_else(|error| error.into_inner());
            let start_worker = !queues.contains_key(&key);
            let queue = queues.entry(key.clone()).or_default();
            if mode == Mode::Restart {
                if let Some((_, cancel)) = &queue.active {
                    cancel.cancel();
                }
                queue.waiting.push_front(id);
            } else {
                queue.waiting.push_back(id);
            }
            if start_worker {
                let gateway = self.clone();
                let context = context.clone();
                tokio::spawn(async move { gateway.run_task_replies(context, key).await });
            }
        }
        self.reply_queues.project(&context, &task_id, &mut detail);
        Ok(detail)
    }

    async fn run_task_replies(&self, context: AgentSchedulerContext, key: ReplyKey) {
        let task_id = &key.1;
        let lock = self.project_dispatch_lock(&context);
        let service = AgentTaskBoardService::new(&self.store);
        loop {
            let (comment_id, cancel) = {
                let _guard = lock.lock().await;
                let mut queues =
                    self.reply_queues.0.lock().unwrap_or_else(|error| error.into_inner());
                let Some(queue) = queues.get_mut(&key) else { return };
                let Some(id) = queue.waiting.pop_front() else {
                    queues.remove(&key);
                    return;
                };
                let cancel = CancellationToken::new();
                queue.active = Some((id.clone(), cancel.clone()));
                (id, cancel)
            };
            let result = match service.update_reply_turn(
                &context,
                task_id,
                &comment_id,
                Status::Running,
                String::new(),
            ) {
                Ok(()) => self.collect_task_reply(&context, task_id, &comment_id, &cancel).await,
                Err(error) => Err(error),
            };
            let unconfirmed = result
                .as_ref()
                .err()
                .is_some_and(|error| error.is::<ExecutionShutdownUnconfirmed>());
            let (status, response) = match result {
                _ if unconfirmed => (Status::Failed, String::new()),
                _ if cancel.is_cancelled() => (Status::Stopped, String::new()),
                Ok(output) => (Status::Completed, output.chars().take(16_000).collect()),
                Err(_) => (Status::Failed, String::new()),
            };
            let _guard = lock.lock().await;
            if service.update_reply_turn(&context, task_id, &comment_id, status, response).is_err()
            {
                tracing::warn!("could not persist the result of a task ACP reply");
            }
            let mut queues = self.reply_queues.0.lock().unwrap_or_else(|error| error.into_inner());
            if let Some(queue) = queues.get_mut(&key) {
                queue.active = None;
                if unconfirmed {
                    queue.shutdown_unconfirmed = true;
                    queue.waiting.clear();
                    return;
                }
                if queue.waiting.is_empty() {
                    queues.remove(&key);
                    return;
                }
            }
        }
    }

    async fn collect_task_reply(
        &self,
        context: &AgentSchedulerContext,
        task_id: &TaskId,
        comment_id: &str,
        cancel: &CancellationToken,
    ) -> Result<String> {
        let task = self
            .store
            .load_task(context, task_id)?
            .ok_or_else(|| anyhow::anyhow!("task not found"))?;
        let detail = AgentTaskBoardService::new(&self.store)
            .task_detail(context, task_id)?
            .ok_or_else(|| anyhow::anyhow!("task not found"))?;
        let comment = detail
            .collaboration
            .comments
            .iter()
            .find(|comment| comment.id == comment_id)
            .ok_or_else(|| anyhow::anyhow!("task comment not found"))?;
        let turn = comment
            .agent_turn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("task reply turn not found"))?;
        anyhow::ensure!(
            detail.reply_agent.as_deref() == Some(turn.agent.as_str())
                && !task
                    .metadata
                    .get("task_board.archived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            "task ACP reply is no longer available"
        );
        if cancel.is_cancelled() {
            anyhow::bail!("task ACP reply was stopped");
        }
        let cwd = context
            .project_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("task reply directory must be UTF-8"))?
            .to_owned();
        let task_prompt =
            execution_metadata_string(&task, "prompt").unwrap_or_else(|| task.title.clone());
        let mut prompt = format!(
            "Task {}\n{}\n\nTask description:\n{}\n",
            task_id.as_str(),
            task.title,
            task_prompt
        );
        if let Some(parent) = detail
            .collaboration
            .comments
            .iter()
            .find(|parent| Some(&parent.id) == comment.reply_to.as_ref())
        {
            prompt.push_str("\nReplying to:\n");
            prompt.push_str(&parent.body);
            for link in &parent.attachments {
                prompt.push('\n');
                prompt.push_str(link);
            }
            prompt.push('\n');
        }
        prompt.push_str("\nLatest task reply:\n");
        prompt.push_str(&comment.body);
        for link in &comment.attachments {
            prompt.push('\n');
            prompt.push_str(link);
        }
        let request = LocalAgentExecutionRequest {
            project_path: context.project_path().into(),
            scheduler_context: context.clone(),
            task_id: task_id.clone(),
            run_id: RunId::new(comment_id)?,
            title: task.title.clone(),
            prompt,
            model: execution_metadata_string(&task, "model"),
            agent: None,
            acp_agent: Some(turn.agent.clone()),
            reasoning_effort: reasoning_effort_field(&task.metadata)
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            workspace: LocalAgentWorkspace { slot_id: None, path: cwd.clone(), branch: None },
        };
        let mut body = gateway_local_agent_stream_request(&request);
        body.session_id = Some(turn.session_id.clone().into());
        if let Some(options) = body.options.as_mut().and_then(Value::as_object_mut) {
            options.remove("acp_force_new_session");
        }
        let tenant = context.scope().project_scope().tenant_scope().clone();
        let result = H::chat_stream().collect_task_reply(tenant, cwd, body, cancel.clone()).await?;
        Ok(result.output)
    }
}
