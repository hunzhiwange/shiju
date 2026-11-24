//! Helper functions and conversions for the OpenAI provider.

use crate::core::embedding_model::EmbeddingModelOptions;
use crate::core::language_model::{
    LanguageModelOptions, LanguageModelResponseContentType, ReasoningEffort, Usage,
};
use crate::core::messages::{Message, UserContentPart, UserImageDetail};
use crate::core::tools::Tool;
use crate::providers::openai::client::{self, types};
use schemars::Schema;
use serde_json::Value;

impl From<UserImageDetail> for types::ImageDetail {
    fn from(value: UserImageDetail) -> Self {
        match value {
            UserImageDetail::Auto => types::ImageDetail::Auto,
            UserImageDetail::Low => types::ImageDetail::Low,
            UserImageDetail::High => types::ImageDetail::High,
        }
    }
}

impl From<UserContentPart> for types::ContentType {
    fn from(value: UserContentPart) -> Self {
        match value {
            UserContentPart::Text(text) => types::ContentType::InputText { text },
            UserContentPart::Image(image) => types::ContentType::InputImage {
                detail: image.detail.into(),
                file_id: None,
                image_url: Some(image.image_url),
            },
        }
    }
}

impl From<Tool> for types::ToolParams {
    fn from(value: Tool) -> Self {
        let mut params = value.input_schema.to_value();

        // open ai requires 'additionalProperties' to be false
        params["additionalProperties"] = Value::Bool(false);

        // open ai requires 'properties' to be an object
        let properties = params.get("properties");
        if let Some(Value::Object(_)) = properties {
        } else {
            params["properties"] = Value::Object(serde_json::Map::new());
        }

        types::ToolParams::Function {
            name: value.name,
            description: Some(value.description),
            strict: true,
            defer_loading: None,
            parameters: params,
        }
    }
}

impl TryFrom<LanguageModelOptions> for client::OpenAILanguageModelOptions {
    type Error = crate::Error;

    fn try_from(options: LanguageModelOptions) -> crate::Result<Self> {
        let extra_body = options.body.clone();
        let extra_headers = options.headers.clone();
        let items: Vec<types::InputItem> = options
            .messages
            .into_iter()
            .map(|m| m.message.try_into())
            .collect::<crate::Result<Vec<Option<types::InputItem>>>>()?
            .into_iter()
            .flatten()
            .collect();

        let mut tools: Option<Vec<types::ToolParams>> = options.tools.map(|t| {
            t.tools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .map(|t| types::ToolParams::from(t.clone()))
                .collect()
        });

        if let Some(native) = options.native_tools {
            let target = tools.get_or_insert_with(Vec::new);
            for tool in native {
                target.push(native_tool(tool)?);
            }
        }

        let reasoning = options
            .reasoning_effort
            .map(|reasoning| types::ReasoningConfig {
                summary: Some(types::SummaryType::Auto),
                effort: Some(reasoning.into()),
            });

        Ok(client::OpenAILanguageModelOptions {
            model: "".to_string(), // will be set in mod.rs
            input: Some(types::Input::InputItemList(items)),
            text: Some(types::TextConfig {
                verbosity: None,
                format: Some(
                    options
                        .schema
                        .map(from_schema_to_response_format)
                        .unwrap_or(types::TextResponseFormat::Text),
                ),
            }),
            reasoning,
            temperature: options.temperature.map(|t| t as f32 / 100.0),
            max_output_tokens: options.max_output_tokens.map(|t| t as usize),
            stream: Some(false),
            top_p: options.top_p.map(|t| t as f32 / 100.0),
            tools,
            extra_body,
            extra_headers,
        })
    }
}

impl TryFrom<Message> for Option<types::InputItem> {
    type Error = crate::Error;

