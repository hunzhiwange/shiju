//! 会话 LLM 原始记录的增量表示与无损重建。

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// 同一助手轮次共享、只保存一次的原始请求上下文。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRawTurnContext {
    pub turn_id: String,
    pub session_id: String,
    pub model: Value,
    pub system: Value,
    pub tools: Value,
    pub initial_request_config: Value,
    pub initial_provider_messages: Vec<Value>,
    pub initial_message_sources: Vec<Value>,
}

/// 同一助手轮次内一次 LLM 调用相对上一调用的差量。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRawRound {
    pub turn_id: String,
    pub step_index: u32,
    pub provider_message_prefix_len: usize,
    pub provider_message_suffix: Vec<Value>,
    pub message_source_suffix: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_delta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_delta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_config_delta: Option<Value>,
    pub output: Value,
}

/// 一个助手轮次的共享上下文和逐次 LLM 调用差量。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRawArtifact {
    pub context: LlmRawTurnContext,
    pub rounds: Vec<LlmRawRound>,
}

impl LlmRawArtifact {
    /// 从既有完整 raw JSON 构造增量工件。
    pub fn from_full_raw_rounds(turn_id: impl Into<String>, raw_rounds: &[Value]) -> Result<Self> {
        let Some(first_raw) = raw_rounds.first() else {
            bail!("LLM raw artifact requires at least one round");
        };
        let turn_id = turn_id.into();
        if turn_id.is_empty() {
            bail!("LLM raw artifact requires a non-empty turn_id");
        }

        let first = RawRoundView::parse(first_raw)?;
        let context = LlmRawTurnContext {
            turn_id: turn_id.clone(),
            session_id: first.session_id.clone(),
            model: first.model.clone(),
            system: first.system.clone(),
            tools: first.tools.clone(),
            initial_request_config: first.request_config.clone(),
            initial_provider_messages: first.messages.clone(),
            initial_message_sources: first.message_sources.clone(),
        };

        let mut previous_messages = context.initial_provider_messages.clone();
        let mut previous_sources = context.initial_message_sources.clone();
        let mut previous_system = context.system.clone();
        let mut previous_tools = context.tools.clone();
        let mut previous_config = context.initial_request_config.clone();
        let mut rounds = Vec::with_capacity(raw_rounds.len());

        for raw in raw_rounds {
            let current = RawRoundView::parse(raw)?;
            ensure_turn_context_matches(&context, &current)?;
            let prefix_len = common_prefix_len(
                &previous_messages,
                &previous_sources,
                &current.messages,
                &current.message_sources,
            );
            let request_config_delta =
                (current.request_config != previous_config).then(|| current.request_config.clone());
            let system_delta = (current.system != previous_system).then(|| current.system.clone());
            let tools_delta = (current.tools != previous_tools).then(|| current.tools.clone());
            rounds.push(LlmRawRound {
                turn_id: turn_id.clone(),
                step_index: current.step_index,
                provider_message_prefix_len: prefix_len,
                provider_message_suffix: current.messages[prefix_len..].to_vec(),
                message_source_suffix: current.message_sources[prefix_len..].to_vec(),
                system_delta,
                tools_delta,
                request_config_delta,
                output: current.output.clone(),
            });
            previous_messages = current.messages;
            previous_sources = current.message_sources;
            previous_system = current.system;
            previous_tools = current.tools;
            previous_config = current.request_config;
        }

        Ok(Self { context, rounds })
    }

    /// 按步骤重建兼容既有读取端的完整 raw JSON。
    pub fn reconstruct_round(&self, step_index: u32) -> Result<Value> {
        let mut messages = self.context.initial_provider_messages.clone();
        let mut message_sources = self.context.initial_message_sources.clone();
        let mut system = self.context.system.clone();
        let mut tools = self.context.tools.clone();
        let mut request_config = self.context.initial_request_config.clone();

        for round in &self.rounds {
            if round.turn_id != self.context.turn_id {
                bail!("LLM raw round turn_id does not match its context");
            }
            replace_with_suffix(
                &mut messages,
                round.provider_message_prefix_len,
                &round.provider_message_suffix,
                "provider messages",
            )?;
            replace_with_suffix(
                &mut message_sources,
                round.provider_message_prefix_len,
                &round.message_source_suffix,
                "message sources",
            )?;
            if messages.len() != message_sources.len() {
                bail!("LLM raw round has unaligned provider messages and message sources");
            }
            if let Some(system_delta) = &round.system_delta {
                system = system_delta.clone();
            }
            if let Some(tools_delta) = &round.tools_delta {
                tools = tools_delta.clone();
            }
            if let Some(config_delta) = &round.request_config_delta {
                request_config = config_delta.clone();
            }
            if round.step_index == step_index {
                return Ok(reconstruct_raw_value(
                    &self.context,
                    round.step_index,
                    messages,
                    message_sources,
                    system,
                    tools,
                    request_config,
                    round.output.clone(),
                ));
            }
        }

        bail!("LLM raw round step_index {step_index} does not exist")
    }
}

