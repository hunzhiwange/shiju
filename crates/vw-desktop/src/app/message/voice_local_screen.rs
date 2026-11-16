//! Local screen frames use the ordinary chat image path and its existing permissions.
use super::*;
use vw_api_voice::ScreenPayload;

pub(in crate::app::message::voice) fn screen_call(app: &App) -> Result<u128, &'static str> {
    let runtime = app.current_session_runtime();
    let supported = !runtime.auto_model
        && app.model_settings.providers.iter().any(|provider| {
            provider.models.iter().any(|model| {
                model.enabled
                    && format!("{}/{}", provider.id, model.id) == runtime.model
                    && model.detail["capabilities"]["input"]["image"].as_bool() == Some(true)
            })
        });
    desktop_voice_ui::message::voice_local_chat::screen_call(
        app.voice.local_conversation.as_ref(), app.voice.conversation_scope == super::super::scope(app), app.session_state.active_session_id.as_deref(), app.voice.local_stop.is_some(), ready_to_send(app), supported,
    )
}

pub(in crate::app::message::voice) fn share_screen(
    app: &mut App,
    call: u128,
    payload: ScreenPayload,
) -> Task<Message> {
    if screen_call(app) != Ok(call) {
        return app.show_error_toast("语音会话或模型已变化，未发送屏幕；请重新分享。");
    }
    match desktop_voice_ui::message::voice_local_chat::screen_input(payload) {
        Ok((input, caption)) => send_request(app, input, caption),
        Err(error) => app.show_error_toast(error),
    }
}