    fn try_from(m: Message) -> crate::Result<Self> {
        Ok(match m {
            Message::Tool(ref tool_info) => Some(types::InputItem::Item(
                types::MessageItem::FunctionCallOutput {
                    id: None,
                    type_: "function_call_output".to_string(),
                    status: None,
                    call_id: tool_info.tool.id.clone(),
                    output: types::FunctionCallOutput::Text(
                        tool_info
                            .output
                            .clone()
                            .unwrap_or_else(|e| Value::String(e.to_string()))
                            .to_string(),
                    ),
                },
            )),
            Message::Assistant(ref assistant_msg) => match assistant_msg.content {
                LanguageModelResponseContentType::Text(ref msg) => {
                    Some(types::InputItem::Item(types::MessageItem::OutputMessage {
                        id: None,
                        type_: "message".to_string(),
                        status: None,
                        role: types::Role::Assistant,
                        content: vec![types::OutputContent::OutputText {
                            annotations: vec![],
                            logprobs: vec![],
                            text: msg.to_owned(),
                        }],
                    }))
                }
                LanguageModelResponseContentType::ToolCall(ref tool_info) => {
                    Some(types::InputItem::Item(types::MessageItem::FunctionCall {
                        id: None,
                        status: None,
                        arguments: tool_info.input.to_string(),
                        call_id: tool_info.tool.id.clone(),
                        name: tool_info.tool.name.clone(),
                        namespace: tool_info.tool.namespace.clone(),
                        type_: "function_call".to_string(),
                    }))
                }
                LanguageModelResponseContentType::Reasoning { ref content, .. } => {
                    Some(types::InputItem::Item(types::MessageItem::Reasoning {
                        id: None,
                        summary: vec![types::ReasoningSummary {
                            type_: "summary_text".to_string(),
                            text: content.clone(),
                        }],
                        type_: "reasoning".to_string(),
                        content: None,
                        encrypted_content: None,
                        status: None,
                    }))
                }
                LanguageModelResponseContentType::ToolSearchCall(ref call) => {
                    validate_search(call.execution, &call.call_id)?;
                    Some(types::InputItem::Item(types::MessageItem::NativeSearch(
                        types::NativeSearchItem::ToolSearchCall(call.clone()),
                    )))
                }
                LanguageModelResponseContentType::ToolSearchOutput(ref output) => {
                    Some(search_output(output.clone())?)
                }
                _ => None,
            },
            Message::User(u) => Some(types::InputItem::Item(types::MessageItem::InputMessage {
                content: u.content.into_iter().map(Into::into).collect(),
                role: types::Role::User,
                type_: "message".to_string(),
            })),
            Message::System(s) => Some(types::InputItem::Item(types::MessageItem::InputMessage {
                content: vec![types::ContentType::InputText { text: s.content }],
                role: types::Role::System,
                type_: "message".to_string(),
            })),
            Message::ToolSearchOutput(output) => Some(search_output(output)?),
            Message::Developer(d) => {
                Some(client::InputItem::Item(types::MessageItem::InputMessage {
                    content: vec![types::ContentType::InputText { text: d }],
                    role: types::Role::Developer,
                    type_: "message".to_string(),
                }))
            }
        })
    }
}

impl From<types::ResponseUsage> for Usage {
    fn from(value: types::ResponseUsage) -> Self {
        Self {
            input_tokens: Some(value.input_tokens as usize),
            output_tokens: Some(value.output_tokens as usize),
            cached_tokens: Some(value.input_tokens_details.cached_tokens as usize),
            reasoning_tokens: Some(value.output_tokens_details.reasoning_tokens as usize),
        }
    }
}

impl From<ReasoningEffort> for types::ReasoningEffort {
    fn from(value: ReasoningEffort) -> Self {
        match value {
            ReasoningEffort::None => client::ReasoningEffort::None,
            ReasoningEffort::Minimal => client::ReasoningEffort::Minimal,
            ReasoningEffort::Low => client::ReasoningEffort::Low,
            ReasoningEffort::Xhigh => client::ReasoningEffort::XHigh,
            ReasoningEffort::Max => client::ReasoningEffort::Max,
            ReasoningEffort::Medium => client::ReasoningEffort::Medium,
            ReasoningEffort::High => client::ReasoningEffort::High,
        }
    }
}

impl From<EmbeddingModelOptions> for types::OpenAIEmbeddingOptions {
    fn from(value: EmbeddingModelOptions) -> Self {
        let extra_body = value.body.clone();
        let extra_headers = value.headers.clone();
        types::OpenAIEmbeddingOptions {
            input: value.input,
            model: "".to_string(), // will be set in mod.rs
            user: None,
            dimensions: value.dimensions,
            encoding_format: None,
            extra_body,
            extra_headers,
        }
    }
}

fn from_schema_to_response_format(schema: Schema) -> types::TextResponseFormat {
    let json = serde_json::to_value(schema).expect("Failed to serialize schema");
    types::TextResponseFormat::JsonSchema {
        name: json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Response Schema")
            .to_owned(),
        description: json
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        schema: json,
        strict: Some(false),
    }
}

