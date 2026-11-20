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
        mut request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&context);
        let _dispatch_guard = dispatch_lock.lock().await;
        let placement = request.placement.take().map(placement_from_dto).transpose()?;
        let mut task = new_task_from_request(request)?;
        let task_id = task.id.clone();
        let existed =
            self.store.load_task(&context, &task_id).map_err(map_scheduler_error)?.is_some();
        if !existed && placement.is_some() {
            if task.kind != AgentTaskKind::Finite {
                return Err(ApiError::bad_request("only finite tasks can be placed in a work plan"));
            }
            let workflow =
                load_dispatch_workflow(&self.store, &context).map_err(map_scheduler_error)?;
            let initial = workflow.statuses.first()
                .ok_or_else(|| ApiError::internal("project workflow has no initial status"))?;
            let mut metadata = task_metadata_object(task.metadata)?;
            for key in [
                "task_board.status_key_map",
                "task_board.status_label",
                "task_board.stage_id",
                "task_board.stage_label",
            ] {
                metadata.remove(key);
            }
            task.status = task_scheduler::task_board::apply_workflow_status(
                &workflow, &initial.key, &mut metadata,
            ).map_err(map_bad_request)?;
            task.metadata = Value::Object(metadata);
        } else if !existed
            && task.kind == AgentTaskKind::Finite
            && let Some(key) = string_field(&task.metadata, "task_board.status_key")
            && let Some(workflow) = self.store.load_task_board_workflow(&context).map_err(map_scheduler_error)?
            && let Some(metadata) = task.metadata.as_object_mut()
        {
            task.status = task_scheduler::task_board::apply_workflow_status(&workflow, &key, metadata)
                .map_err(map_bad_request)?;
        }
        let (stored, placement) = if let Some(placement) = placement {
            let stored = self.store.create_task_with_plan(&context, task, placement.clone())
                .map_err(map_scheduler_error)?;
            (stored, Some(placement_to_dto(placement)))
        } else {
            let stored = self.store.create_task(&context, task).map_err(map_scheduler_error)?;
            if !existed {
                append_task_created_event(&self.store, &context, &stored)?;
            }
            let placement = if existed {
                load_placement_dto(&self.store, &context, &stored.id)?
            } else {
                None
            };
            (stored, placement)
        };

        Ok(AgentSchedulerCreateTaskResponse {
            task_id: stored.id.as_str().to_string(),
            task: task_to_dto(stored, placement),
        })
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        let project_path = parse_project_path(&query.project_path)?;
        let mut placements = project_placements(&self.store, &project_path)?;
        let tasks = self
            .store
            .list_active_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(|task| {
                let placement = placements.remove(&task.id);
                task_to_dto(task, placement)
            })
            .collect();
        Ok(AgentSchedulerTasksResponse { tasks })
    }

    async fn wake(
        &self,
        request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.apply_policy_tick(None, &project_path, request.now_ms, false).await?;
        let max_concurrent = normalize_project_wake_max_concurrent(request.max_concurrent);
        let engine = DispatchEngine::new(&self.store)
            .with_dispatch_mode(dispatch_mode_from_dto(request.dispatch_mode))
            .with_max_concurrent(max_concurrent);
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
            let dispatch_lock = self.project_dispatch_lock(&project_path);
            let _dispatch_guard = dispatch_lock.lock().await;
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
        let placement = load_placement_dto(&self.store, &project_path, &task_after.id)?;
        let task = task_board_task_to_dto(
            &self.store,
            &project_id,
            &project_path,
            task_after,
            outcome.as_ref(),
            placement,
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

    async fn select_task_board_workflow(
        &self,
        project_id: String,
        context: AgentSchedulerContext,
        request: AgentSchedulerSelectWorkflowRequest,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, ApiError> {
        use task_scheduler::store::tasks::WorkflowSelectionError;

        let dispatch_lock = self.project_dispatch_lock(&context);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        let workflow = self.store.select_task_board_workflow(
            &context,
            request.workflow_id,
            request.expected_workflow,
        ).await.map_err(|error| match error {
            WorkflowSelectionError::Invalid(message) => ApiError::bad_request(message),
            WorkflowSelectionError::NotFound(message) => ApiError::not_found(message),
            WorkflowSelectionError::Conflict(message) => ApiError::conflict(message),
            WorkflowSelectionError::Storage(error) => map_scheduler_error(error),
        })?;
        Ok(AgentSchedulerTaskBoardWorkflowResponse {
            project_id: project_id.into(),
            workflow: Some(workflow),
        })
    }

    async fn task_labels(
        &self,
        context: AgentSchedulerContext,
    ) -> Result<Vec<AgentSchedulerTaskLabelDto>, ApiError> {
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        self.store.load_task_labels(&context).map_err(map_scheduler_error)
    }

    async fn mutate_task_labels(
        &self,
        context: AgentSchedulerContext,
        request: AgentSchedulerTaskLabelMutation,
    ) -> Result<Vec<AgentSchedulerTaskLabelDto>, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&context);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        self.store
            .edit_task_labels(&context, |labels| {
                mutate_label_directory(labels, request).map_err(anyhow::Error::new)
            })
            .map_err(|error| error.downcast::<ApiError>().unwrap_or_else(map_scheduler_error))
    }

    async fn task_properties(
        &self,
        context: AgentSchedulerContext,
    ) -> Result<Vec<AgentSchedulerTaskPropertyDto>, ApiError> {
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        self.store.load_task_properties(&context).map_err(map_scheduler_error)
    }

    async fn mutate_task_properties(
        &self,
        context: AgentSchedulerContext,
        request: AgentSchedulerTaskPropertyMutation,
    ) -> Result<Vec<AgentSchedulerTaskPropertyDto>, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&context);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        self.store
            .edit_task_properties(&context, |properties| {
                mutate_property_directory(properties, request).map_err(anyhow::Error::new)
            })
            .map_err(|error| error.downcast::<ApiError>().unwrap_or_else(map_scheduler_error))
    }

    async fn task_board_status(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let workflow = load_dispatch_workflow(&self.store, &project_path).map_err(map_scheduler_error)?;
        let readiness = DispatchReadinessSnapshot::load(&self.store, &project_path)
            .map_err(map_scheduler_error)?;
        let mut placements = project_placements(&self.store, &project_path)?;
        let tasks = self
            .store
            .list_task_board_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(|task| {
                let status_key = task_scheduler::task_board::task_board_status_key(&task);
                let placement = placements.remove(&task.id);
                let run = self.store.load_latest_run_for_task(&project_path, &task.id)
                    .map_err(map_scheduler_error)?;
                let blocker = if task.kind == AgentTaskKind::Finite
                    && matches!(task.status, AgentTaskStatus::Draft | AgentTaskStatus::Queued)
                {
                    readiness.blocker_for(&task.id).map_err(map_scheduler_error)?
                } else {
                    None
                };
                let mut dto = task_board_task_to_dto(
                    &self.store, &project_id, &project_path, task, run.as_ref(), placement,
                )?;
                dto.read_model.dispatch_blocker = blocker.map(dispatch_blocker_to_dto);
                if let Some(status) = workflow.statuses.iter().find(|status| status.key == status_key) {
                    if dto.read_model.stage_id.as_deref() == Some(dto.status_key.as_str()) {
                        dto.read_model.stage_id = Some(status_key.clone());
                    }
                    dto.status_key = status_key;
                    dto.read_model.workflow_id = Some(workflow.id.clone());
                    dto.read_model.status_label = Some(status.label.clone());
                    dto.read_model.status_role = Some(status.role);
                }
                Ok(dto)
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        Ok(AgentSchedulerTaskBoardStatusResponse {
            project_id: project_id.into(),
            read_model: AgentSchedulerTaskBoardSnapshotReadModelDto {
                workflow: Some(workflow),
                labels: self.store.load_task_labels(&project_path).map_err(map_scheduler_error)?,
                properties: self.store.load_task_properties(&project_path).map_err(map_scheduler_error)?,
            },
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

    async fn update_task_plan(
        &self,
        project_id: String,
        context: AgentSchedulerContext,
        task_id: String,
        request: AgentSchedulerTaskPlanUpdateRequest,
    ) -> Result<AgentSchedulerTaskBoardTaskDto, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&context);
        let _dispatch_guard = dispatch_lock.lock().await;
        self.store.initialize_project(&context).map_err(map_scheduler_error)?;
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let placement = match request.placement {
            AgentSchedulerTaskPlanChangeDto::Keep => None,
            AgentSchedulerTaskPlanChangeDto::Set { placement } => {
                Some(Some(placement_from_dto(placement)?))
            }
            AgentSchedulerTaskPlanChangeDto::Clear => Some(None),
        };
        let task = if let Some(placement) = placement {
            self.store.edit_task_with_plan(
                &context,
                &task_id,
                placement,
                Default::default(),
                |current| Ok(AgentTaskBoardUpdate {
                    title: None,
                    status: None,
                    metadata: current.metadata.clone(),
                    updated_at_ms: current_timestamp_ms(),
                }),
            ).map_err(map_scheduler_error)?.map(|(_, updated)| updated)
        } else {
            self.store.load_task(&context, &task_id).map_err(map_scheduler_error)?
        }.ok_or_else(|| ApiError::not_found("task not found"))?;

        task_board_task_to_dto_with_latest_run(&self.store, &project_id, &context, task)
    }

    async fn update_task_metadata(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        task_id: String,
        request: AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, ApiError> {
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
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
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
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
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        {
            let dispatch_lock = self.project_dispatch_lock(&project_path);
            let _dispatch_guard = dispatch_lock.lock().await;
            if let Err(error) = RecoveryEngine::new(&self.store)
                .reconcile_once(&project_path, request.now_ms)
                .await
            {
                tracing::warn!(%error, "scheduler periodic recovery failed");
            }
            self.apply_policy_tick(
                Some(project_id.as_str()),
                &project_path,
                request.now_ms,
                request.auto_promote_pool_tasks,
            ).await?;
        }
        let runs = self.fill_project_slots(
            &project_path,
            request.now_ms,
            request.dispatch_mode,
            normalize_project_wake_max_concurrent(request.max_concurrent),
        ).await?;
        Ok(project_wake_response(runs))
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

fn mutate_label_directory(
    labels: &mut Vec<AgentSchedulerTaskLabelDto>,
    request: AgentSchedulerTaskLabelMutation,
) -> Result<(), ApiError> {
    use AgentSchedulerTaskLabelMutation::{Archive, Create, Update};

    let (id, name, color) = match request {
        Create { name, color } => (None, Some(name), Some(color)),
        Update { id, name, color } => (Some(id), name, color),
        Archive { id } => {
            let label = labels.iter_mut().find(|label| label.id == id)
                .ok_or_else(|| ApiError::not_found("task label not found"))?;
            label.archived = true;
            return Ok(());
        }
    };
    let name = name.as_deref().map(str::trim);
    if let Some(name) = name {
        if name.is_empty() {
            return Err(ApiError::bad_request("task label name must not be empty"));
        }
        let normalized = name.to_lowercase();
        if labels.iter().any(|label| {
            Some(&label.id) != id.as_ref() && label.name.to_lowercase() == normalized
        }) {
            return Err(ApiError::conflict("task label name already exists"));
        }
    }
    let color = color.as_deref().map(str::trim);
    if color.is_some_and(|color| {
        color.len() != 7 || !color.starts_with('#')
            || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    }) {
        return Err(ApiError::bad_request("task label color must use #RRGGBB"));
    }
    if let Some(id) = id {
        let label = labels.iter_mut().find(|label| label.id == id)
            .ok_or_else(|| ApiError::not_found("task label not found"))?;
        if let Some(name) = name {
            label.name = name.to_owned();
        }
        if let Some(color) = color {
            label.color = color.to_ascii_uppercase();
        }
    } else if let (Some(name), Some(color)) = (name, color) {
        labels.push(AgentSchedulerTaskLabelDto {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            color: color.to_ascii_uppercase(),
            archived: false,
        });
    }
    Ok(())
}

fn mutate_property_directory(
    properties: &mut Vec<AgentSchedulerTaskPropertyDto>,
    request: AgentSchedulerTaskPropertyMutation,
) -> Result<(), ApiError> {
    use AgentSchedulerTaskPropertyMutation::{Archive, Create, Update};

    match request {
        Create { name, kind, options } => {
            let name = validate_property_name(properties, None, name)?;
            if kind == AgentSchedulerTaskPropertyKind::SingleSelect && options.is_empty() {
                return Err(ApiError::bad_request("single select properties require options"));
            }
            let mut property = AgentSchedulerTaskPropertyDto {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                kind,
                options: Vec::new(),
                archived: false,
            };
            add_property_options(&mut property, options)?;
            properties.push(property);
        }
        Update { id, name, new_options, option_changes } => {
            let name = name.map(|name| validate_property_name(properties, Some(&id), name))
                .transpose()?;
            let property = properties.iter_mut().find(|property| property.id == id)
                .ok_or_else(|| ApiError::not_found("task property not found"))?;
            if property.archived {
                return Err(ApiError::conflict("task property is archived"));
            }
            if let Some(name) = name {
                property.name = name;
            }
            for (id, change) in option_changes {
                let option = property.options.iter_mut().find(|option| option.id == id)
                    .ok_or_else(|| ApiError::not_found("task property option not found"))?;
                if let Some(name) = change.name {
                    if option.archived {
                        return Err(ApiError::conflict("task property option is archived"));
                    }
                    option.name = non_empty(name.trim().to_owned(), "task property option name")?;
                }
                option.archived |= change.archive;
            }
            add_property_options(property, new_options)?;
        }
        Archive { id } => {
            let property = properties.iter_mut().find(|property| property.id == id)
                .ok_or_else(|| ApiError::not_found("task property not found"))?;
            property.archived = true;
        }
    }
    Ok(())
}

fn validate_property_name(
    properties: &[AgentSchedulerTaskPropertyDto],
    id: Option<&str>,
    name: String,
) -> Result<String, ApiError> {
    let name = non_empty(name.trim().to_owned(), "task property name")?;
    let normalized = name.to_lowercase();
    if properties.iter().any(|property| {
        Some(property.id.as_str()) != id && property.name.to_lowercase() == normalized
    }) {
        return Err(ApiError::conflict("task property name already exists"));
    }
    Ok(name)
}

fn add_property_options(
    property: &mut AgentSchedulerTaskPropertyDto,
    new_options: Vec<String>,
) -> Result<(), ApiError> {
    if property.kind != AgentSchedulerTaskPropertyKind::SingleSelect && !new_options.is_empty() {
        return Err(ApiError::bad_request("only single select properties accept options"));
    }
    for option in new_options {
        property.options.push(AgentSchedulerTaskPropertyOption {
            id: uuid::Uuid::new_v4().to_string(),
            name: non_empty(option.trim().to_owned(), "task property option name")?,
            archived: false,
        });
    }
    let mut names = std::collections::HashSet::new();
    if property.options.iter().any(|option| !names.insert(option.name.to_lowercase())) {
        return Err(ApiError::bad_request("task property option names must be unique"));
    }
    Ok(())
}
