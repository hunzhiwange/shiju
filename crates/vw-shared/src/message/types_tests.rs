use serde_json::{Map, Value, json};

use super::*;
use crate::snapshot::{DiffStatus, FileDiff};

fn base(id: &str) -> PartBase {
    PartBase {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
    }
}

fn cache() -> TokenCacheInfo {
    TokenCacheInfo { read: 3, write: 5 }
}

fn tokens() -> TokenInfo {
    TokenInfo { total: Some(21), input: 13, output: 8, reasoning: 2, cache: cache() }
}

fn path() -> PathInfo {
    PathInfo { cwd: "/work/current".to_string(), root: "/work".to_string() }
}

fn user_info() -> UserInfo {
    let mut tools = std::collections::HashMap::new();
    tools.insert("shell".to_string(), true);

    UserInfo {
        id: "user-1".to_string(),
        session_id: "session-1".to_string(),
        time: UserTime { created: 10 },
        summary: Some(FileDiffSummary {
            title: Some("Changed files".to_string()),
            body: Some("One file changed".to_string()),
            diffs: vec![FileDiff {
                file: "src/lib.rs".to_string(),
                before: "old".to_string(),
                after: "new".to_string(),
                additions: 2,
                deletions: 1,
                status: Some(DiffStatus::Modified),
            }],
        }),
        agent: "coder".to_string(),
        model: ModelRef { provider_id: "openai".to_string(), model_id: "gpt".to_string() },
        system: Some("system prompt".to_string()),
        tools: Some(tools),
        variant: Some("default".to_string()),
    }
}

fn assistant_info() -> AssistantInfo {
    AssistantInfo {
        id: "assistant-1".to_string(),
        session_id: "session-1".to_string(),
        time: AssistantTime {
            created: 11,
            completed: Some(12),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        },
        error: Some(AssistantError::APIError {
            message: "rate limited".to_string(),
            status_code: Some(429),
            is_retryable: true,
            response_headers: Some(std::collections::HashMap::from([(
                "retry-after".to_string(),
                "1".to_string(),
            )])),
            response_body: Some("body".to_string()),
            metadata: Some(std::collections::HashMap::from([(
                "request".to_string(),
                "abc".to_string(),
            )])),
        }),
        parent_id: "user-1".to_string(),
        model_id: "gpt".to_string(),
        provider_id: "openai".to_string(),
        mode: "build".to_string(),
        agent: "coder".to_string(),
        path: path(),
        summary: Some(false),
        cost: 0.25,
        tokens: tokens(),
        variant: Some("default".to_string()),
        finish: Some("stop".to_string()),
    }
}

fn file_source_text() -> FileSourceText {
    FileSourceText { value: "fn main() {}".to_string(), start: 1, end: 13 }
}

fn file_source_base() -> FileSourceBase {
    FileSourceBase { text: file_source_text() }
}

fn metadata() -> Map<String, Value> {
    Map::from_iter([("key".to_string(), json!("value"))])
}

