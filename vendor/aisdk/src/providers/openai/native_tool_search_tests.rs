use super::client::types::ToolParams;
use crate::core::language_model::LanguageModelOptions;
use serde_json::json;

#[test]
fn native_tool_search_wire_wire_preserves_namespace_and_search_output() {
    let wire = json!({"type":"tool_search","execution":"client","description":"find","parameters":{"type":"object"}});
    let tool: ToolParams = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(tool).unwrap(), wire);
    native_tool_search_wire_namespace_and_output_remain_structured();
}

use super::client::{OpenAILanguageModelOptions, types::InputItem};
use crate::core::messages::Message;
use crate::core::{
    FunctionToolSpec, LoadableToolSpec, NamespaceToolSpec, NativeToolSpec, ToolSearchExecution,
    ToolSearchOutput,
};

fn function() -> FunctionToolSpec {
    FunctionToolSpec {
        name: "read".into(),
        description: "read".into(),
        parameters: serde_json::from_value(json!({"type":"object"})).unwrap(),
        strict: Some(true),
        defer_loading: Some(true),
    }
}

#[test]
fn native_tool_search_wire_namespace_and_output_remain_structured() {
    let ns = NamespaceToolSpec {
        name: "files".into(),
        description: "files".into(),
        tools: vec![function()],
    };
    let options = LanguageModelOptions {
        native_tools: Some(vec![NativeToolSpec::Namespace(ns.clone())]),
        ..Default::default()
    };
    let request: OpenAILanguageModelOptions = options.try_into().unwrap();
    let tools = serde_json::to_value(request.tools.unwrap()).unwrap();
    assert_eq!(tools[0]["type"], "namespace");
    assert_eq!(tools[0]["tools"][0]["defer_loading"], true);
    assert_eq!(
        tools[0]["tools"][0]["parameters"]["additionalProperties"],
        false
    );
    let output = ToolSearchOutput {
        id: None,
        call_id: Some("search-1".into()),
        status: "completed".into(),
        execution: ToolSearchExecution::Client,
        tools: vec![
            LoadableToolSpec::Namespace(ns.clone()),
            LoadableToolSpec::Namespace(ns),
        ],
    };
    let item: Option<InputItem> = Message::ToolSearchOutput(output).try_into().unwrap();
    let wire = serde_json::to_value(item.unwrap()).unwrap();
    assert_eq!(wire["type"], "tool_search_output");
    assert_eq!(wire["call_id"], "search-1");
    assert_eq!(wire["tools"].as_array().unwrap().len(), 1);
    assert_eq!(wire["tools"][0]["tools"].as_array().unwrap().len(), 2);
    assert!(wire.get("output").is_none());
    let decoded: super::client::types::MessageItem = serde_json::from_value(wire).unwrap();
    assert!(matches!(
        decoded,
        super::client::types::MessageItem::NativeSearch(_)
    ));
}

#[test]
fn native_tool_search_wire_rejects_missing_client_call_id_and_server_declaration() {
    let output = ToolSearchOutput {
        id: None,
        call_id: None,
        status: "completed".into(),
        execution: ToolSearchExecution::Client,
        tools: vec![],
    };
    assert!(Option::<InputItem>::try_from(Message::ToolSearchOutput(output)).is_err());
    let options = LanguageModelOptions {
        native_tools: Some(vec![NativeToolSpec::ToolSearch {
            execution: ToolSearchExecution::Server,
            description: "find".into(),
            parameters: serde_json::from_value(json!({"type":"object"})).unwrap(),
        }]),
        ..Default::default()
    };
    assert!(OpenAILanguageModelOptions::try_from(options).is_err());
    assert!(serde_json::from_value::<ToolParams>(json!({"type":"namespace","name":"bad","description":"bad","tools":[{"type":"tool_search"}]})).is_err());
}

#[test]
fn native_tool_search_wire_call_and_namespaced_function_preserve_identity() {
    use crate::core::language_model::LanguageModelResponseContentType;
    use crate::core::messages::AssistantMessage;
    use crate::core::{ToolCallInfo, ToolSearchCall};
    let call = ToolSearchCall {
        id: Some("tsc-1".into()),
        call_id: Some("call-1".into()),
        status: None,
        execution: ToolSearchExecution::Client,
        arguments: json!({"query":"read"}),
        encrypted_content: Some("opaque".into()),
    };
    let message = Message::Assistant(AssistantMessage::new(
        LanguageModelResponseContentType::ToolSearchCall(call),
        None,
    ));
    let item: Option<InputItem> = message.try_into().unwrap();
    let wire = serde_json::to_value(item.unwrap()).unwrap();
    assert_eq!(wire["type"], "tool_search_call");
    assert_eq!(wire["execution"], "client");
    assert_eq!(wire["call_id"], "call-1");
    assert_eq!(wire["arguments"], json!({"query":"read"}));
    assert_eq!(wire["encrypted_content"], "opaque");
    assert!(matches!(
        serde_json::from_value::<super::client::types::MessageItem>(wire).unwrap(),
        super::client::types::MessageItem::NativeSearch(_)
    ));
    for namespace in [None, Some("files".to_string())] {
        let mut call = ToolCallInfo::new("read");
        call.tool.namespace = namespace.clone();
        let item: Option<InputItem> = Message::Assistant(AssistantMessage::new(
            LanguageModelResponseContentType::ToolCall(call),
            None,
        ))
        .try_into()
        .unwrap();
        let wire = serde_json::to_value(item.unwrap()).unwrap();
        assert_eq!(wire["type"], "function_call");
        assert_eq!(wire["name"], "read");
        assert_eq!(
            wire.get("namespace").and_then(|v| v.as_str()),
            namespace.as_deref()
        );
    }
}

#[test]
fn native_tool_search_wire_rejects_boolean_function_schema() {
    let mut f = function();
    f.parameters = serde_json::from_value(json!(true)).unwrap();
    let options = LanguageModelOptions {
        native_tools: Some(vec![NativeToolSpec::Function(f)]),
        ..Default::default()
    };
    assert!(OpenAILanguageModelOptions::try_from(options).is_err());
}
