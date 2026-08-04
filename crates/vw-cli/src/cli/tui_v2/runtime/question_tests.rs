use std::collections::BTreeMap;

use super::question::{
    QuestionFormRuntime, reject_question_form, submit_elicitation, submit_question_form,
};
use super::super::model::{
    UiQuestionOption, UiQuestionOverlay, UiQuestionPrompt,
};
use super::super::model::elicitation::{
    ElicitationField, ElicitationKind, ElicitationRequest, ElicitationResponse,
};

#[derive(Default)]
struct RecordingForms {
    questions: Vec<(String, Vec<Vec<String>>)>,
    rejected: Vec<String>,
    elicitations: Vec<(String, ElicitationResponse)>,
}

impl QuestionFormRuntime for RecordingForms {
    fn reply_question(
        &mut self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<(), String> {
        self.questions.push((request_id.to_string(), answers));
        Ok(())
    }

    fn reject_question(&mut self, request_id: &str) -> Result<(), String> {
        self.rejected.push(request_id.to_string());
        Ok(())
    }

    fn reply_elicitation(
        &mut self,
        request_id: &str,
        response: ElicitationResponse,
    ) -> Result<(), String> {
        self.elicitations.push((request_id.to_string(), response));
        Ok(())
    }
}

fn question() -> UiQuestionOverlay {
    UiQuestionOverlay {
        request_id: "q-1".to_string(),
        session_id: "s-1".to_string(),
        prompts: vec![
            UiQuestionPrompt {
                header: "Pick".to_string(),
                question: "Choose one".to_string(),
                options: vec![UiQuestionOption {
                    label: "A".to_string(),
                    description: String::new(),
                    preview: None,
                }],
                multiple: false,
                allow_custom_input: true,
            },
            UiQuestionPrompt {
                header: "Many".to_string(),
                question: "Choose many".to_string(),
                options: Vec::new(),
                multiple: true,
                allow_custom_input: true,
            },
        ],
        answers: vec![vec!["custom".to_string()], vec!["x".to_string(), "y".to_string()]],
        tool: None,
        selected_index: 1,
    }
}

#[test]
fn tui_question_and_mcp_elicitation_forms_submit_exact_responses() {
    let mut runtime = RecordingForms::default();
    let question = question();
    submit_question_form(&question, &mut runtime).unwrap();
    reject_question_form(&question, &mut runtime).unwrap();
    assert_eq!(runtime.questions[0], ("q-1".to_string(), question.answers.clone()));
    assert_eq!(runtime.rejected, ["q-1"]);

    let form = ElicitationRequest {
        id: "e-form".to_string(),
        server_id: "mcp-a".to_string(),
        kind: ElicitationKind::Form {
            fields: vec![ElicitationField {
                name: "token".to_string(),
                label: "Token".to_string(),
                required: true,
                value: "abc".to_string(),
            }],
        },
    };
    submit_elicitation(&form, true, &mut runtime).unwrap();
    assert_eq!(
        runtime.elicitations[0],
        (
            "e-form".to_string(),
            ElicitationResponse::Accept {
                values: BTreeMap::from([("token".to_string(), "abc".to_string())]),
            },
        )
    );

    let mut url = ElicitationRequest {
        id: "e-url".to_string(),
        server_id: "mcp-a".to_string(),
        kind: ElicitationKind::Url {
            url: "https://example.test/authorize".to_string(),
            confirmed: false,
        },
    };
    assert!(submit_elicitation(&url, true, &mut runtime).unwrap_err().contains("confirmation"));
    if let ElicitationKind::Url { confirmed, .. } = &mut url.kind {
        *confirmed = true;
    }
    submit_elicitation(&url, true, &mut runtime).unwrap();

    let invalid = ElicitationRequest {
        id: "bad-url".to_string(),
        server_id: "mcp-a".to_string(),
        kind: ElicitationKind::Url {
            url: "file:///etc/passwd".to_string(),
            confirmed: true,
        },
    };
    assert!(submit_elicitation(&invalid, true, &mut runtime).is_err());

    let mut incomplete = question;
    incomplete.answers[1].clear();
    assert!(submit_question_form(&incomplete, &mut runtime).is_err());
}