fn part_variants() -> Vec<Part> {
    vec![
        Part::Text(TextPart {
            base: base("text-1"),
            text: "hello".to_string(),
            synthetic: Some(true),
            ignored: Some(false),
            time: Some(PartTime { start: 1, end: Some(2) }),
            metadata: Some(metadata()),
        }),
        Part::Subtask(SubtaskPart {
            base: base("subtask-1"),
            prompt: "Do it".to_string(),
            description: "A subtask".to_string(),
            agent: "coder".to_string(),
            model: Some(SubtaskModel {
                provider_id: "openai".to_string(),
                model_id: "gpt".to_string(),
            }),
            command: Some("cargo check".to_string()),
        }),
        Part::Reasoning(ReasoningPart {
            base: base("reasoning-1"),
            text: "thinking".to_string(),
            metadata: Some(metadata()),
            time: PartTime { start: 3, end: Some(4) },
        }),
        Part::File(FilePart {
            base: base("file-1"),
            mime: "text/plain".to_string(),
            filename: Some("lib.rs".to_string()),
            url: "file:///lib.rs".to_string(),
            source: Some(FilePartSource::File(FileSource {
                base: file_source_base(),
                path: "src/lib.rs".to_string(),
            })),
        }),
        Part::Tool(ToolPart {
            base: base("tool-1"),
            call_id: "call-1".to_string(),
            tool: "shell".to_string(),
            state: ToolState::Completed(ToolStateCompleted {
                input: metadata(),
                output: "ok".to_string(),
                title: "Run shell".to_string(),
                metadata: metadata(),
                time: ToolStateCompletedTime { start: 5, end: 6, compacted: Some(7) },
                attachments: Some(vec![ToolAttachment {
                    mime: "text/plain".to_string(),
                    url: "file:///out.txt".to_string(),
                }]),
            }),
            metadata: Some(metadata()),
        }),
        Part::StepStart(StepStartPart {
            base: base("step-start-1"),
            snapshot: Some("snap-1".to_string()),
        }),
        Part::StepFinish(StepFinishPart {
            base: base("step-finish-1"),
            reason: "done".to_string(),
            snapshot: Some("snap-2".to_string()),
            cost: 1.5,
            tokens: tokens(),
        }),
        Part::Snapshot(SnapshotPart { base: base("snapshot-1"), snapshot: "snap-3".to_string() }),
        Part::Patch(PatchPart {
            base: base("patch-1"),
            hash: "hash-1".to_string(),
            files: vec!["src/lib.rs".to_string()],
            additions: Some(5),
            deletions: Some(2),
        }),
        Part::Agent(AgentPart {
            base: base("agent-1"),
            name: "reviewer".to_string(),
            source: Some(file_source_text()),
        }),
        Part::Retry(RetryPart {
            base: base("retry-1"),
            attempt: 2,
            error: AssistantError::MessageAbortedError { message: "cancelled".to_string() },
            time: RetryTime { created: 8 },
        }),
        Part::Compaction(CompactionPart { base: base("compaction-1"), auto: true }),
    ]
}

#[test]
fn patch_part_line_stats_are_optional_and_round_trip() {
    let value = serde_json::to_value(PatchPart {
        base: base("patch-stats"),
        hash: "hash-stats".to_string(),
        files: vec!["src/lib.rs".to_string()],
        additions: Some(8),
        deletions: Some(3),
    })
    .unwrap();
    let parsed: PatchPart = serde_json::from_value(value.clone()).unwrap();
    let legacy: PatchPart = serde_json::from_value(serde_json::json!({
        "id": "patch-legacy",
        "sessionID": "session-1",
        "messageID": "message-1",
        "hash": "hash-legacy",
        "files": ["src/legacy.rs"]
    }))
    .unwrap();

    assert_eq!(value["additions"], 8);
    assert_eq!(value["deletions"], 3);
    assert_eq!(parsed.additions, Some(8));
    assert_eq!(parsed.deletions, Some(3));
    assert_eq!(legacy.additions, None);
    assert_eq!(legacy.deletions, None);
}

#[test]
fn info_user_serializes_wire_names_and_nested_summary() {
    let info = Info::User(Box::new(user_info()));

    let value = serde_json::to_value(&info).unwrap();
    let parsed: Info = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value["role"], "user");
    assert_eq!(value["sessionID"], "session-1");
    assert_eq!(value["model"]["providerID"], "openai");
    assert_eq!(value["summary"]["diffs"][0]["status"], "modified");
    assert!(matches!(parsed, Info::User(user) if user.id == "user-1"));
}

#[test]
fn info_assistant_serializes_error_tokens_and_path() {
    let info = Info::Assistant(Box::new(assistant_info()));

    let value = serde_json::to_value(&info).unwrap();
    let parsed: Info = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value["role"], "assistant");
    assert_eq!(value["parentID"], "user-1");
    assert_eq!(value["providerID"], "openai");
    assert_eq!(value["tokens"]["cache"]["write"], 5);
    assert_eq!(value["error"]["name"], "APIError");
    assert_eq!(value["error"]["status_code"], 429);
    assert!(
        matches!(parsed, Info::Assistant(assistant) if assistant.finish.as_deref() == Some("stop"))
    );
}

#[test]
fn optional_info_fields_are_omitted_when_absent() {
    let user = UserInfo { summary: None, system: None, tools: None, variant: None, ..user_info() };
    let assistant = AssistantInfo {
        time: AssistantTime {
            created: 1,
            completed: None,
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        },
        error: None,
        summary: None,
        tokens: TokenInfo { total: None, ..tokens() },
        variant: None,
        finish: None,
        ..assistant_info()
    };

    let user_value = serde_json::to_value(Info::User(Box::new(user))).unwrap();
    let assistant_value = serde_json::to_value(Info::Assistant(Box::new(assistant))).unwrap();

    assert!(user_value.get("summary").is_none());
    assert!(user_value.get("system").is_none());
    assert!(user_value.get("tools").is_none());
    assert!(user_value.get("variant").is_none());
    assert!(assistant_value["time"].get("completed").is_none());
    assert!(assistant_value.get("error").is_none());
    assert!(assistant_value["tokens"].get("total").is_none());
    assert!(assistant_value.get("finish").is_none());
}