fn validate_search(
    execution: crate::core::tools::ToolSearchExecution,
    call_id: &Option<String>,
) -> crate::Result<()> {
    if execution == crate::core::tools::ToolSearchExecution::Client
        && call_id.as_ref().is_none_or(|id| id.trim().is_empty())
    {
        return Err(crate::Error::InvalidInput(
            "client tool_search requires call_id".into(),
        ));
    }
    Ok(())
}

fn normalize_function(function: &mut crate::core::tools::FunctionToolSpec) -> crate::Result<()> {
    let mut schema = function.parameters.clone().to_value();
    if !schema.is_object() {
        return Err(crate::Error::InvalidInput(
            "native function schema must be an object".into(),
        ));
    }
    schema["additionalProperties"] = Value::Bool(false);
    if !schema.get("properties").is_some_and(Value::is_object) {
        schema["properties"] = serde_json::json!({});
    }
    function.parameters =
        Schema::try_from(schema).map_err(|e| crate::Error::InvalidInput(e.to_string()))?;
    Ok(())
}

fn native_tool(tool: crate::core::tools::NativeToolSpec) -> crate::Result<types::ToolParams> {
    use crate::core::tools::NativeToolSpec;
    Ok(match tool {
        NativeToolSpec::Function(mut f) => {
            normalize_function(&mut f)?;
            types::ToolParams::Function {
                name: f.name,
                description: Some(f.description),
                parameters: f.parameters.to_value(),
                strict: f.strict.unwrap_or(true),
                defer_loading: f.defer_loading,
            }
        }
        NativeToolSpec::Namespace(mut namespace) => {
            for f in &mut namespace.tools {
                normalize_function(f)?;
            }
            types::ToolParams::Namespace(namespace)
        }
        NativeToolSpec::ToolSearch {
            execution,
            description,
            parameters,
        } => {
            if execution != crate::core::tools::ToolSearchExecution::Client {
                return Err(crate::Error::InvalidInput(
                    "only client tool_search declarations are supported".into(),
                ));
            }
            types::ToolParams::ToolSearch {
                execution,
                description,
                parameters: parameters.to_value(),
            }
        }
    })
}

fn search_output(
    mut output: crate::core::tools::ToolSearchOutput,
) -> crate::Result<types::InputItem> {
    use crate::core::tools::LoadableToolSpec;
    validate_search(output.execution, &output.call_id)?;
    let mut merged: Vec<crate::core::tools::LoadableToolSpec> = Vec::new();
    for mut tool in output.tools {
        match &mut tool {
            LoadableToolSpec::Function(f) => normalize_function(f)?,
            LoadableToolSpec::Namespace(ns) => {
                for f in &mut ns.tools {
                    normalize_function(f)?;
                }
                if let Some(LoadableToolSpec::Namespace(existing)) = merged
                    .iter_mut()
                    .find(|t| matches!(t, LoadableToolSpec::Namespace(n) if n.name == ns.name))
                {
                    existing.tools.append(&mut ns.tools);
                    continue;
                }
            }
        }
        merged.push(tool);
    }
    output.tools = merged;
    Ok(types::InputItem::Item(types::MessageItem::NativeSearch(
        types::NativeSearchItem::ToolSearchOutput(output),
    )))
}

