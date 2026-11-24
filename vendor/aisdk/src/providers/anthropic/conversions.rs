use crate::core::Message;
use crate::core::language_model::{
    LanguageModelOptions, LanguageModelResponseContentType, ReasoningEffort, Usage,
};
use crate::core::messages::UserContentPart;
use crate::providers::anthropic::client::{
    AnthropicAssistantMessageParamContent, AnthropicImageSource, AnthropicMessageDeltaUsage,
    AnthropicMessageParam, AnthropicOptions, AnthropicThinking, AnthropicTool, AnthropicUsage,
};
use crate::providers::anthropic::extensions;

impl From<UserContentPart>
    for crate::providers::anthropic::client::AnthropicUserMessageContentBlock
{
    fn from(value: UserContentPart) -> Self {
        match value {
            UserContentPart::Text(text) => {
                crate::providers::anthropic::client::AnthropicUserMessageContentBlock::Text { text }
            }
            UserContentPart::Image(image) => {
                let source = if let Some((media_type, data)) = image.inline_data() {
                    AnthropicImageSource::Base64 { media_type, data }
                } else {
                    AnthropicImageSource::Url {
                        url: image.image_url,
                    }
                };

                crate::providers::anthropic::client::AnthropicUserMessageContentBlock::Image {
                    source,
                }
            }
        }
    }
}

impl TryFrom<LanguageModelOptions> for AnthropicOptions {
    type Error = crate::Error;

    fn try_from(options: LanguageModelOptions) -> crate::Result<Self> {
        options.reject_native_tool_protocol("anthropic")?;
        let extra_body = options.body.clone();
        let extra_headers = options.headers.clone();
        let mut messages = Vec::new();
        let mut request = AnthropicOptions::builder();
        request.model("");

        // TODO: anthropic max_tokens is required. handle compile
        // time checks if not set in core
        let max_tokens = options.max_output_tokens.unwrap_or(4096);

        // TODO: temperature, top_p, top_k, and stop_sequences are not mapped for Anthropic yet.
        // Add support once provider behavior is confirmed and covered.

        if let Some(system) = options.system
            && !system.is_empty()
        {
            request.system(Some(system));
        } else {
            request.system(None);
        }

        // convert messages to anthropic messages
        for msg in options.messages {
            match msg.message {
                Message::System(s) => {
                    if !s.content.is_empty() {
                        request.system(Some(s.content));
                    }
                }
                Message::User(u) => {
                    let blocks: Vec<_> = u.content.into_iter().map(Into::into).collect();
                    let content = if blocks.len() == 1 {
                        match blocks.into_iter().next().expect("single block should exist") {
                            crate::providers::anthropic::client::AnthropicUserMessageContentBlock::Text {
                                text,
                            } => {
                                crate::providers::anthropic::client::AnthropicUserMessageContent::Text(text)
                            }
                            block => {
                                crate::providers::anthropic::client::AnthropicUserMessageContent::Blocks(vec![block])
                            }
                        }
                    } else {
                        crate::providers::anthropic::client::AnthropicUserMessageContent::Blocks(
                            blocks,
                        )
                    };

                    messages.push(AnthropicMessageParam::User { content });
                }
                Message::Assistant(a) => match a.content {
                    LanguageModelResponseContentType::Text(text) => {
                        messages.push(AnthropicMessageParam::Assistant {
                            content: vec![AnthropicAssistantMessageParamContent::Text { text }],
                        });
                    }
                    LanguageModelResponseContentType::ToolCall(tool) => {
                        messages.push(AnthropicMessageParam::Assistant {
                            content: vec![AnthropicAssistantMessageParamContent::ToolUse {
                                id: tool.tool.id,
                                input: tool.input,
                                name: tool.tool.name,
                            }],
                        });
                    }
                    LanguageModelResponseContentType::Reasoning {
                        content,
                        extensions,
                    } => {
                        // Retrieve Anthropic-specific signature from extensions
                        let signature = extensions
                            .get::<extensions::AnthropicThinkingMetadata>()
                            .signature
                            .clone()
                            .unwrap_or_else(|| content.clone());

                        messages.push(AnthropicMessageParam::Assistant {
                            content: vec![AnthropicAssistantMessageParamContent::Thinking {
                                thinking: content.clone(),
                                signature,
                            }],
                        });
                    }
                    LanguageModelResponseContentType::ToolSearchCall(_)
                    | LanguageModelResponseContentType::ToolSearchOutput(_) => {
                        return Err(crate::Error::InvalidInput(
                            "anthropic does not support the native tool protocol".into(),
                        ));
                    }
                    LanguageModelResponseContentType::NotSupported(_) => {}
                },
                Message::Tool(tool) => {
                    messages.push(AnthropicMessageParam::User {
                        content: crate::providers::anthropic::client::AnthropicUserMessageContent::Blocks(vec![
                            crate::providers::anthropic::client::AnthropicUserMessageContentBlock::ToolResult {
                                tool_use_id: tool.tool.id,
                                content: tool.output.unwrap_or_default().to_string(),
                            },
                        ]),
                    });
                }
                Message::ToolSearchOutput(_) => {
                    return Err(crate::Error::InvalidInput(
                        "anthropic does not support the native tool protocol".into(),
                    ));
                }
                Message::Developer(dev) => {
                    messages.push(AnthropicMessageParam::User {
                        content:
                            crate::providers::anthropic::client::AnthropicUserMessageContent::Text(
                                format!("<developer>\n{dev}\n</developer>"),
                            ),
                    });
                }
            }
        }
        // update messages
        request.messages(messages);

        // convert tools to anthropic tools
        if let Some(tools) = options.tools {
            request.tools(Some(
                tools
                    .tools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .map(|t| {
                        let tool = t.clone();
                        let mut tool_schema = tool.input_schema.to_value();
                        if let Some(schema) = tool_schema.as_object_mut() {
                            schema.remove("$schema");
                        };
                        AnthropicTool {
                            name: tool.name,
                            description: tool.description,
                            input_schema: tool_schema,
                        }
                    })
                    .collect(),
            ));
        }

        request.max_tokens(max_tokens);
        // Effort is a native output setting, not a percentage of max_tokens.
        if let Some(effort) = options.reasoning_effort {
            if effort == ReasoningEffort::None {
                request.thinking(Some(AnthropicThinking::Disable));
            } else {
                request.output_config(Some(serde_json::json!({"effort": effort.as_str()})));
            }
        }

        let mut request = request.build().expect("Failed to build AntropicRequest");
        request.extra_body = extra_body;
        request.extra_headers = extra_headers;
        Ok(request)
    }
}

