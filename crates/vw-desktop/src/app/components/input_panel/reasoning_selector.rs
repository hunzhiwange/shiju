use crate::app::App;
use desktop_chat_input::input_panel::reasoning_selector as domain;
use vw_api_chat_identity::ReasoningEffortDto;

pub(crate) fn model_reasoning_effort_options(
    app: &App,
    model: &str,
) -> Vec<Option<ReasoningEffortDto>> {
    domain::model_reasoning_effort_options(&app.model_settings.providers, model)
}

pub(super) struct SessionReasoning {
    pub options: Vec<Option<ReasoningEffortDto>>,
    pub current: Option<ReasoningEffortDto>,
}

pub(super) fn session_reasoning(app: &App, model: &str) -> Option<SessionReasoning> {
    let runtime = app.current_session_runtime_ref();
    if runtime.is_some_and(|runtime| runtime.acp_agent.is_some()) {
        return None;
    }
    let options = model_reasoning_effort_options(app, model);
    if options.len() <= 1 {
        return None;
    }
    let current = runtime.and_then(|runtime| runtime.reasoning_effort);
    let current = current.filter(|value| options.contains(&Some(*value)));
    Some(SessionReasoning { options, current })
}
