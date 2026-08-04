use super::super::model::elicitation::{
    ElicitationRequest, ElicitationResponse,
};
use super::super::model::UiQuestionOverlay;

pub(crate) trait QuestionFormRuntime {
    fn reply_question(&mut self, request_id: &str, answers: Vec<Vec<String>>)
        -> Result<(), String>;
    fn reject_question(&mut self, request_id: &str) -> Result<(), String>;
    fn reply_elicitation(
        &mut self,
        request_id: &str,
        response: ElicitationResponse,
    ) -> Result<(), String>;
}

pub(crate) fn submit_question_form(
    overlay: &UiQuestionOverlay,
    runtime: &mut dyn QuestionFormRuntime,
) -> Result<(), String> {
    if overlay.prompts.len() != overlay.answers.len() {
        return Err("question answer shape does not match prompt pages".to_string());
    }
    for (index, (prompt, answers)) in
        overlay.prompts.iter().zip(&overlay.answers).enumerate()
    {
        if answers.is_empty() || answers.iter().all(|answer| answer.trim().is_empty()) {
            return Err(format!("question page {} has no answer", index + 1));
        }
        if !prompt.multiple && answers.len() > 1 {
            return Err(format!("question page {} accepts one answer", index + 1));
        }
    }
    runtime.reply_question(&overlay.request_id, overlay.answers.clone())
}

pub(crate) fn reject_question_form(
    overlay: &UiQuestionOverlay,
    runtime: &mut dyn QuestionFormRuntime,
) -> Result<(), String> {
    runtime.reject_question(&overlay.request_id)
}

pub(crate) fn submit_elicitation(
    request: &ElicitationRequest,
    accept: bool,
    runtime: &mut dyn QuestionFormRuntime,
) -> Result<(), String> {
    let response = request.response(accept)?;
    runtime.reply_elicitation(&request.id, response)
}
