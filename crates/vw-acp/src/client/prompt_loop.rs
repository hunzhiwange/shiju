//! ACP prompt 执行循环。
//!
//! 本模块处理 prompt future、取消信号和 ACP session/update 事件的并发等待。
//! actor 模块只负责准备可用 runtime，并把请求交给这里执行。

use super::*;

impl AcpClient {
    #[cfg(test)]
    pub(super) async fn run_actor_prompt(
        &self,
        runtime: &mut ActorRuntime,
        request: PromptRequest,
        event_tx: mpsc::UnboundedSender<PromptEvent>,
    ) -> Result<PromptResult, AcpError> {
        self.run_actor_prompt_with_controls(
            runtime,
            request,
            event_tx,
            None,
            &mut std::collections::VecDeque::new(),
        )
        .await
    }

    pub(super) async fn run_actor_prompt_with_controls(
        &self,
        runtime: &mut ActorRuntime,
        request: PromptRequest,
        event_tx: mpsc::UnboundedSender<PromptEvent>,
        mut command_rx: Option<&mut mpsc::UnboundedReceiver<ActorCommand>>,
        deferred: &mut std::collections::VecDeque<ActorCommand>,
    ) -> Result<PromptResult, AcpError> {
        let mut controls = Vec::<super::prompt_controls::PromptControlFuture<'_>>::new();
        while runtime.event_rx.try_recv().is_ok() {}

        let mut session_id = self
            .resolve_session(
                &runtime.conn,
                &request.cwd,
                &request.session_strategy,
                &runtime.expected_session_id,
            )
            .await?;
        self.store_reusable_session(Some(session_id.clone()));

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let (completed_tx, completed_rx) = watch::channel(false);
        self.register_active_prompt(session_id.clone(), cancel_tx, completed_rx);

        let prompt_future = runtime.conn.prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.clone()),
            vec![request.prompt.into()],
        ));
        tokio::pin!(prompt_future);

        let mut deltas = Vec::new();
        let mut prompt_error = None::<AcpError>;
        let mut finish_reason = None;
        let mut usage = None;
        let mut prompt_finished = false;
        let mut cancel_sent = false;

        'prompt_loop: loop {
            if prompt_finished {
                let cancel_requested = self.cancelling_session_ids.lock().contains(&session_id);
                if !cancel_sent && cancel_requested {
                    self.cancelling_session_ids.lock().insert(session_id.clone());
                    if let Err(err) = runtime
                        .conn
                        .cancel(acp::CancelNotification::new(acp::SessionId::new(
                            session_id.clone(),
                        )))
                        .await
                    {
                        prompt_error = Some(AcpError::Cancel(err.to_string()));
                    }
                }

                while let Ok(event) = runtime.event_rx.try_recv() {
                    self.handle_prompt_internal_event(
                        event,
                        &mut session_id,
                        &runtime.expected_session_id,
                        &event_tx,
                        &mut deltas,
                    );
                }
                break;
            }

            tokio::select! {
                biased;

                cancel_changed = cancel_rx.changed(), if !cancel_sent => {
                    match cancel_changed {
                        Ok(_) if *cancel_rx.borrow() => {
                            self.cancelling_session_ids.lock().insert(session_id.clone());
                            if let Err(err) = runtime.conn.cancel(acp::CancelNotification::new(acp::SessionId::new(session_id.clone()))).await {
                                prompt_error = Some(AcpError::Cancel(err.to_string()));
                                break 'prompt_loop;
                            }
                            cancel_sent = true;
                        }
                        Ok(_) => {}
                        Err(_) => {
                            cancel_sent = true;
                        }
                    }
                }
                joined = &mut prompt_future, if !prompt_finished => {
                    prompt_finished = true;
                    match joined {
                        Ok(response) => {
                            finish_reason = Some(acp_finish_reason(response.stop_reason));
                            usage = response.usage.as_ref().map(map_usage);
                        }
                        Err(err) => {
                            prompt_error = Some(AcpError::Prompt(err.to_string()));
                        }
                    }
                }
                command = async { command_rx.as_mut().expect("actor command receiver").recv().await }, if command_rx.is_some() => {
                    match command {
                        Some(command) => {
                            if matches!(&command, ActorCommand::Close { .. }) {
                                let _ = self.request_cancel_active_prompt().await;
                            }
                            match super::prompt_controls::control_future(&runtime.conn, command, &session_id, &request.cwd) {
                                Ok(control) => controls.push(control),
                                Err(command) => deferred.push_back(command),
                            }
                        }
                        None => command_rx = None,
                    }
                }
                () = std::future::poll_fn(|cx| {
                    for index in 0..controls.len() {
                        if controls[index].as_mut().poll(cx).is_ready() {
                            drop(controls.swap_remove(index));
                            return Poll::Ready(());
                        }
                    }
                    Poll::Pending
                }), if !controls.is_empty() => {}
                maybe_event = runtime.event_rx.recv() => {
                    match maybe_event {
                        Some(event) => {
                            self.handle_prompt_internal_event(
                                event,
                                &mut session_id,
                                &runtime.expected_session_id,
                                &event_tx,
                                &mut deltas,
                            );
                        }
                        None => {
                            if prompt_finished {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let _ = completed_tx.send(true);
        self.cancelling_session_ids.lock().remove(&session_id);
        self.clear_active_prompt(&session_id);

        if let Some(err) = prompt_error {
            return Err(err);
        }

        Ok(PromptResult { session_id, deltas, finish_reason, usage })
    }

    fn handle_prompt_internal_event(
        &self,
        event: InternalEvent,
        session_id: &mut String,
        expected_session_id: &Arc<Mutex<Option<String>>>,
        event_tx: &mpsc::UnboundedSender<PromptEvent>,
        deltas: &mut Vec<String>,
    ) {
        match event {
            InternalEvent::Delta(delta) => {
                if !delta.is_empty() {
                    deltas.push(delta.clone());
                    let _ = event_tx.send(PromptEvent::TextDelta(delta));
                }
            }
            InternalEvent::SessionChanged { expected, actual } => {
                self.accept_session_change(
                    session_id,
                    expected,
                    actual,
                    expected_session_id,
                    event_tx,
                );
            }
        }
    }

    fn accept_session_change(
        &self,
        session_id: &mut String,
        expected: String,
        actual: String,
        expected_session_id: &Arc<Mutex<Option<String>>>,
        event_tx: &mpsc::UnboundedSender<PromptEvent>,
    ) {
        let _ = event_tx.send(PromptEvent::SessionChanged {
            expected: expected.clone(),
            actual: actual.clone(),
        });
        *expected_session_id.lock() = Some(actual.clone());
        self.update_active_prompt_session(&expected, actual.clone());
        self.store_reusable_session(Some(actual.clone()));
        *session_id = actual;
    }
}