impl From<AnthropicUsage> for Usage {
    fn from(usage: AnthropicUsage) -> Self {
        Self {
            input_tokens: Some(usage.input_tokens),
            output_tokens: Some(usage.output_tokens),
            cached_tokens: Some(usage.cache_creation_input_tokens + usage.cache_read_input_tokens),
            reasoning_tokens: None,
        }
    }
}

impl From<AnthropicMessageDeltaUsage> for Usage {
    fn from(usage: AnthropicMessageDeltaUsage) -> Self {
        Self {
            input_tokens: Some(usage.input_tokens.unwrap_or(0)),
            output_tokens: Some(usage.output_tokens),
            cached_tokens: Some(
                usage.cache_creation_input_tokens.unwrap_or(0)
                    + usage.cache_read_input_tokens.unwrap_or(0),
            ),
            reasoning_tokens: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Message;
    use crate::core::language_model::{LanguageModelOptions, ReasoningEffort};
    use crate::core::tools::{Tool, ToolExecute, ToolList};
    use crate::core::{UserImage, UserMessage};
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct SumInput {
        a: i32,
        b: i32,
    }

    #[test]
    fn test_scalar_request_options_map_to_anthropic_body() {
        let options = LanguageModelOptions {
            system: Some("You are helpful".to_string()),
            messages: vec![Message::User("Hello".to_string().into()).into()],
            ..Default::default()
        };

        let req: AnthropicOptions = options.try_into().unwrap();

        assert_eq!(req.system.as_deref(), Some("You are helpful"));
        assert_eq!(req.messages.len(), 1);

        match &req.messages[0] {
            AnthropicMessageParam::User { content } => match content {
                crate::providers::anthropic::client::AnthropicUserMessageContent::Text(text) => {
                    assert_eq!(text, "Hello")
                }
                _ => panic!("expected user text message"),
            },
            _ => panic!("expected user message"),
        }
    }

    #[test]
    fn test_reasoning_maps_to_native_effort() {
        let req: AnthropicOptions = LanguageModelOptions {
            max_output_tokens: Some(200),
            reasoning_effort: Some(ReasoningEffort::High),
            ..Default::default()
        }
        .try_into()
        .unwrap();
        assert_eq!(req.output_config, Some(json!({"effort": "high"})));
        assert!(req.thinking.is_none());
        assert_eq!(req.max_tokens, 200);
    }

    #[test]
    fn test_tools_map_to_anthropic_body() {
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

        let req: AnthropicOptions = options.try_into().unwrap();
        let tools = req.tools.expect("tools should be present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "sum");
        assert_eq!(tools[0].description, "Adds two numbers");
        assert_eq!(tools[0].input_schema["type"], json!("object"));
        assert!(tools[0].input_schema["properties"].get("a").is_some());
        assert!(tools[0].input_schema["properties"].get("b").is_some());
        assert!(tools[0].input_schema.get("$schema").is_none());
    }

    #[test]
    fn test_multimodal_user_message_maps_to_anthropic_native_image_blocks() {
        let options = LanguageModelOptions {
            messages: vec![
                Message::User(
                    UserMessage::new("Describe this image")
                        .with_image(UserImage::new("https://example.com/cat.png")),
                )
                .into(),
            ],
            ..Default::default()
        };

        let req: AnthropicOptions = options.try_into().unwrap();

        match &req.messages[0] {
            AnthropicMessageParam::User { content } => match content {
                crate::providers::anthropic::client::AnthropicUserMessageContent::Blocks(
                    blocks,
                ) => {
                    assert!(matches!(
                        blocks.first(),
                        Some(crate::providers::anthropic::client::AnthropicUserMessageContentBlock::Text { text })
                            if text == "Describe this image"
                    ));
                    assert!(matches!(
                        blocks.get(1),
                        Some(crate::providers::anthropic::client::AnthropicUserMessageContentBlock::Image {
                            source: AnthropicImageSource::Url { url }
                        }) if url == "https://example.com/cat.png"
                    ));
                }
                _ => panic!("expected anthropic content blocks"),
            },
            _ => panic!("expected anthropic user message"),
        }
    }

    #[test]
    fn test_anthropic_usage_to_usage_conversion() {
        let usage = AnthropicUsage {
            cache_creation: crate::providers::anthropic::client::AnthropicCacheCreation {
                ephemeral_1h_input_tokens: 0,
                ephemeral_5m_input_tokens: 0,
            },
            cache_creation_input_tokens: 4,
            cache_read_input_tokens: 6,
            input_tokens: 100,
            output_tokens: 50,
            server_tool_use: crate::providers::anthropic::client::AnthropicServerToolUsage::default(
            ),
            service_tier: "standard".to_string(),
        };

        let sdk_usage: Usage = usage.into();
        assert_eq!(sdk_usage.input_tokens, Some(100));
        assert_eq!(sdk_usage.output_tokens, Some(50));
        assert_eq!(sdk_usage.cached_tokens, Some(10));
        assert_eq!(sdk_usage.reasoning_tokens, None);
    }

    #[test]
    fn test_anthropic_delta_usage_to_usage_conversion() {
        let usage = AnthropicMessageDeltaUsage {
            cache_creation_input_tokens: Some(3),
            cache_read_input_tokens: Some(2),
            input_tokens: Some(10),
            output_tokens: 7,
            server_tool_use: None,
        };

        let sdk_usage: Usage = usage.into();
        assert_eq!(sdk_usage.input_tokens, Some(10));
        assert_eq!(sdk_usage.output_tokens, Some(7));
        assert_eq!(sdk_usage.cached_tokens, Some(5));
        assert_eq!(sdk_usage.reasoning_tokens, None);
    }
}