#[cfg(test)]
mod tests {
    use super::client::*;
    use crate::core::Message;
    use crate::core::language_model::{
        LanguageModelOptions, ReasoningEffort as LMReasoningEffort, Usage,
    };
    use crate::core::tools::{Tool, ToolExecute, ToolList};
    use crate::core::{UserImage, UserImageDetail, UserMessage};
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct StructuredOutput {
        answer: String,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct SumInput {
        a: i32,
        b: i32,
    }

    #[test]
    fn test_scalar_request_options_map_to_openai_body() {
        let options = LanguageModelOptions {
            messages: vec![
                Message::System("You are helpful".to_string().into()).into(),
                Message::User("Hello".to_string().into()).into(),
            ],
            temperature: Some(70),
            top_p: Some(90),
            max_output_tokens: Some(256),
            ..Default::default()
        };

        let req: OpenAILanguageModelOptions = options.try_into().unwrap();

        assert!(
            req.temperature
                .is_some_and(|value| (value - 0.7).abs() < f32::EPSILON)
        );
        assert!(
            req.top_p
                .is_some_and(|value| (value - 0.9).abs() < f32::EPSILON)
        );
        assert_eq!(req.max_output_tokens, Some(256));
        assert_eq!(req.stream, Some(false));

        let input = req.input.expect("input should be present");
        let Input::InputItemList(items) = input else {
            panic!("expected input item list")
        };
        assert_eq!(items.len(), 2);

        match &items[0] {
            InputItem::Item(MessageItem::InputMessage { role, content, .. }) => {
                assert_eq!(role, &Role::System);
                assert!(matches!(
                    content.first(),
                    Some(ContentType::InputText { text }) if text == "You are helpful"
                ));
            }
            _ => panic!("expected first item to be a system input message"),
        }

        match &items[1] {
            InputItem::Item(MessageItem::InputMessage { role, content, .. }) => {
                assert_eq!(role, &Role::User);
                assert!(
                    matches!(content.first(), Some(ContentType::InputText { text }) if text == "Hello")
                );
            }
            _ => panic!("expected second item to be a user input message"),
        }
    }

    #[test]
    fn test_schema_maps_to_openai_body() {
        let options = LanguageModelOptions {
            schema: Some(schema_for!(StructuredOutput)),
            ..Default::default()
        };

        let req: OpenAILanguageModelOptions = options.try_into().unwrap();
        let text = req.text.expect("text config should be present");
        let format = text.format.expect("format should be present");

        let TextResponseFormat::JsonSchema {
            name,
            schema,
            strict,
            ..
        } = format
        else {
            panic!("expected json_schema response format")
        };

        assert_eq!(name, "StructuredOutput");
        assert_eq!(strict, Some(false));
        assert_eq!(schema["type"], json!("object"));
        assert_eq!(schema["properties"]["answer"]["type"], json!("string"));
    }

    #[test]
    fn test_multimodal_user_message_maps_to_openai_body() {
        let options = LanguageModelOptions {
            messages: vec![
                Message::User(UserMessage::new("Describe this image").with_image(
                    UserImage::new("https://example.com/cat.png").detail(UserImageDetail::High),
                ))
                .into(),
            ],
            ..Default::default()
        };

        let req: OpenAILanguageModelOptions = options.try_into().unwrap();
        let input = req.input.expect("input should be present");
        let Input::InputItemList(items) = input else {
            panic!("expected input item list")
        };

        match &items[0] {
            InputItem::Item(MessageItem::InputMessage { role, content, .. }) => {
                assert_eq!(role, &Role::User);
                assert!(matches!(
                    content.first(),
                    Some(ContentType::InputText { text }) if text == "Describe this image"
                ));
                assert!(matches!(
                    content.get(1),
                    Some(ContentType::InputImage { detail: ImageDetail::High, image_url: Some(url), file_id: None })
                        if url == "https://example.com/cat.png"
                ));
            }
            _ => panic!("expected a multimodal user input message"),
        }
    }

    #[test]
    fn test_tools_map_to_openai_body() {
        let tool = Tool::builder()
            .name("sum")
            .description("Adds two numbers")
            .input_schema(schema_for!(SumInput))
            .execute(ToolExecute::from_sync(|_, _| Ok("3".to_string())))
            .build()
            .expect("tool should build");

        let options = LanguageModelOptions {
            tools: Some(ToolList::new(vec![tool])),
            ..Default::default()
        };

        let req: OpenAILanguageModelOptions = options.try_into().unwrap();
        let tools = req.tools.expect("tools should be present");
        assert_eq!(tools.len(), 1);

        match &tools[0] {
            ToolParams::Function {
                name,
                description,
                strict,
                parameters,
                ..
            } => {
                assert_eq!(name, "sum");
                assert_eq!(description.as_deref(), Some("Adds two numbers"));
                assert!(*strict);
                assert_eq!(parameters["type"], json!("object"));
                assert_eq!(parameters["additionalProperties"], json!(false));
                assert!(parameters["properties"].get("a").is_some());
                assert!(parameters["properties"].get("b").is_some());
            }
            _ => panic!("expected ordinary function"),
        }
    }

    #[test]
    fn test_reasoning_effort_conversion_low() {
        let effort = LMReasoningEffort::Low;
        let openai_effort: ReasoningEffort = effort.into();
        assert_eq!(openai_effort, ReasoningEffort::Low);
        let _ = openai_effort;
    }

    #[test]
    fn test_reasoning_effort_conversion_medium() {
        let effort = LMReasoningEffort::Medium;
        let openai_effort: ReasoningEffort = effort.into();
        assert_eq!(openai_effort, ReasoningEffort::Medium);
    }

    #[test]
    fn test_reasoning_effort_conversion_high() {
        let effort = ReasoningEffort::High;
        let openai_effort: ReasoningEffort = effort;
        assert_eq!(openai_effort, ReasoningEffort::High);
    }

    #[test]
    fn test_language_model_options_to_create_response_with_reasoning_effort_low() {
        let options = LanguageModelOptions {
            reasoning_effort: Some(LMReasoningEffort::Low),
            ..Default::default()
        };
        let lm_options: OpenAILanguageModelOptions = options.try_into().unwrap();
        assert!(lm_options.reasoning.is_some());
        let reasoning = lm_options.reasoning.unwrap();
        assert_eq!(reasoning.effort, Some(ReasoningEffort::Low));
        assert_eq!(reasoning.summary, Some(SummaryType::Auto));
    }

    #[test]
    fn test_language_model_options_to_create_response_with_reasoning_effort_medium() {
        let options = LanguageModelOptions {
            reasoning_effort: Some(LMReasoningEffort::Medium),
            ..Default::default()
        };
        let lm_options: OpenAILanguageModelOptions = options.try_into().unwrap();
        assert!(lm_options.reasoning.is_some());
        let reasoning = lm_options.reasoning.unwrap();
        assert_eq!(reasoning.effort, Some(ReasoningEffort::Medium));
        assert_eq!(reasoning.summary, Some(SummaryType::Auto));
    }

    #[test]
    fn test_language_model_options_to_create_response_with_reasoning_effort_high() {
        let options = LanguageModelOptions {
            reasoning_effort: Some(LMReasoningEffort::High),
            ..Default::default()
        };
        let lm_options: OpenAILanguageModelOptions = options.try_into().unwrap();
        assert!(lm_options.reasoning.is_some());
        let reasoning = lm_options.reasoning.unwrap();
        assert_eq!(reasoning.effort, Some(ReasoningEffort::High));
        assert_eq!(reasoning.summary, Some(SummaryType::Auto));
    }

    #[test]
    fn test_language_model_options_to_create_response_without_reasoning_effort() {
        let options = LanguageModelOptions {
            reasoning_effort: None,
            ..Default::default()
        };
        let lm_options: OpenAILanguageModelOptions = options.try_into().unwrap();
        assert!(lm_options.reasoning.is_none());
    }

    #[test]
    fn test_openai_usage_to_usage_conversion() {
        let openai_usage = types::ResponseUsage {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            input_tokens_details: Default::default(),
            output_tokens_details: Default::default(),
        };

        let usage: Usage = openai_usage.into();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        // These will be 0 because the details are default (None)
        assert_eq!(usage.cached_tokens, Some(0));
        assert_eq!(usage.reasoning_tokens, Some(0));
    }
}

/// Native searches are released only after successful response completion. Item
/// done and the final snapshot describe the same call, rather than two calls.
#[derive(Default)]
pub(super) struct NativeSearchStream {
    calls: std::collections::BTreeMap<String, crate::core::tools::ToolSearchCall>,
    items: std::collections::HashMap<String, String>,
    arguments: std::collections::HashMap<String, String>,
    completed: std::collections::HashSet<String>,
}

impl NativeSearchStream {
    fn remember(
        &mut self,
        call: &crate::core::tools::ToolSearchCall,
        done: bool,
    ) -> crate::Result<()> {
        validate_search(call.execution, &call.call_id)?;
        if call.execution != crate::core::tools::ToolSearchExecution::Client {
            return Err(crate::Error::InvalidInput(
                "server tool_search is unsupported".into(),
            ));
        }
        let key = call.call_id.clone().expect("validated client call_id");
        if let Some(id) = &call.id {
            if self.items.get(id).is_some_and(|previous| previous != &key) {
                return Err(crate::Error::InvalidInput(
                    "tool_search item changed call_id".into(),
                ));
            }
            self.items.insert(id.clone(), key.clone());
        }
        let mut call = call.clone();
        if done {
            if call
                .status
                .as_deref()
                .is_some_and(|status| status != "completed")
            {
                return Err(crate::Error::InvalidInput(
                    "tool_search did not complete".into(),
                ));
            }
            if let Value::String(arguments) = &call.arguments {
                call.arguments = serde_json::from_str(arguments).map_err(|_| {
                    crate::Error::InvalidInput("Invalid tool_search arguments".into())
                })?;
            }
            if !call.arguments.is_object() {
                return Err(crate::Error::InvalidInput(
                    "tool_search arguments must be an object".into(),
                ));
            }
            if let Some(arguments) = self.arguments.get(&key) {
                let assembled: Value = serde_json::from_str(arguments).map_err(|_| {
                    crate::Error::InvalidInput("Truncated tool_search arguments".into())
                })?;
                if assembled != call.arguments {
                    return Err(crate::Error::InvalidInput(
                        "tool_search arguments disagree with completed item".into(),
                    ));
                }
            }
            if self.completed.contains(&key) {
                if self.calls.get(&key).is_some_and(|old| {
                    old.arguments != call.arguments || old.execution != call.execution
                }) {
                    return Err(crate::Error::InvalidInput(
                        "Conflicting completed tool_search call".into(),
                    ));
                }
                return Ok(());
            }
            self.completed.insert(key.clone());
        } else if self.completed.contains(&key) {
            return Ok(());
        }
        self.calls.insert(key, call);
        Ok(())
    }

