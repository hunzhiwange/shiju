//! 模型选择和推理强度校验；供应商能力由宿主只读投影。
use crate::message::TaskBoardMessage;
use crate::draft::normalize_task_model;
use desktop_task_model::TaskDraft;
use vw_api_chat_identity::ReasoningEffortDto;

#[derive(Default)]
pub struct ModelEffect { pub clear_hover: bool, pub refresh_models: bool }
pub struct ModelState<'a> {
    pub bulk_acp_agent: &'a mut Option<String>,
    pub bulk_model_input: &'a mut String,
    pub bulk_model_popover: &'a mut bool,
    pub bulk_reasoning_effort: &'a mut Option<ReasoningEffortDto>,
    pub draft: &'a mut TaskDraft,
    pub last_model: &'a mut String,
    pub model_popover: &'a mut bool,
}

pub fn reasoning_model_for_message(message: &TaskBoardMessage) -> Option<String> {
    match message {
        TaskBoardMessage::UpdateDraftModel(value)
        | TaskBoardMessage::UpdateEditingTaskModel(value)
        | TaskBoardMessage::UpdateEditingTaskModelInput(value)
        | TaskBoardMessage::ModelSelected(value)
        | TaskBoardMessage::UpdateBulkModelInput(value) => Some(value.clone()),
        TaskBoardMessage::BulkModelSelected(value) => Some(normalize_task_model(value)),
        _ => None,
    }
}

pub fn update_model(state: ModelState<'_>, message: TaskBoardMessage, options: &[Option<ReasoningEffortDto>], models_need_refresh: bool) -> Result<ModelEffect, TaskBoardMessage> {
    let mut effect = ModelEffect::default();
    match message {
        TaskBoardMessage::UpdateDraftModel(v) => {
            (*state.draft).model = v;
            if (*state.draft).acp_agent.is_none() && !options.contains(&(*state.draft).reasoning_effort) {
                (*state.draft).reasoning_effort = None;
            }
            (*state.last_model) = (*state.draft).model.clone();
        }
        TaskBoardMessage::UpdateEditingTaskModel(model) => {
            (*state.draft).model = model;
            if (*state.draft).acp_agent.is_none() && !options.contains(&(*state.draft).reasoning_effort) {
                (*state.draft).reasoning_effort = None;
            }
            (*state.last_model) = normalize_task_model(&(*state.draft).model);
            (*state.model_popover) = false;
        }
        TaskBoardMessage::UpdateEditingTaskModelInput(model) => {
            (*state.draft).model = model;
            if (*state.draft).acp_agent.is_none() && !options.contains(&(*state.draft).reasoning_effort) {
                (*state.draft).reasoning_effort = None;
            }
            (*state.last_model) = normalize_task_model(&(*state.draft).model);
        }
        TaskBoardMessage::ToggleModelPopover => {
            let new = !(*state.model_popover);
            (*state.model_popover) = new;
            if new {
                (*state.bulk_model_popover) = false;
                effect.clear_hover = true;
                if models_need_refresh {
                    effect.refresh_models = true;
                }
            }
        }
        TaskBoardMessage::ModelSelected(model) => {
            (*state.draft).model = model;
            if (*state.draft).acp_agent.is_none() && !options.contains(&(*state.draft).reasoning_effort) {
                (*state.draft).reasoning_effort = None;
            }
            (*state.last_model) = (*state.draft).model.clone();
            (*state.model_popover) = false;
            effect.clear_hover = true;
        }
        TaskBoardMessage::CloseModelPopover => {
            (*state.model_popover) = false;
            effect.clear_hover = true;
        }
        TaskBoardMessage::UpdateBulkModelInput(value) => {
            (*state.bulk_model_input) = value;
            if (*state.bulk_acp_agent).is_none() && !options.contains(&(*state.bulk_reasoning_effort)) {
                (*state.bulk_reasoning_effort) = None;
            }
        }
        TaskBoardMessage::ToggleBulkModelPopover => {
            let new = !(*state.bulk_model_popover);
            (*state.bulk_model_popover) = new;
            if new {
                (*state.model_popover) = false;
                effect.clear_hover = true;
                if models_need_refresh {
                    effect.refresh_models = true;
                }
            }
        }
        TaskBoardMessage::BulkModelSelected(model) => {
            (*state.bulk_model_input) = normalize_task_model(&model);
            if (*state.bulk_acp_agent).is_none() && !options.contains(&(*state.bulk_reasoning_effort)) {
                (*state.bulk_reasoning_effort) = None;
            }
            (*state.last_model) = (*state.bulk_model_input).clone();
            (*state.bulk_model_popover) = false;
            effect.clear_hover = true;
        }
        TaskBoardMessage::CloseBulkModelPopover => {
            (*state.bulk_model_popover) = false;
            effect.clear_hover = true;
        }
        other => return Err(other),
    }
    Ok(effect)
}

pub fn validated_reasoning_effort(
    acp_agent: Option<&str>, effort: Option<ReasoningEffortDto>,
    options: &[Option<ReasoningEffortDto>],
) -> Option<ReasoningEffortDto> {
    if acp_agent.is_none() && !options.contains(&effort) { None } else { effort }
}
