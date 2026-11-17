pub struct QuestionState<'a> {
    pub question_modal_request_id: &'a mut Option<String>,
    pub question_modal_request: &'a mut Option<question_types::Request>,
    pub question_modal_answers: &'a mut Vec<Vec<String>>,
    pub question_modal_custom: &'a mut Vec<String>,
}
pub fn clear_question_modal(app: &mut QuestionState<'_>) {
    *app.question_modal_request_id = None;
    *app.question_modal_request = None;
    app.question_modal_answers.clear();
    app.question_modal_custom.clear();
}
pub fn apply_question_request(app: &mut QuestionState<'_>, requests: Vec<question_types::Request>) {
    let mut pending = requests;
    if pending.is_empty() {
        clear_question_modal(app);
        return;
    }
    pending.sort_by(|a, b| a.id.cmp(&b.id));
    let req = pending[0].clone();
    let reset = app.question_modal_request_id.as_deref() != Some(req.id.as_str());
    *app.question_modal_request = Some(req.clone());
    if reset {
        *app.question_modal_request_id = Some(req.id.clone());
        *app.question_modal_answers = req
            .questions
            .iter()
            .map(|q| {
                let allow_multi = q.multiple.unwrap_or(false);
                if allow_multi {
                    return Vec::new();
                }
                if q.options.iter().any(|o| o.label == "once") {
                    return vec!["once".to_string()];
                }
                Vec::new()
            })
            .collect();
        *app.question_modal_custom = vec![String::new(); req.questions.len()];
    } else {
        let qlen = req.questions.len();
        if app.question_modal_answers.len() != qlen {
            app.question_modal_answers.resize_with(qlen, Vec::new);
        }
        if app.question_modal_custom.len() != qlen {
            app.question_modal_custom.resize_with(qlen, String::new);
        }
    }
}
pub fn handle_question_option_toggled(app: &mut QuestionState<'_>, q_idx: usize, label: String) {
    let Some(req) = app.question_modal_request.as_ref() else {
        return;
    };
    let allow_multi = req.questions.get(q_idx).and_then(|q| q.multiple).unwrap_or(false);
    let Some(existing) = app.question_modal_answers.get_mut(q_idx) else {
        return;
    };
    if allow_multi {
        if let Some(pos) = existing.iter().position(|v| v == &label) {
            existing.remove(pos);
        } else {
            existing.push(label);
        }
    } else {
        existing.clear();
        existing.push(label);
    }
}
pub fn handle_question_custom_changed(app: &mut QuestionState<'_>, q_idx: usize, value: String) {
    let Some(existing) = app.question_modal_custom.get_mut(q_idx) else {
        return;
    };
    *existing = value;
}
pub fn collect_answers(
    req: &question_types::Request,
    selected: &[Vec<String>],
    custom_answers: &[String],
) -> Vec<Vec<String>> {
    let mut answers: Vec<Vec<String>> = Vec::with_capacity(req.questions.len());
    for i in 0..req.questions.len() {
        let allow_multi = req.questions.get(i).and_then(|q| q.multiple).unwrap_or(false);
        let allow_custom = req.questions.get(i).and_then(|q| q.custom).unwrap_or(false);
        let mut a = selected.get(i).cloned().unwrap_or_default();
        let custom = custom_answers.get(i).cloned().unwrap_or_default().trim().to_string();
        if allow_custom && !custom.is_empty() {
            if allow_multi {
                if !a.contains(&custom) {
                    a.push(custom);
                }
            } else {
                a = vec![custom];
            }
        }
        answers.push(a);
    }

    answers
}

#[cfg(test)]
#[path = "questions_tests.rs"]
mod questions_tests;

/// Prepare a reply while keeping invalid forms open for correction.
pub fn prepare_reply(
    state: &mut QuestionState<'_>,
    valid: impl FnOnce(&question_types::Request, &[Vec<String>]) -> bool,
) -> Result<Option<(String, Vec<Vec<String>>)>, ()> {
    let Some(id) = state.question_modal_request_id.clone() else {
        return Ok(None);
    };
    let Some(request) = state.question_modal_request.as_ref() else {
        return Ok(None);
    };
    let answers =
        collect_answers(request, state.question_modal_answers, state.question_modal_custom);
    if !valid(request, &answers) {
        return Err(());
    }
    clear_question_modal(state);
    Ok(Some((id, answers)))
}
pub fn reject_request(state: &mut QuestionState<'_>) -> Option<String> {
    let id = state.question_modal_request_id.clone()?;
    clear_question_modal(state);
    Some(id)
}
pub fn apply_list_result(
    state: &mut QuestionState<'_>,
    result: Result<Vec<question_types::Request>, String>,
) {
    match result {
        Ok(requests) => apply_question_request(state, requests),
        Err(_) => clear_question_modal(state),
    }
}

#[cfg(test)]
#[path = "question_reply_tests.rs"]
mod question_reply_tests;
