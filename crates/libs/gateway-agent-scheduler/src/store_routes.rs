//! Store-backed scheduler gateway API actions.
use super::*;
#[async_trait]
impl<H: GatewayExecutorHost> AgentSchedulerGateway for StoreBackedAgentSchedulerGateway<H> {
    async fn create_task(
        &self,
        _request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        Err(ApiError::bad_request("explicit agent scheduler scope is required"))
    }

    async fn create_task_scoped(
        &self,
        context: AgentSchedulerContext,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        let task = new_task_from_request(request)?;
        let task_id = task.id.clone();
        let existed =
            self.store.load_task(&context, &task_id).map_err(map_scheduler_error)?.is_some();
        let stored = self.store.create_task(&context, task).map_err(map_scheduler_error)?;
        if !existed {
            append_task_created_event(&self.store, &context, &stored)?;
        }

        Ok(AgentSchedulerCreateTaskResponse {
            task_id: stored.id.as_str().to_string(),
            task: task_to_dto(stored),
        })
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        let project_path = parse_project_path(&query.project_path)?;
        let tasks = self
            .store
            .list_active_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_to_dto)
            .collect();
        Ok(AgentSchedulerTasksResponse { tasks })
    }

    async fn wake(
        &self,
        request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        self.apply_policy_tick(None, &project_path, request.now_ms, false).await?;
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        let max_concurrent = normalize_project_wake_max_concurrent(request.max_concurrent);
        let active_count =
            self.active_dispatch_finite_task_count(&project_path, request.dispatch_mode)?;
        if active_count >= max_concurrent {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                active_count,
                max_concurrent,
                "agent scheduler wake skipped because concurrency limit is reached"
            );
            return Ok(AgentSchedulerWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        }
        let engine = DispatchEngine::new(&self.store)
            .with_dispatch_mode(dispatch_mode_from_dto(request.dispatch_mode));
        let outcome = engine
            .dispatch_once(&project_path, request.now_ms)
            .await
            .map_err(map_scheduler_error)?;
        Ok(wake_response_from_outcome(outcome))
    }

    async fn recover(
        &self,
        request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let engine = RecoveryEngine::new(&self.store);
        let outcome = engine
            .reconcile_once(&project_path, request.now_ms)
            .await
            .map_err(map_scheduler_error)?;
        Ok(recovery_response_from_outcome(outcome))
    }

    async fn list_events(
        &self,
        query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError> {
        let project_path = parse_project_path(&query.project_path)?;
        let limit = query.limit.unwrap_or(DEFAULT_EVENT_LIMIT).min(MAX_EVENT_LIMIT);
        let events = self
            .store
            .list_events_after(&project_path, query.after_cursor, limit)
            .map_err(map_scheduler_error)?;
        let cursor = events.last().map_or(query.after_cursor.unwrap_or(0), |event| event.cursor);
        let events = events.into_iter().map(event_to_dto).collect();
        Ok(AgentSchedulerEventsResponse { cursor, events })
    }

    async fn drain_outbox(
        &self,
        request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        let limit = request.limit.unwrap_or(DEFAULT_OUTBOX_DRAIN_LIMIT).min(MAX_OUTBOX_DRAIN_LIMIT);
        let dispatch_adapter = LocalOnlyDispatchAdapter;
        let dispatcher = OutboxDispatcher::new(&self.store, &dispatch_adapter);
        let (summary, messages) = dispatcher
            .drain_messages_once(&project_path, request.now_ms, limit)
            .await
            .map_err(map_scheduler_error)?;
        let messages = messages.into_iter().map(outbox_to_dto).collect();
        Ok(AgentSchedulerOutboxDrainResponse { claimed: summary.claimed, messages })
    }

    async fn command(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectCommandRequest,
    ) -> Result<AgentSchedulerProjectCommandResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let task_id = task_id_from_api(&request.task_id)?;
        let now_ms = current_timestamp_ms();
        if task_board_write_command(request.command) {
            return self
                .apply_task_board_command(&project_id, &project_path, &task_id, request, now_ms)
                .await;
        }
        let task_before = self
            .store
            .load_task(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let outcome = self
            .apply_project_command(
                &project_id,
                &project_path,
                &task_before,
                request.command,
                request.dispatch_mode,
                request.max_concurrent,
                now_ms,
            )
            .await?;
        let task_after = self
            .store
            .load_task(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let event = append_command_event(
            &self.store,
            &project_path,
            &task_after,
            outcome.as_ref(),
            request.command,
            task_before.status,
            request.reason.as_deref(),
            now_ms,
        )?;
        let task = task_board_task_to_dto(
            &self.store,
            &project_id,
            &project_path,
            task_after,
            outcome.as_ref(),
        )?;
        let event = task_board_event_to_dto(&project_id, event)?;
        Ok(AgentSchedulerProjectCommandResponse {
            accepted: true,
            task: Some(task),
            event: Some(event),
        })
    }

    async fn bulk_command(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut results = Vec::new();
        for command in request.commands {
            match self.command(project_id.clone(), project_path.clone(), command).await {
                Ok(result) => {
                    if result.accepted {
                        accepted += 1;
                    } else {
                        rejected += 1;
                    }
                    results.push(result);
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        project_id = %project_id,
                        "agent scheduler bulk command item rejected"
                    );
                    rejected += 1;
                    results.push(AgentSchedulerProjectCommandResponse {
                        accepted: false,
                        task: None,
                        event: None,
                    });
                    if request.stop_on_error {
                        break;
                    }
                }
            }
        }
        Ok(AgentSchedulerBulkCommandResponse { accepted, rejected, results })
    }

    async fn task_board_workflow(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let workflow =
            self.store.load_task_board_workflow(&project_path).map_err(map_scheduler_error)?;
        Ok(AgentSchedulerTaskBoardWorkflowResponse { project_id: project_id.into(), workflow })
    }

    async fn task_board_status(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let tasks = self
            .store
            .list_task_board_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(|task| {
                task_board_task_to_dto_with_latest_run(
                    &self.store,
                    &project_id,
                    &project_path,
                    task,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let workflow =
            self.store.load_task_board_workflow(&project_path).map_err(map_scheduler_error)?;
        Ok(AgentSchedulerTaskBoardStatusResponse {
            project_id: project_id.into(),
            read_model: AgentSchedulerTaskBoardSnapshotReadModelDto { workflow },
            tasks,
        })
    }

    async fn task_graph(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        self.task_graph_snapshot(project_id, &project_path)
    }

    async fn replace_task_graph(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let dependencies = request
            .edges
            .into_iter()
            .map(|edge| {
                Ok(AgentTaskDependency::new(
                    TaskId::new(edge.predecessor_task_id).map_err(map_bad_request)?,
                    TaskId::new(edge.successor_task_id).map_err(map_bad_request)?,
                ))
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        self.store
            .replace_task_dependencies(&project_path, dependencies)
            .map_err(map_bad_request)?;
        self.task_graph_snapshot(project_id, &project_path)
    }

    async fn update_task_metadata(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        task_id: String,
        request: AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let outcome = AgentTaskBoardService::new(&self.store)
            .update_metadata(
                &project_path,
                &task_id,
                request.patch,
                request.reason.as_deref(),
                current_timestamp_ms(),
            )
            .map_err(map_task_board_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let task = task_board_task_to_dto_with_latest_run(
            &self.store,
            &project_id,
            &project_path,
            outcome.task,
        )?;
        let event = task_board_event_to_dto(&project_id, outcome.event)?;
        Ok(AgentSchedulerTaskMetadataUpdateResponse { task, event: Some(event) })
    }

    async fn import_legacy_tasks(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let outcome = AgentTaskBoardService::new(&self.store)
            .import_legacy_tasks(
                &project_path,
                request.workflow,
                request.tasks,
                request.dry_run,
                current_timestamp_ms(),
            )
            .map_err(map_task_board_error)?;
        let tasks = outcome
            .tasks
            .into_iter()
            .map(|task| {
                task_board_task_to_dto_with_latest_run(
                    &self.store,
                    &project_id,
                    &project_path,
                    task,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let events = outcome
            .events
            .into_iter()
            .map(|event| task_board_event_to_dto(&project_id, event))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AgentSchedulerLegacyImportResponse {
            imported: outcome.imported,
            skipped: outcome.skipped,
            tasks,
            events,
        })
    }

    async fn task_logs(
        &self,
        project_path: AgentSchedulerContext,
        task_id: String,
    ) -> Result<AgentSchedulerTaskLogsResponse, ApiError> {
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let logs = self
            .store
            .list_task_logs(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_log_to_dto)
            .collect();
        Ok(AgentSchedulerTaskLogsResponse { task_id: task_id.as_str().into(), logs })
    }

    async fn task_artifacts(
        &self,
        project_path: AgentSchedulerContext,
        task_id: String,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, ApiError> {
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let artifacts = self
            .store
            .list_task_artifacts(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_artifact_to_dto)
            .collect();
        Ok(AgentSchedulerTaskArtifactsResponse { task_id: task_id.as_str().into(), artifacts })
    }

    async fn task_board_events(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        query: AgentSchedulerProjectEventsQuery,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, ApiError> {
        let limit = query.limit.unwrap_or(DEFAULT_EVENT_LIMIT).min(MAX_EVENT_LIMIT);
        let events = self
            .store
            .list_events_after(&project_path, query.after_cursor, limit)
            .map_err(map_scheduler_error)?;
        let cursor = events.last().map_or(query.after_cursor.unwrap_or(0), |event| event.cursor);
        let events = events
            .into_iter()
            .map(|event| task_board_event_to_dto(&project_id, event))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AgentSchedulerTaskBoardEventsResponse { cursor, events })
    }

    async fn wake_project(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectWakeRequest,
    ) -> Result<AgentSchedulerProjectWakeResponse, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            dispatch_mode = ?request.dispatch_mode,
            now_ms = request.now_ms.0,
            "agent scheduler project wake requested"
        );
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        self.apply_policy_tick(
            Some(project_id.as_str()),
            &project_path,
            request.now_ms,
            request.auto_promote_pool_tasks,
        )
        .await?;
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        let max_concurrent = normalize_project_wake_max_concurrent(request.max_concurrent);
        let active_count =
            self.active_dispatch_finite_task_count(&project_path, request.dispatch_mode)?;
        if active_count >= max_concurrent {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                active_count,
                max_concurrent,
                "agent scheduler project wake skipped because concurrency limit is reached"
            );
            return Ok(AgentSchedulerProjectWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        }
        if request.dispatch_mode != AgentSchedulerDispatchModeDto::Local {
            let outcome = self
                .dispatch_project_once(&project_path, request.now_ms, request.dispatch_mode)
                .await?;
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                dispatched = outcome.is_some(),
                task_id = ?outcome.as_ref().map(|outcome| outcome.task_id.as_str()),
                run_id = ?outcome.as_ref().map(|outcome| outcome.run_id.as_str()),
                outbox_message_id = ?outcome.as_ref().and_then(|outcome| outcome.outbox_message_id.as_deref()),
                "agent scheduler project wake dispatch finished"
            );
            return Ok(project_wake_response_from_dispatch_outcome(outcome));
        }
        let Some(task) = ready_queued_finite_task(&self.store, &project_path, None)
            .map_err(map_scheduler_error)?
        else {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                "agent scheduler local project wake found no queued finite task"
            );
            return Ok(AgentSchedulerProjectWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        };
        let outcome = self
            .run_local_task(owned_scheduler_context(&project_path)?, task.id, request.now_ms)
            .await?;
        tracing::info!(
            dispatched = outcome.is_some(),
            task_id = ?outcome.as_ref().map(|outcome| outcome.task_id.as_str()),
            run_id = ?outcome.as_ref().map(|outcome| outcome.run.id.as_str()),
            "agent scheduler local project wake started task"
        );
        Ok(project_wake_response_from_local_outcome(outcome))
    }

    async fn drain_project_outbox(
        &self,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectOutboxDrainRequest,
    ) -> Result<AgentSchedulerProjectOutboxDrainResponse, ApiError> {
        let limit = request.limit.unwrap_or(DEFAULT_OUTBOX_DRAIN_LIMIT).min(MAX_OUTBOX_DRAIN_LIMIT);
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            now_ms = request.now_ms.0,
            limit,
            "agent scheduler project outbox drain requested"
        );
        let dispatch_adapter = LocalOnlyDispatchAdapter;
        let dispatcher = OutboxDispatcher::new(&self.store, &dispatch_adapter);
        let (summary, messages) = dispatcher
            .drain_messages_once(&project_path, request.now_ms, limit)
            .await
            .map_err(map_scheduler_error)?;
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            claimed = summary.claimed,
            delivered = summary.delivered,
            failed = summary.failed,
            "agent scheduler project outbox drain finished"
        );
        let messages = messages.into_iter().map(outbox_to_dto).collect();
        Ok(AgentSchedulerProjectOutboxDrainResponse { claimed: summary.claimed, messages })
    }
}
