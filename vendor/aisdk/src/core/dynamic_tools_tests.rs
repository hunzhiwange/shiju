use super::language_model::{LanguageModelOptions, LanguageModelResponseContentType, Step};
use super::messages::{AssistantMessage, Message, TaggedMessage};
use super::tools::{
    FunctionToolSpec, LoadableToolSpec, NamespaceToolSpec, NativeToolSpec, ToolCallInfo,
    ToolSearchCall, ToolSearchExecution, ToolSearchOutput,
};
use serde_json::json;

fn function() -> FunctionToolSpec {
    FunctionToolSpec {
        name: "read".into(),
        description: "Read an entry".into(),
        parameters: serde_json::from_value(json!({"type":"object","properties":{}})).unwrap(),
        strict: Some(false),
        defer_loading: Some(true),
    }
}
fn call() -> ToolSearchCall {
    ToolSearchCall {
        id: Some("tsc_1".into()),
        call_id: Some("call_1".into()),
        status: Some("completed".into()),
        execution: ToolSearchExecution::Client,
        arguments: json!({"query":"read entry","limit":2}),
        encrypted_content: Some("opaque".into()),
    }
}
fn output() -> ToolSearchOutput {
    ToolSearchOutput {
        id: Some("tso_1".into()),
        call_id: Some("call_1".into()),
        status: "completed".into(),
        execution: ToolSearchExecution::Client,
        tools: vec![LoadableToolSpec::Namespace(NamespaceToolSpec {
            name: "entries".into(),
            description: "Entry tools".into(),
            tools: vec![function()],
        })],
    }
}
#[test]
fn native_tool_search_core_typed_search_items_roundtrip() {
    let c = call();
    let o = output();
    assert_eq!(
        serde_json::from_value::<ToolSearchCall>(serde_json::to_value(&c).unwrap()).unwrap(),
        c
    );
    assert_eq!(
        serde_json::from_value::<ToolSearchOutput>(serde_json::to_value(&o).unwrap()).unwrap(),
        o
    );
    let options = LanguageModelOptions {
        native_tools: Some(vec![
            NativeToolSpec::Function(function()),
            NativeToolSpec::ToolSearch {
                execution: ToolSearchExecution::Client,
                description: "Find tools".into(),
                parameters: serde_json::from_value(json!({"type":"object"})).unwrap(),
            },
        ]),
        messages: vec![
            TaggedMessage::new(
                0,
                Message::Assistant(AssistantMessage::new(
                    LanguageModelResponseContentType::ToolSearchCall(c.clone()),
                    None,
                )),
            ),
            TaggedMessage::new(0, Message::ToolSearchOutput(o.clone())),
        ],
        ..Default::default()
    };
    assert_eq!(options.clone().native_tools, options.native_tools);
    assert!(
        matches!(&options.messages()[0], Message::Assistant(a) if matches!(&a.content, LanguageModelResponseContentType::ToolSearchCall(v) if v == &c))
    );
    assert!(matches!(&options.messages()[1], Message::ToolSearchOutput(v) if v == &o));
    let step = Step::new(0, options.messages());
    assert!(step.tool_calls().is_none());
    assert!(step.tool_results().is_none());
    let wire = serde_json::to_value(&o).unwrap();
    assert_eq!(wire["tools"][0]["type"], "namespace");
    assert_eq!(wire["tools"][0]["tools"][0]["type"], "function");
    assert_eq!(wire["tools"][0]["tools"][0]["defer_loading"], true);
}
#[test]
fn native_tool_search_core_plain_and_namespaced_calls_are_distinct() {
    let plain = ToolCallInfo::new("read");
    let mut qualified = plain.clone();
    qualified.tool.namespace = Some("entries".into());
    assert_ne!(plain, qualified);
    assert_eq!(qualified.clone().tool.namespace.as_deref(), Some("entries"));
    assert!(plain.tool.namespace.is_none());
    let step = Step::new(
        0,
        vec![Message::Assistant(AssistantMessage::new(
            LanguageModelResponseContentType::ToolCall(plain.clone()),
            None,
        ))],
    );
    assert_eq!(step.tool_calls().unwrap(), vec![plain]);
}
#[test]
fn native_tool_search_core_rejects_nonloadable_and_unknown_tools() {
    for value in [
        json!({"type":"custom","name":"x"}),
        json!({"type":"tool_search","execution":"client","description":"x","parameters":{}}),
        json!({"type":"namespace","name":"x","description":"x","tools":[{"type":"tool_search"}]}),
    ] {
        assert!(serde_json::from_value::<LoadableToolSpec>(value).is_err());
    }
    assert!(serde_json::from_value::<ToolSearchExecution>(json!("unknown")).is_err());
}
#[cfg(all(
    feature = "openai",
    feature = "openaicompatible",
    feature = "anthropic",
    feature = "google"
))]
#[tokio::test]
async fn native_tool_search_core_providers_reject_before_network() {
    use super::{DynamicModel, LanguageModel};
    use crate::providers::{
        anthropic::Anthropic, google::Google, openai_chat_completions::OpenAIChatCompletions,
        openai_compatible::OpenAICompatible,
    };
    async fn rejects<M: LanguageModel>(mut model: M) {
        let mut qualified = ToolCallInfo::new("read");
        qualified.tool.namespace = Some("entries".into());
        for options in [
            LanguageModelOptions {
                native_tools: Some(vec![NativeToolSpec::Function(function())]),
                ..Default::default()
            },
            LanguageModelOptions {
                messages: vec![TaggedMessage::new(
                    0,
                    Message::Assistant(AssistantMessage::new(
                        LanguageModelResponseContentType::ToolSearchCall(call()),
                        None,
                    )),
                )],
                ..Default::default()
            },
            LanguageModelOptions {
                messages: vec![TaggedMessage::new(0, Message::ToolSearchOutput(output()))],
                ..Default::default()
            },
            LanguageModelOptions {
                messages: vec![TaggedMessage::new(
                    0,
                    Message::Assistant(AssistantMessage::new(
                        LanguageModelResponseContentType::ToolSearchOutput(output()),
                        None,
                    )),
                )],
                ..Default::default()
            },
            LanguageModelOptions {
                messages: vec![TaggedMessage::new(
                    0,
                    Message::Tool(super::tools::ToolResultInfo {
                        tool: qualified.tool.clone(),
                        output: Ok(json!("result")),
                    }),
                )],
                ..Default::default()
            },
            LanguageModelOptions {
                messages: vec![TaggedMessage::new(
                    0,
                    Message::Assistant(AssistantMessage::new(
                        LanguageModelResponseContentType::ToolCall(qualified),
                        None,
                    )),
                )],
                ..Default::default()
            },
        ] {
            let error = model.generate_text(options.clone()).await.unwrap_err();
            assert!(
                matches!(error,crate::Error::InvalidInput(ref s) if s.contains("native tool protocol")),
                "{error}"
            );
            let error = match model.stream_text(options).await {
                Ok(_) => panic!("unsupported native stream accepted"),
                Err(e) => e,
            };
            assert!(
                matches!(error,crate::Error::InvalidInput(ref s) if s.contains("native tool protocol")),
                "{error}"
            );
        }
    }
    let mut chat = OpenAIChatCompletions::<DynamicModel>::model_name("fixture");
    chat.settings.base_url = "http://127.0.0.1:1".into();
    chat.settings.api_key = "fixture".into();
    rejects(chat).await;
    rejects(
        Anthropic::<DynamicModel>::builder()
            .model_name("fixture")
            .api_key("fixture")
            .base_url("http://127.0.0.1:1")
            .build()
            .unwrap(),
    )
    .await;
    rejects(
        Google::<DynamicModel>::builder()
            .model_name("fixture")
            .api_key("fixture")
            .base_url("http://127.0.0.1:1")
            .build()
            .unwrap(),
    )
    .await;
    rejects(
        OpenAICompatible::<DynamicModel>::builder()
            .model_name("fixture")
            .api_key("fixture")
            .base_url("http://127.0.0.1:1")
            .build()
            .unwrap(),
    )
    .await;
}