struct RawRoundView {
    session_id: String,
    step_index: u32,
    model: Value,
    system: Value,
    tools: Value,
    request_config: Value,
    messages: Vec<Value>,
    message_sources: Vec<Value>,
    output: Value,
}

impl RawRoundView {
    fn parse(raw: &Value) -> Result<Self> {
        let object = raw.as_object().context("LLM raw round must be a JSON object")?;
        let session_id = required_value(object, "session_id")?
            .as_str()
            .context("LLM raw round session_id must be a string")?
            .to_string();
        let step_index = required_value(object, "step_index")?
            .as_u64()
            .context("LLM raw round step_index must be an unsigned integer")?
            .try_into()
            .context("LLM raw round step_index exceeds u32")?;
        let request = required_value(object, "request")?
            .as_object()
            .context("LLM raw round request must be a JSON object")?;
        let messages = required_value(object, "messages")?
            .as_array()
            .context("LLM raw round messages must be an array")?
            .clone();
        let message_sources = required_value(object, "message_sources")?
            .as_array()
            .context("LLM raw round message_sources must be an array")?
            .clone();
        if messages.len() != message_sources.len() {
            bail!("LLM raw round messages and message_sources must have the same length");
        }

        Ok(Self {
            session_id,
            step_index,
            model: required_value(object, "model")?.clone(),
            system: required_value(object, "system")?.clone(),
            tools: required_value(request, "tools")?.clone(),
            request_config: required_value(request, "config")?.clone(),
            messages,
            message_sources,
            output: required_value(object, "output")?.clone(),
        })
    }
}

fn required_value<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value> {
    object
        .get(name)
        .with_context(|| format!("LLM raw round is missing {name}"))
}

fn ensure_turn_context_matches(context: &LlmRawTurnContext, current: &RawRoundView) -> Result<()> {
    if current.session_id != context.session_id {
        bail!("LLM raw rounds in one artifact must share session_id");
    }
    if current.model != context.model {
        bail!("LLM raw rounds in one artifact must share model");
    }
    Ok(())
}

fn common_prefix_len(
    previous_messages: &[Value],
    previous_sources: &[Value],
    current_messages: &[Value],
    current_sources: &[Value],
) -> usize {
    previous_messages
        .iter()
        .zip(previous_sources)
        .zip(current_messages.iter().zip(current_sources))
        .take_while(|((previous_message, previous_source), (current_message, current_source))| {
            previous_message == current_message && previous_source == current_source
        })
        .count()
}

fn replace_with_suffix(
    values: &mut Vec<Value>,
    prefix_len: usize,
    suffix: &[Value],
    field_name: &str,
) -> Result<()> {
    if prefix_len > values.len() {
        bail!("LLM raw {field_name} prefix exceeds the previous round length");
    }
    values.truncate(prefix_len);
    values.extend_from_slice(suffix);
    Ok(())
}

fn reconstruct_raw_value(
    context: &LlmRawTurnContext,
    step_index: u32,
    messages: Vec<Value>,
    message_sources: Vec<Value>,
    system: Value,
    tools: Value,
    request_config: Value,
    output: Value,
) -> Value {
    let mut request = Map::new();
    request.insert("config".to_string(), request_config);
    request.insert("tools".to_string(), tools);

    let mut raw = Map::new();
    raw.insert("session_id".to_string(), Value::String(context.session_id.clone()));
    raw.insert("step_index".to_string(), Value::from(step_index));
    raw.insert("model".to_string(), context.model.clone());
    raw.insert("system".to_string(), system);
    raw.insert("messages".to_string(), Value::Array(messages));
    raw.insert("message_sources".to_string(), Value::Array(message_sources));
    raw.insert("request".to_string(), Value::Object(request));
    raw.insert("output".to_string(), output);
    Value::Object(raw)
}