#[test]
fn assistant_time_elapsed_fields_are_optional_and_round_trip() {
    let legacy: AssistantTime = serde_json::from_str(r#"{"created":100,"completed":250}"#).unwrap();
    assert_eq!(legacy.processing_elapsed_ms, None);
    assert_eq!(legacy.llm_elapsed_ms, None);

    let time = AssistantTime {
        created: 100,
        completed: Some(250),
        processing_elapsed_ms: Some(140),
        llm_elapsed_ms: Some(125),
    };
    let value = serde_json::to_value(&time).unwrap();
    assert_eq!(value["processing_elapsed_ms"], 140);
    assert_eq!(value["llm_elapsed_ms"], 125);
    let round_trip = serde_json::from_value::<AssistantTime>(value).unwrap();
    assert_eq!(round_trip.created, time.created);
    assert_eq!(round_trip.completed, time.completed);
    assert_eq!(round_trip.processing_elapsed_ms, time.processing_elapsed_ms);
    assert_eq!(round_trip.llm_elapsed_ms, time.llm_elapsed_ms);
}

#[test]
fn assistant_error_variants_use_name_tag_and_optional_fields() {
    let errors = [
        AssistantError::ProviderAuthError {
            provider_id: "openai".to_string(),
            message: "login required".to_string(),
        },
        AssistantError::MessageOutputLengthError,
        AssistantError::MessageAbortedError { message: "aborted".to_string() },
        AssistantError::ContextOverflowError {
            message: "too much context".to_string(),
            response_body: None,
        },
        AssistantError::Unknown { message: "unknown".to_string() },
    ];

    let names: Vec<String> = errors
        .into_iter()
        .map(|error| {
            let value = serde_json::to_value(error).unwrap();
            value["name"].as_str().unwrap().to_string()
        })
        .collect();

    assert_eq!(
        names,
        vec![
            "ProviderAuthError",
            "MessageOutputLengthError",
            "MessageAbortedError",
            "ContextOverflowError",
            "Unknown"
        ]
    );
}

#[test]
fn info_mutators_update_only_supported_fields() {
    let mut user = Info::User(Box::new(user_info()));
    let mut assistant = Info::Assistant(Box::new(assistant_info()));

    user.set_id("user-2");
    user.set_session_id("session-2");
    user.set_parent_id("ignored-parent");
    assistant.set_id("assistant-2");
    assistant.set_session_id("session-2");
    assistant.set_parent_id("parent-2");

    assert_eq!(user.id(), "user-2");
    assert!(matches!(
        user,
        Info::User(ref user) if user.session_id == "session-2"
    ));
    assert_eq!(assistant.id(), "assistant-2");
    assert!(matches!(
        assistant,
        Info::Assistant(ref assistant)
            if assistant.session_id == "session-2" && assistant.parent_id == "parent-2"
    ));
}

#[test]
fn part_variants_round_trip_with_expected_type_tags() {
    let expected_types = [
        "text",
        "subtask",
        "reasoning",
        "file",
        "tool",
        "step-start",
        "step-finish",
        "snapshot",
        "patch",
        "agent",
        "retry",
        "compaction",
    ];

    for (part, expected_type) in part_variants().into_iter().zip(expected_types) {
        let value = serde_json::to_value(&part).unwrap();
        let parsed: Part = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(value["type"], expected_type);
        assert_eq!(parsed.id(), part.id());
        assert_eq!(parsed.session_id(), "session-1");
        assert_eq!(parsed.message_id(), "message-1");
    }
}

#[test]
fn part_mutators_update_base_fields_for_every_variant() {
    for mut part in part_variants() {
        part.set_id("part-2");
        part.set_session_id("session-2");
        part.set_message_id("message-2");

        assert_eq!(part.id(), "part-2");
        assert_eq!(part.session_id(), "session-2");
        assert_eq!(part.message_id(), "message-2");
    }
}

#[test]
fn file_part_source_variants_use_lowercase_type_tags() {
    let sources = [
        FilePartSource::File(FileSource {
            base: file_source_base(),
            path: "src/lib.rs".to_string(),
        }),
        FilePartSource::Symbol(SymbolSource {
            base: file_source_base(),
            path: "src/main.rs".to_string(),
            range: LspRange {
                start: LspPosition { line: 1, character: 2 },
                end: LspPosition { line: 3, character: 4 },
            },
            name: "main".to_string(),
            kind: 12,
        }),
        FilePartSource::Resource(ResourceSource {
            base: file_source_base(),
            client_name: "docs".to_string(),
            uri: "https://example.test/doc".to_string(),
        }),
    ];

    let values: Vec<Value> = sources
        .into_iter()
        .map(|source| {
            let value = serde_json::to_value(source).unwrap();
            serde_json::from_value::<FilePartSource>(value.clone()).unwrap();
            value
        })
        .collect();

    assert_eq!(values[0]["type"], "file");
    assert_eq!(values[1]["type"], "symbol");
    assert_eq!(values[1]["range"]["end"]["character"], 4);
    assert_eq!(values[2]["type"], "resource");
    assert_eq!(values[2]["clientName"], "docs");
}

#[test]
fn tool_state_variants_use_lowercase_status_tags() {
    let states = [
        ToolState::Pending(ToolStatePending {
            input: metadata(),
            raw: "{\"key\":\"value\"}".to_string(),
        }),
        ToolState::Running(ToolStateRunning {
            input: metadata(),
            title: None,
            metadata: None,
            time: PartTime { start: 1, end: None },
        }),
        ToolState::Completed(ToolStateCompleted {
            input: metadata(),
            output: "done".to_string(),
            title: "Complete".to_string(),
            metadata: metadata(),
            time: ToolStateCompletedTime { start: 1, end: 2, compacted: None },
            attachments: None,
        }),
        ToolState::Error(ToolStateError {
            input: metadata(),
            error: "failed".to_string(),
            metadata: None,
            time: PartTime { start: 1, end: Some(2) },
        }),
    ];

    let statuses: Vec<String> = states
        .into_iter()
        .map(|state| {
            let value = serde_json::to_value(state).unwrap();
            serde_json::from_value::<ToolState>(value.clone()).unwrap();
            value["status"].as_str().unwrap().to_string()
        })
        .collect();

    assert_eq!(statuses, vec!["pending", "running", "completed", "error"]);
}

#[test]
fn optional_part_fields_are_omitted_when_absent() {
    let text = TextPart {
        base: base("text-min"),
        text: "hello".to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    };
    let file = FilePart {
        base: base("file-min"),
        mime: "text/plain".to_string(),
        filename: None,
        url: "file:///min.txt".to_string(),
        source: None,
    };

    let text_value = serde_json::to_value(text).unwrap();
    let file_value = serde_json::to_value(file).unwrap();

    assert!(text_value.get("synthetic").is_none());
    assert!(text_value.get("ignored").is_none());
    assert!(text_value.get("time").is_none());
    assert!(text_value.get("metadata").is_none());
    assert!(file_value.get("filename").is_none());
    assert!(file_value.get("source").is_none());
}

#[test]
fn with_parts_round_trips_info_and_parts_together() {
    let message =
        WithParts { info: Info::Assistant(Box::new(assistant_info())), parts: part_variants() };

    let value = serde_json::to_value(&message).unwrap();
    let parsed: WithParts = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value["info"]["role"], "assistant");
    assert_eq!(value["parts"].as_array().unwrap().len(), 12);
    assert_eq!(parsed.info.id(), "assistant-1");
    assert_eq!(parsed.parts.len(), 12);
}