#[test]
fn native_tool_search_core_declarations_and_server_items_preserve_optional_fields() {
    let declarations = vec![
        NativeToolSpec::Function(function()),
        NativeToolSpec::Namespace(NamespaceToolSpec {
            name: "entries".into(),
            description: "Entry tools".into(),
            tools: vec![function()],
        }),
        NativeToolSpec::ToolSearch {
            execution: ToolSearchExecution::Client,
            description: "Find entry tools".into(),
            parameters: serde_json::from_value(json!({"type":"object"})).unwrap(),
        },
    ];
    let wire = serde_json::to_value(&declarations).unwrap();
    assert_eq!(
        serde_json::from_value::<Vec<NativeToolSpec>>(wire.clone()).unwrap(),
        declarations
    );
    assert_eq!(wire[2]["type"], "tool_search");
    assert_eq!(wire[2]["execution"], "client");
    assert_eq!(wire[0]["strict"], false);
    let mut c = call();
    c.execution = ToolSearchExecution::Server;
    c.id = None;
    c.call_id = None;
    c.status = None;
    c.encrypted_content = None;
    assert_eq!(
        serde_json::from_value::<ToolSearchCall>(serde_json::to_value(&c).unwrap()).unwrap(),
        c
    );
    let mut o = output();
    o.execution = ToolSearchExecution::Server;
    o.call_id = None;
    o.tools.clear();
    assert_eq!(
        serde_json::from_value::<ToolSearchOutput>(serde_json::to_value(&o).unwrap()).unwrap(),
        o
    );
    let message = Message::Assistant(AssistantMessage::new(
        LanguageModelResponseContentType::ToolSearchOutput(o.clone()),
        None,
    ));
    assert!(
        matches!(message.clone(), Message::Assistant(a) if matches!(&a.content, LanguageModelResponseContentType::ToolSearchOutput(v) if v == &o))
    );
    assert!(Step::new(0, vec![message]).tool_calls().is_none());
}

#[tokio::test]
async fn native_tool_search_core_namespaced_call_cannot_execute_same_named_local_tool() {
    use super::tools::{Tool, ToolContext, ToolExecute, ToolList};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let executions = Arc::new(AtomicUsize::new(0));
    let counter = executions.clone();
    let tools = ToolList::new(vec![Tool {
        name: "read".into(),
        description: "local read".into(),
        input_schema: function().parameters,
        execute: ToolExecute::from_sync(move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok("local result".into())
        }),
    }]);
    let mut qualified = ToolCallInfo::new("read");
    qualified.tool.namespace = Some("entries".into());
    let context = ToolContext::new(LanguageModelOptions::default());
    let error = tools.execute(context.clone(), qualified).await.unwrap_err();
    assert!(matches!(error, crate::Error::ToolCallError(ref s) if s.contains("external executor")));
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        tools
            .execute(context, ToolCallInfo::new("read"))
            .await
            .unwrap(),
        "local result"
    );
    assert_eq!(executions.load(Ordering::SeqCst), 1);
}
