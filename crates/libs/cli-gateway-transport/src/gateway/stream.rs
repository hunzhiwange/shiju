use super::*;
#[cfg(not(target_arch = "wasm32"))]
use vw_gateway_client::{GatewayChatStreamEvent, GatewayStreamAttemptResult, GatewayStreamResume};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};

impl GatewayUiRuntime {
    /// 以异步方式发起聊天流，并把 gateway 事件规整为 `UiRuntimeEvent`。
    ///
    /// 返回值始终是统一的终态控制事件，用于让上层明确区分
    /// done、cancel、timeout 与 error。
    pub async fn stream_chat(
        &self,
        body: &GatewayChatStreamRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let directory = self.directory_value();
        let body = self.prepare_stream_request(body);
        let mut terminal = None;

        let result =
            GatewayClient::stream_chat(self.endpoint(), directory.as_deref(), &body, |event| {
                let runtime_event = adapt_gateway_stream_event(event);
                if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                    terminal = Some(runtime_terminal.clone());
                }

                if on_event(runtime_event) {
                    true
                } else {
                    if terminal.is_none() {
                        terminal = Some(cancelled_by_consumer_terminal());
                    }
                    false
                }
            })
            .await
            .map_err(|err| annotate_gateway_transport_error(err, self.endpoint()));

        finalize_stream_terminal(result, terminal)
    }

    /// 以阻塞方式发起聊天流，并把 gateway 事件规整为 `UiRuntimeEvent`。
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn stream_chat_blocking(
        &self,
        body: &GatewayChatStreamRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let directory = self.directory_value();
        let body = self.prepare_stream_request(body);
        let mut terminal = None;

        let result = GatewayClient::stream_chat_blocking(
            self.endpoint(),
            directory.as_deref(),
            &body,
            |event| {
                let runtime_event = adapt_gateway_stream_event(event);
                if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                    terminal = Some(runtime_terminal.clone());
                }

                if on_event(runtime_event) {
                    true
                } else {
                    if terminal.is_none() {
                        terminal = Some(cancelled_by_consumer_terminal());
                    }
                    false
                }
            },
        )
        .map_err(|err| annotate_gateway_transport_error(err, self.endpoint()));

        finalize_stream_terminal(result, terminal)
    }

    /// Reconnect once using the original request and its last delivered SSE sequence.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn stream_chat_resumable_blocking(
        &self,
        body: &GatewayChatStreamRequest,
        request_id: &str,
        cancel: &AtomicBool,
        mut on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
    ) -> Result<GatewayStreamAttemptResult, String> {
        let directory = self.directory_value();
        let body = self.prepare_stream_request(body);
        block_on_gateway(async {
            let mut last_event_id = None;
            let mut first_error = None;
            for attempt in 0..2 {
                if cancel.load(Ordering::Acquire) {
                    return Ok(GatewayStreamAttemptResult::CallerStopped);
                }
                let resume =
                    GatewayStreamResume { request_id: request_id.to_string(), last_event_id };
                let result = GatewayClient::stream_chat_resumable(
                    self.endpoint(),
                    directory.as_deref(),
                    &body,
                    &resume,
                    |event| {
                        if cancel.load(Ordering::Acquire) || !on_event(event.event) {
                            return false;
                        }
                        last_event_id = Some(event.id);
                        true
                    },
                )
                .await;
                match result {
                    Ok(
                        GatewayStreamAttemptResult::Terminated
                        | GatewayStreamAttemptResult::CallerStopped,
                    ) => return result,
                    Err(error) if attempt == 1 => return Err(first_error.unwrap_or(error)),
                    Ok(GatewayStreamAttemptResult::Incomplete) if attempt == 1 => {
                        return first_error.map_or(result, Err);
                    }
                    Err(error) => first_error = Some(error),
                    Ok(GatewayStreamAttemptResult::Incomplete) => {}
                }
                // A replay from sequence zero must also require an existing server request.
                last_event_id = Some(last_event_id.unwrap_or_default());
            }
            unreachable!("the final stream attempt always returns");
        })
    }

    /// Stops exactly one resumable chat stream by its stable request ID.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn chat_stream_stop_blocking(&self, request_id: &str) -> Result<bool, String> {
        block_on_gateway(gateway_session::GatewaySessionClient::new(self.client.transport().clone()).chat_stream_stop(request_id))
    }

}
