use super::messages;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn acp_model_source_messages_chat_accepts_disabled_thinking_and_tool_cache_hints() {
    let mut request = json!({"model":"selected","max_tokens":64,
        "thinking":{"type":"disabled"},
        "messages":[{"role":"user","content":"hello"}],
        "tools":[{"name":"Read","input_schema":{"type":"object"},
            "cache_control":{"type":"ephemeral","ttl":"1h"}}]
    });
    let converted = messages::to_chat(&request).unwrap();
    assert!(converted.get("thinking").is_none());
    assert_eq!(converted["tools"][0]["function"]["name"], "Read");
    request["tools"][0]["cache_control"]["type"] = "persistent".into();
    assert_eq!(messages::to_chat(&request), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn acp_model_source_messages_chat_accepts_empty_reasoning_without_losing_state() {
    let mut response = json!({"id":"response","choices":[{"message":{
        "role":"assistant","content":"answer","reasoning_content":null,"tool_calls":null},
        "finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}});
    for empty in [serde_json::Value::Null, json!("")] {
        response["choices"][0]["message"]["reasoning_content"] = empty;
        assert_eq!(
            messages::from_chat(&response, "selected").unwrap()["content"][0]["text"],
            "answer"
        );
    }
    response["choices"][0]["message"]["reasoning_content"] = "requires replay".into();
    assert_eq!(messages::from_chat(&response, "selected"), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn acp_model_source_alibaba_native_preserves_thinking_and_tool_cache_hints() {
    let mut request = json!({
        "model":"deepseek-v4.1-flash-1017", "max_tokens":4096,
        "metadata":{"user_id":"private-native-account"},
        "thinking":{"type":"adaptive"},
        "output_config":{"effort":"high"},
        "context_management":{"edits":[]},
        "messages":[
            {"role":"assistant","content":[
                {"type":"thinking","thinking":"plan","signature":"native-signature"},
                {"type":"tool_use","id":"read-1","name":"Read","input":{}}
            ]},
            {"role":"user","content":[
                {"type":"tool_result","tool_use_id":"read-1","content":"file contents"}
            ]}
        ],
        "tools":[{"name":"Read","input_schema":{"type":"object"},
            "cache_control":{"type":"ephemeral"}}]
    });
    let original = request.clone();
    assert_eq!(messages::to_chat(&request), Err(StatusCode::UNPROCESSABLE_ENTITY));
    messages::prepare_hosted_native(&mut request).unwrap();
    let mut expected = original;
    expected.as_object_mut().unwrap().remove("metadata");
    assert_eq!(request, expected);
    request["mcp_servers"] = json!([{"url":"https://example.invalid"}]);
    assert_eq!(
        messages::prepare_hosted_native(&mut request),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn acp_model_source_official_deepseek_rejects_missing_thinking_in_current_tool_turn() {
    let request = json!({
        "thinking":{"type":"adaptive"},
        "metadata":{"user_id":"private"},
        "messages":[
            {"role":"user","content":"Look it up"},
            {"role":"assistant","content":[{"type":"tool_use","id":"t","name":"lookup","input":{}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"42"}]}
        ]
    });
    let mut legacy = request.clone();
    assert_eq!(
        messages::prepare_deepseek_native(&mut legacy),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(legacy["thinking"]["type"], "adaptive");
    assert!(legacy.get("metadata").is_none());
    assert_eq!(legacy["messages"], request["messages"]);

    let mut next_turn = request.clone();
    next_turn["messages"].as_array_mut().unwrap().extend([
        json!({"role":"assistant","content":"42"}),
        json!({"role":"user","content":"What else?"}),
    ]);
    messages::prepare_deepseek_native(&mut next_turn).unwrap();
    assert_eq!(next_turn["thinking"]["type"], "adaptive");

    let mut current = request;
    current["messages"][1]["content"].as_array_mut().unwrap().insert(
        0,
        json!({"type":"thinking","thinking":"Need to look up the answer","signature":"native"}),
    );
    let history = current["messages"].clone();
    messages::prepare_deepseek_native(&mut current).unwrap();
    assert_eq!(current["thinking"]["type"], "adaptive");
    assert_eq!(current["messages"], history);
}

#[test]
fn acp_model_source_official_deepseek_rejects_server_side_tools() {
    let mut request = json!({"messages": [], "mcp_servers": []});
    assert_eq!(
        messages::prepare_deepseek_native(&mut request),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
    let mut request = json!({"messages": [], "tools": [{"type": "web_search"}]});
    assert_eq!(
        messages::prepare_deepseek_native(&mut request),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn acp_model_source_relay_messages_preserves_tool_roundtrip_and_system() {
    let request = json!({"model":"selected","max_tokens":64,"system":[{"type":"text","text":"Be concise"}],"messages":[
        {"role":"user","content":"hello"},
        {"role":"assistant","content":[{"type":"text","text":"Checking"},{"type":"tool_use","id":"call-a","name":"lookup","input":{"city":"杭州"}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"call-a","content":[{"type":"text","text":"sunny"}]},{"type":"text","text":"continue"}]}
    ],"tools":[{"name":"lookup","description":"lookup weather","input_schema":{"type":"object"}}],"tool_choice":{"type":"auto"}});
    let chat = messages::to_chat(&request).unwrap();
    assert_eq!(
        chat["messages"],
        json!([
            {"role":"system","content":"Be concise"}, {"role":"user","content":"hello"},
            {"role":"assistant","content":"Checking","tool_calls":[{"id":"call-a","type":"function","function":{"name":"lookup","arguments":"{\"city\":\"杭州\"}"}}]},
            {"role":"tool","tool_call_id":"call-a","content":"sunny"}, {"role":"user","content":"continue"}
        ])
    );
    assert_eq!(chat["tools"][0]["function"]["parameters"], json!({"type":"object"}));
    assert_eq!(chat["tool_choice"], "auto");
}

#[test]
fn acp_model_source_relay_messages_preserves_failed_tool_results_in_history() {
    let mut request = json!({"model":"selected","max_tokens":64,"messages":[
        {"role":"assistant","content":[
            {"type":"tool_use","id":"read-a","name":"Read","input":{}},
            {"type":"tool_use","id":"read-b","name":"Read","input":{}}
        ]},
        {"role":"user","content":[
            {"type":"tool_result","tool_use_id":"read-a","content":"data","is_error":false},
            {"type":"tool_result","tool_use_id":"read-b","content":[{"type":"text","text":"File missing"}],"is_error":true},
            {"type":"text","text":"Try another path"}
        ]}
    ]});
    let chat = messages::to_chat(&request).unwrap();
    assert_eq!(
        chat["messages"].as_array().unwrap()[1..],
        json!([
            {"role":"tool","tool_call_id":"read-a","content":"data"},
            {"role":"tool","tool_call_id":"read-b","content":"Tool execution failed:\nFile missing"},
            {"role":"user","content":"Try another path"}
        ])
        .as_array()
        .unwrap()[..]
    );
    request["messages"][1]["content"][1]["is_error"] = json!("true");
    assert_eq!(messages::to_chat(&request), Err(StatusCode::BAD_REQUEST));
}

#[test]
fn acp_model_source_relay_messages_rejects_unrepresentable_state() {
    for block in [
        json!({"type":"thinking","thinking":"secret","signature":"sig"}),
        json!({"type":"image","source":{"type":"url","url":"https://example.invalid"}}),
        json!({"type":"server_tool_use","id":"s","name":"web_search","input":{}}),
        json!({"type":"text","text":"quote","citations":[{}]}),
    ] {
        assert_eq!(
            messages::to_chat(
                &json!({"model":"selected","max_tokens":64,"messages":[{"role":"assistant","content":[block]}]})
            ),
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
}


#[test]
fn acp_model_source_relay_messages_response_stop_usage_and_capability_errors() {
    let response = json!({"id":"response","choices":[{"message":{"role":"assistant","content":"answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":4,"total_tokens":15}});
    let converted = messages::from_chat(&response, "selected").unwrap();
    assert_eq!(converted["content"], json!([{"type":"text","text":"answer"}]));
    assert_eq!(converted["stop_reason"], "end_turn");
    assert_eq!(converted["usage"], json!({"input_tokens":11,"output_tokens":4}));
    for key in ["reasoning_content", "annotations", "audio"] {
        let mut unsupported = response.clone();
        unsupported["choices"][0]["message"][key] = json!({"unconvertible":true});
        assert_eq!(
            messages::from_chat(&unsupported, "selected"),
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
    let mut invalid = response.clone();
    invalid["choices"][0]["finish_reason"] = "content_filter".into();
    assert_eq!(messages::from_chat(&invalid, "selected"), Err(StatusCode::UNPROCESSABLE_ENTITY));
    for field in ["previous_response_id", "container"] {
        let mut request = json!({"model":"selected","max_tokens":64,"messages":[{"role":"user","content":"hello"}]});
        request[field] = json!("unrepresentable state");
        assert_eq!(messages::to_chat(&request), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
}

#[test]
fn acp_model_source_claude_messages_annotations_preserve_tools_without_native_identity() {
    let body = json!({"model":"selected", "max_tokens":64,
    "metadata":{"user_id":"native-user-must-not-be-forwarded"},
    "system":[{"type":"text","text":"system", "cache_control":{"type":"ephemeral","ttl":"1h","scope":"global"}}],
    "messages":[
        {"role":"user","content":[{"type":"text","text":"read", "cache_control":{"type":"ephemeral"}}]},
        {"role":"assistant","content":[{"type":"tool_use","id":"read-1","name":"Read","input":{"file_path":"challenge.txt"}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"read-1","content":"challenge", "cache_control":{"type":"ephemeral"}}]}
    ]});
    let converted = messages::to_chat(&body).unwrap();
    assert_eq!(converted["messages"][0], json!({"role":"system","content":"system"}));
    assert_eq!(
        converted["messages"][3],
        json!({"role":"tool","tool_call_id":"read-1","content":"challenge"})
    );
    let serialized = converted.to_string();
    assert!(!serialized.contains("native-user-must-not-be-forwarded"));
    assert!(!serialized.contains("cache_control"));
    for invalid in [
        json!({"type":"persistent"}),
        json!({"type":"ephemeral","ttl":"forever"}),
        json!({"type":"ephemeral","unknown":true}),
        json!({"type":"ephemeral","scope":"unverified"}),
    ] {
        let mut invalid_body = body.clone();
        invalid_body["system"][0]["cache_control"] = invalid;
        assert!(messages::to_chat(&invalid_body).is_err());
    }
    let mut thinking = body;
    thinking["thinking"] = json!({"type":"adaptive"});
    assert_eq!(messages::to_chat(&thinking), Err(StatusCode::UNPROCESSABLE_ENTITY));
}
