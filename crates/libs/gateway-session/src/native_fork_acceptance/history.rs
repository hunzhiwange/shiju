use super::*;
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Prefix {
    Full,
    Empty,
    Compacted,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Branch {
    pub(super) info: Info,
    pub(super) prefix: Prefix,
    pub(super) initial_history: Vec<SessionMessageRecord>,
}

pub(super) fn assert_no_future(value: &str) {
    for marker in [PROMPTS[1], PROMPTS[2], LATE_SUMMARY] {
        assert!(!value.contains(marker), "discarded history leaked: {marker}");
    }
}

pub fn fork_acceptance_negative_detects_discarded_model_history() {
    assert_no_future(
        &json!([{"role":"user", "content": PROMPTS[0]},
        {"role":"user", "content": PROMPTS[1]}, {"role":"user", "content":NEW_USER}])
        .to_string(),
    );
}

pub(super) fn assert_initial_history(history: &[SessionMessageRecord], prefix: Prefix, tool_output: &str) {
    assert_no_future(&serde_json::to_string(history).unwrap());
    if prefix == Prefix::Empty {
        assert!(history.is_empty());
        return;
    }
    let [user, assistant] = history else { panic!("expected one complete turn: {history:?}") };
    assert_ne!(user.message_id, assistant.message_id);
    assert_eq!(user.message.role, ChatRole::User);
    assert_eq!(user.message.content, PROMPTS[0]);
    assert_eq!(assistant.message.role, ChatRole::Assistant);
    let mut lines = assistant.message.content.lines();
    let tool_header = format!("tool {TOOL}");
    assert_eq!(lines.next(), Some(tool_header.as_str()));
    let call: Value = serde_json::from_str(lines.next().expect("tool payload")).unwrap();
    assert_eq!(call["tool_call_id"], CALL);
    assert_eq!(call["status"], "completed");
    assert_eq!(
        serde_json::from_str::<Value>(call["input"].as_str().unwrap()).unwrap(),
        json!({"path":FILE})
    );
    assert_eq!(call["output"], tool_output);
    assert_eq!(lines.next(), Some(ANSWERS[0]));
    assert_eq!(lines.next(), None);
}

pub(super) fn assert_continuation(body: &Value, branch: &Branch, tool_output: &str) {
    assert_eq!(body["model"], "original");
    let messages = body["messages"].as_array().unwrap();
    assert_no_future(&body["messages"].to_string());
    let (system, dialogue) = messages.split_at(
        messages
            .iter()
            .take_while(|message| message["role"] == "system" || message["role"] == "developer")
            .count(),
    );
    assert!(
        system.iter().any(|msg| msg["content"].as_str().is_some_and(
            |text| text.contains(&format!("working directory is `{}`", branch.info.directory))
        )),
        "model request did not use {}",
        branch.info.directory
    );
    let roles = dialogue.iter().map(|msg| msg["role"].as_str().unwrap()).collect::<Vec<_>>();
    match branch.prefix {
        Prefix::Full => {
            assert_eq!(roles, ["user", "assistant", "tool", "assistant", "user"]);
            assert_eq!(dialogue[0]["content"], PROMPTS[0]);
            let calls = dialogue[1]["tool_calls"].as_array().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["id"], CALL);
            assert_eq!(calls[0]["function"]["name"], TOOL);
            assert_eq!(
                serde_json::from_str::<Value>(calls[0]["function"]["arguments"].as_str().unwrap())
                    .unwrap(),
                json!({"path":FILE})
            );
            assert_eq!(dialogue[2]["tool_call_id"], CALL);
            assert_eq!(dialogue[2]["content"], tool_output);
            assert_eq!(dialogue[3]["content"], ANSWERS[0]);
        }
        Prefix::Empty => assert_eq!(roles, ["user"]),
        Prefix::Compacted => {
            assert_eq!(roles, ["assistant", "user"]);
            assert_eq!(dialogue[0]["content"], format!("[Compaction summary]\n{SUMMARY}"));
        }
    }
    assert_eq!(dialogue.last().unwrap()["content"], NEW_USER);
}