#[test]
fn file_diff_summary_defaults_missing_diffs_to_empty() {
    let summary: FileDiffSummary = serde_json::from_value(json!({
        "title": "No changes",
        "body": null
    }))
    .unwrap();

    assert_eq!(summary.title.as_deref(), Some("No changes"));
    assert!(summary.body.is_none());
    assert!(summary.diffs.is_empty());
}

#[test]
fn nested_wire_names_are_preserved_for_subtask_and_tool_parts() {
    let subtask = Part::Subtask(SubtaskPart {
        base: base("subtask-wire"),
        prompt: "Inspect".to_string(),
        description: "Inspect the workspace".to_string(),
        agent: "reviewer".to_string(),
        model: Some(SubtaskModel {
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
        }),
        command: None,
    });
    let tool = Part::Tool(ToolPart {
        base: base("tool-wire"),
        call_id: "call-9".to_string(),
        tool: "read".to_string(),
        state: ToolState::Pending(ToolStatePending { input: Map::new(), raw: "{}".to_string() }),
        metadata: None,
    });

    let subtask_value = serde_json::to_value(subtask).unwrap();
    let tool_value = serde_json::to_value(tool).unwrap();

    assert_eq!(subtask_value["model"]["providerID"], "provider");
    assert_eq!(subtask_value["model"]["modelID"], "model");
    assert!(subtask_value.get("command").is_none());
    assert_eq!(tool_value["callID"], "call-9");
    assert!(tool_value.get("metadata").is_none());
}

