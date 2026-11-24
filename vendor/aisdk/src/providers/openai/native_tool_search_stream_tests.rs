use super::OpenAI;
use crate::core::{
    DynamicModel,
    language_model::{
        LanguageModel, LanguageModelOptions, LanguageModelResponseContentType,
        LanguageModelStreamChunk,
    },
};
use futures::StreamExt;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
fn call(id: &str, arguments: Value) -> Value {
    json!({"type":"tool_search_call","id":format!("item-{id}"),"call_id":id,"execution":"client","status":"completed","arguments":arguments})
}
fn item(kind: &str, index: u32, value: Value) -> Value {
    json!({"type":format!("response.output_item.{kind}"),"sequence_number":1,"output_index":index,"item":value})
}
fn completed(output: Vec<Value>) -> Value {
    json!({"type":"response.completed","sequence_number":10,"response":{"output":output}})
}
async fn run(events: Vec<Value>) -> Vec<crate::error::Result<Vec<LanguageModelStreamChunk>>> {
    let server = MockServer::start().await;
    let body = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>();
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .expect(1)
        .mount(&server)
        .await;
    let mut model = OpenAI::<DynamicModel>::model_name("fixture");
    model.settings.base_url = server.uri();
    model.settings.api_key = "fixture".into();
    let mut stream = model
        .stream_text(LanguageModelOptions::default())
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        let failed = chunk.is_err();
        chunks.push(chunk);
        if failed {
            break;
        }
    }
    chunks
}
fn calls(
    chunks: &[crate::error::Result<Vec<LanguageModelStreamChunk>>],
) -> Vec<crate::core::ToolSearchCall> {
    chunks
        .iter()
        .flat_map(|c| c.as_ref().unwrap())
        .filter_map(|c| match c {
            LanguageModelStreamChunk::Done(m) => match &m.content {
                LanguageModelResponseContentType::ToolSearchCall(call) => Some(call.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
#[tokio::test]
async fn native_tool_search_stream_stream_deduplicates_completed_call() {
    let c = call("a", json!({"query":"files"}));
    let chunks = run(vec![
        item("added", 0, c.clone()),
        item("done", 0, c.clone()),
        item("done", 0, c.clone()),
        completed(vec![c]),
    ])
    .await;
    assert_eq!(calls(&chunks).len(), 1);
}
#[tokio::test]
async fn native_tool_search_stream_interleaved_arguments_are_assembled() {
    let mut events = vec![
        item("added", 0, call("a", json!(""))),
        item("added", 1, call("b", json!(""))),
    ];
    for (id, index, delta) in [
        ("a", 0, "{\"query\":"),
        ("b", 1, "{\"query\":\"other\"}"),
        ("a", 0, "\"files\"}"),
    ] {
        events.push(json!({"type":"response.function_call_arguments.delta","sequence_number":2,"item_id":format!("item-{id}"),"output_index":index,"delta":delta}));
    }
    events.push(json!({"type":"response.function_call_arguments.done","sequence_number":3,"item_id":"item-a","output_index":0,"arguments":"{\"query\":\"files\"}"}));
    events.extend([
        item("done", 0, call("a", json!({"query":"files"}))),
        item("done", 1, call("b", json!({"query":"other"}))),
        completed(vec![]),
    ]);
    let found = calls(&run(events).await);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].arguments, json!({"query":"files"}));
    assert_eq!(found[1].arguments, json!({"query":"other"}));
}
#[tokio::test]
async fn native_tool_search_stream_rejects_invalid_call() {
    for search in [call("", json!({})), call("a", json!("{")), {
        let mut c = call("a", json!({}));
        c["execution"] = json!("server");
        c
    }] {
        assert!(
            run(vec![item("done", 0, search), completed(vec![])])
                .await
                .iter()
                .any(Result::is_err)
        );
    }
}
#[tokio::test]
async fn native_tool_search_stream_unknown_native_type_is_observable() {
    assert!(
        run(vec![
            item(
                "done",
                0,
                json!({"type":"tool_search_future","arguments":{}})
            ),
            completed(vec![])
        ])
        .await
        .iter()
        .any(Result::is_err)
    );
}
#[tokio::test]
async fn native_tool_search_stream_terminal_failures_are_observable() {
    for terminal in [
        json!({"type":"error","sequence_number":2,"message":"cancelled"}),
        json!({"type":"response.incomplete","sequence_number":2,"response":{"incomplete_details":{"reason":"max_output_tokens"}}}),
        json!({"type":"response.failed","sequence_number":2,"response":{"error":{"code":"cancelled","message":"cancelled"}}}),
        json!({"type":"response.cancelled","sequence_number":2,"response":{}}),
    ] {
        let chunks = run(vec![item("added", 0, call("a", json!(""))), terminal]).await;
        assert!(chunks.iter().any(|c| c.is_err()
            || format!("{c:?}").contains("Failed")
            || format!("{c:?}").contains("Incomplete")));
        assert!(
            chunks
                .iter()
                .filter_map(|c| c.as_ref().ok())
                .flatten()
                .all(|c| !matches!(c, LanguageModelStreamChunk::Done(_)))
        );
    }
}
#[tokio::test]
async fn native_tool_search_stream_truncation_is_observable() {
    assert!(
        run(vec![item("added", 0, call("a", json!("")))])
            .await
            .iter()
            .any(Result::is_err)
    );
}
#[tokio::test]
async fn native_tool_search_stream_ordinary_function_and_usage_survive() {
    let f = json!({"type":"function_call","id":"fc","call_id":"f","name":"read","arguments":"{}"});
    let mut end = completed(vec![f.clone(), call("a", json!({}))]);
    end["response"]["usage"] = json!({"input_tokens":2,"input_tokens_details":{},"output_tokens":3,"output_tokens_details":{},"total_tokens":5});
    let chunks = run(vec![item("added", 0, f), end]).await;
    assert_eq!(calls(&chunks).len(), 1);
    assert!(chunks.iter().flat_map(|c|c.as_ref().unwrap()).any(|c|matches!(c,LanguageModelStreamChunk::Done(m) if matches!(m.content,LanguageModelResponseContentType::ToolCall(_)) && m.usage.is_some())));
}

#[tokio::test]
async fn native_tool_search_stream_rejects_conflicting_or_truncated_arguments() {
    for events in [
        vec![
            item("added", 0, call("a", json!(""))),
            json!({"type":"response.function_call_arguments.delta","sequence_number":2,"item_id":"item-a","output_index":0,"delta":"{"}),
            item("done", 0, call("a", json!({}))),
            completed(vec![]),
        ],
        vec![item("added", 0, call("a", json!(""))), completed(vec![])],
        vec![
            item("done", 0, call("a", json!({"query":"a"}))),
            completed(vec![call("a", json!({"query":"b"}))]),
        ],
        vec![
            item("added", 0, call("a", json!(""))),
            json!({"type":"response.function_call_arguments.delta","sequence_number":2,"item_id":"item-a","output_index":0,"delta":"{}"}),
            json!({"type":"response.function_call_arguments.done","sequence_number":3,"item_id":"item-a","output_index":0,"arguments":"{\"query\":\"other\"}"}),
            completed(vec![]),
        ],
    ] {
        let chunks = run(events).await;
        assert!(chunks.iter().any(Result::is_err));
        assert!(
            chunks
                .iter()
                .filter_map(|c| c.as_ref().ok())
                .flatten()
                .all(|c| !matches!(c, LanguageModelStreamChunk::Done(_)))
        );
    }
}

#[tokio::test]
async fn native_tool_search_stream_ordinary_completion_lifecycle_preserves_namespace() {
    use crate::core::language_model::LanguageModelStreamChunkType;
    let function = json!({"type":"function_call","id":"function-item","call_id":"function-call","name":"read","namespace":"files","arguments":"{}","status":"completed"});
    let text = json!({"type":"message","id":"text-item","role":"assistant","status":"completed","content":[{"type":"output_text","text":"finished","annotations":[],"logprobs":[]}]});
    let chunks = run(vec![
        item("added", 0, function.clone()),
        json!({"type":"response.function_call_arguments.delta","sequence_number":2,"item_id":"function-item","output_index":0,"delta":"{}"}),
        json!({"type":"response.function_call_arguments.done","sequence_number":3,"item_id":"function-item","output_index":0,"arguments":"{}"}),
        item("done", 0, function.clone()),
        item("added", 1, text.clone()),
        item("done", 1, text.clone()),
        completed(vec![function, text, call("search", json!({"query":"read"}))]),
    ]).await;
    let messages: Vec<_> = chunks.iter().flat_map(|c| c.as_ref().unwrap()).collect();
    assert!(
        !messages.iter().any(|c| matches!(
            c,
            LanguageModelStreamChunk::Delta(LanguageModelStreamChunkType::NotSupported(_))
        )),
        "ordinary completion events must be consumed"
    );
    let functions: Vec<_> = messages
        .iter()
        .filter_map(|c| match c {
            LanguageModelStreamChunk::Done(m) => match &m.content {
                LanguageModelResponseContentType::ToolCall(call) => Some(call),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].tool.id, "function-call");
    assert_eq!(functions[0].tool.namespace.as_deref(), Some("files"));
    assert_eq!(functions[0].input, json!({}));
    assert!(messages.iter().any(|c| matches!(c, LanguageModelStreamChunk::Done(m) if matches!(&m.content,LanguageModelResponseContentType::Text(text) if text == "finished"))));
    assert_eq!(calls(&chunks).len(), 1);
}