    /// Return true when the native event is consumed here rather than exposed as
    /// an ordinary function-call delta.
    pub(super) fn observe(&mut self, event: &types::OpenAiStreamEvent) -> crate::Result<bool> {
        use types::{MessageItem, NativeSearchItem, OpenAiStreamEvent as Event};
        match event {
            Event::ResponseOutputItemAdded {
                item: MessageItem::NativeSearch(NativeSearchItem::ToolSearchCall(call)),
                ..
            } => {
                self.remember(call, false)?;
                Ok(true)
            }
            Event::ResponseOutputItemDone {
                item: MessageItem::NativeSearch(NativeSearchItem::ToolSearchCall(call)),
                ..
            } => {
                self.remember(call, true)?;
                Ok(true)
            }
            Event::ResponseFunctionCallArgumentsDelta { item_id, delta, .. }
                if self.items.contains_key(item_id) =>
            {
                let key = &self.items[item_id];
                if !self.completed.contains(key) {
                    self.arguments
                        .entry(key.clone())
                        .or_default()
                        .push_str(delta);
                }
                Ok(true)
            }
            Event::ResponseFunctionCallArgumentsDone {
                item_id, arguments, ..
            } if self.items.contains_key(item_id) => {
                let key = &self.items[item_id];
                if let Some(assembled) = self.arguments.get(key) {
                    if assembled != arguments {
                        return Err(crate::Error::InvalidInput(
                            "tool_search argument delta/done mismatch".into(),
                        ));
                    }
                } else {
                    self.arguments.insert(key.clone(), arguments.clone());
                }
                Ok(true)
            }
            Event::ResponseCompleted { response, .. } => {
                for item in response.output.iter().flatten() {
                    if let MessageItem::NativeSearch(NativeSearchItem::ToolSearchCall(call)) = item
                    {
                        self.remember(call, true)?;
                    }
                }
                Ok(false)
            }
            Event::ResponseOutputItemAdded {
                item: MessageItem::NativeSearch(_),
                ..
            }
            | Event::ResponseOutputItemDone {
                item: MessageItem::NativeSearch(_),
                ..
            } => Err(crate::Error::InvalidInput(
                "Unexpected tool_search output from provider".into(),
            )),
            _ => Ok(false),
        }
    }

    pub(super) fn finish(
        &mut self,
        usage: Option<Usage>,
    ) -> crate::Result<Vec<crate::core::language_model::LanguageModelStreamChunk>> {
        if self.calls.len() != self.completed.len() {
            return Err(crate::Error::InvalidInput(
                "Response completed with unfinished tool_search".into(),
            ));
        }
        Ok(std::mem::take(&mut self.calls)
            .into_values()
            .map(|call| {
                crate::core::language_model::LanguageModelStreamChunk::Done(
                    crate::core::messages::AssistantMessage {
                        content: LanguageModelResponseContentType::ToolSearchCall(call),
                        usage: usage.clone(),
                    },
                )
            })
            .collect())
    }
}