#[test]
fn optional_state_and_event_fields_are_omitted_when_absent() {
    let running = ToolState::Running(ToolStateRunning {
        input: Map::new(),
        title: None,
        metadata: None,
        time: PartTime { start: 1, end: None },
    });
    let completed = ToolState::Completed(ToolStateCompleted {
        input: Map::new(),
        output: "done".to_string(),
        title: "Done".to_string(),
        metadata: Map::new(),
        time: ToolStateCompletedTime { start: 1, end: 2, compacted: None },
        attachments: None,
    });
    let error = ToolState::Error(ToolStateError {
        input: Map::new(),
        error: "failed".to_string(),
        metadata: None,
        time: PartTime { start: 1, end: None },
    });
    let agent = Part::Agent(AgentPart {
        base: base("agent-min"),
        name: "reviewer".to_string(),
        source: None,
    });
    let step_start =
        Part::StepStart(StepStartPart { base: base("step-start-min"), snapshot: None });

    let running_value = serde_json::to_value(running).unwrap();
    let completed_value = serde_json::to_value(completed).unwrap();
    let error_value = serde_json::to_value(error).unwrap();
    let agent_value = serde_json::to_value(agent).unwrap();
    let step_start_value = serde_json::to_value(step_start).unwrap();

    assert!(running_value.get("title").is_none());
    assert!(running_value.get("metadata").is_none());
    assert!(running_value["time"].get("end").is_none());
    assert!(completed_value["time"].get("compacted").is_none());
    assert!(completed_value.get("attachments").is_none());
    assert!(error_value.get("metadata").is_none());
    assert!(error_value["time"].get("end").is_none());
    assert!(agent_value.get("source").is_none());
    assert!(step_start_value.get("snapshot").is_none());
}

#[test]
fn assistant_error_optional_payloads_are_omitted_when_absent() {
    let context = AssistantError::ContextOverflowError {
        message: "too large".to_string(),
        response_body: None,
    };
    let api = AssistantError::APIError {
        message: "unavailable".to_string(),
        status_code: None,
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    };

    let context_value = serde_json::to_value(context).unwrap();
    let api_value = serde_json::to_value(api).unwrap();

    assert!(context_value.get("response_body").is_none());
    assert!(api_value.get("status_code").is_none());
    assert!(api_value.get("response_headers").is_none());
    assert!(api_value.get("response_body").is_none());
    assert!(api_value.get("metadata").is_none());
    assert_eq!(api_value["is_retryable"], false);
}

#[test]
fn unknown_discriminator_values_are_rejected() {
    let unknown_info = json!({ "role": "system" });
    let unknown_part = json!({ "type": "audio" });
    let unknown_source = json!({ "type": "database" });
    let unknown_state = json!({ "status": "cancelled" });

    assert!(serde_json::from_value::<Info>(unknown_info).is_err());
    assert!(serde_json::from_value::<Part>(unknown_part).is_err());
    assert!(serde_json::from_value::<FilePartSource>(unknown_source).is_err());
    assert!(serde_json::from_value::<ToolState>(unknown_state).is_err());
}

#[test]
fn required_wire_fields_are_rejected_when_missing() {
    let missing_message_id = json!({
        "type": "text",
        "id": "text-1",
        "sessionID": "session-1",
        "text": "hello"
    });
    let missing_tool_input = json!({
        "status": "pending",
        "raw": "{}"
    });

    assert!(serde_json::from_value::<Part>(missing_message_id).is_err());
    assert!(serde_json::from_value::<ToolState>(missing_tool_input).is_err());
}
