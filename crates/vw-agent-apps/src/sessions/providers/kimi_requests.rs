//! Request metadata shared by Kimi's session reader and usage importer.

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct RequestInfo {
    pub model: Option<String>,
    pub latency_ms: i64,
    pub first_token_ms: Option<i64>,
    /// When the final `llm.request` attempt for this step started. Committed messages carry
    /// the batch-commit time instead, so imports date requests from this.
    pub request_ms: Option<i64>,
}

#[derive(Default)]
struct Step {
    uuid: String,
    turn_step: Option<String>,
    model: Option<String>,
    request_ms: Option<i64>,
    response_ms: Option<i64>,
    has_tools: bool,
}

/// The human projection contains the synthetic model `agent-loop`. Resolve actual models
/// from loop requests, then join the completed step to its committed message by messageId.
/// Keep only metadata; never retain prompts, tool arguments, or response content here.
#[derive(Default)]
pub(crate) struct RequestIndex {
    /// The agent whose wire is being read; `None` means the main agent.
    agent: Option<String>,
    step: Option<Step>,
    messages: HashMap<String, RequestInfo>,
}

pub(crate) fn timestamp_ms(value: &Value) -> Option<i64> {
    let time = value
        .as_f64()
        .filter(|time| time.is_finite() && *time > 0.0)?;
    let millis = if time < 1e12 { time * 1000.0 } else { time };
    (millis < i64::MAX as f64).then_some(millis as i64)
}

fn model_name(value: &Value) -> Option<&str> {
    value
        .as_str()
        .filter(|name| !name.trim().is_empty() && !matches!(*name, "agent-loop" | "unknown"))
}

fn duration_ms(value: &Value) -> Option<i64> {
    let value = value.as_f64()?;
    (value.is_finite() && (0.0..=3_600_000.0).contains(&value)).then_some(value.round() as i64)
}

impl RequestIndex {
    pub(crate) fn for_agent(agent: &str) -> Self {
        Self { agent: Some(agent.to_owned()), ..Self::default() }
    }

    pub fn observe(&mut self, record: &Value) {
        // Each agent writes its own wire; ignore requests attributed to any other agent.
        let own = self.agent.as_deref().unwrap_or("main");
        if record["agentId"].as_str().is_some_and(|id| id != own) {
            return;
        }
        match record["type"].as_str() {
            Some("agent.switched" | "agent.turn.started") => self.step = None,
            Some("llm.request") => {
                if record["kind"].as_str().is_some_and(|kind| kind != "loop") {
                    return;
                }
                if let Some(step) = &mut self.step {
                    if let (Some(expected), Some(actual)) =
                        (step.turn_step.as_deref(), record["turnStep"].as_str())
                    {
                        if expected != actual {
                            return;
                        }
                    }
                    // Retries replace the failed attempt; native timing belongs to the final one.
                    step.model = model_name(&record["model"]).map(str::to_owned);
                    step.request_ms = timestamp_ms(&record["time"]);
                    step.response_ms = None;
                    step.has_tools = false;
                }
            }
            Some("context.append_loop_event") => {
                let event = &record["event"];
                match event["type"].as_str() {
                    Some("step.begin") => {
                        self.step = event["uuid"].as_str().map(|uuid| Step {
                            uuid: uuid.into(),
                            turn_step: event["turnId"]
                                .as_str()
                                .zip(event["step"].as_u64())
                                .map(|(turn, step)| format!("{turn}.{step}")),
                            ..Default::default()
                        });
                    }
                    Some("content.part" | "tool.call") => {
                        if let Some(step) = &mut self.step {
                            if event["stepUuid"].as_str() != Some(step.uuid.as_str()) {
                                return;
                            }
                            if event["type"] == "tool.call" {
                                step.has_tools = true;
                            } else {
                                // These are completed parts, not first-token notifications.
                                step.response_ms =
                                    step.response_ms.max(timestamp_ms(&record["time"]));
                            }
                        }
                    }
                    Some("step.end") => {
                        if self
                            .step
                            .as_ref()
                            .is_some_and(|step| event["uuid"].as_str() != Some(step.uuid.as_str()))
                        {
                            return;
                        }
                        let step = self.step.take().unwrap_or_default();
                        let Some(id) = event["messageId"].as_str().filter(|id| !id.is_empty())
                        else {
                            return;
                        };
                        let first_token_ms = duration_ms(&event["llmFirstTokenLatencyMs"]);
                        // step.end can be delayed by tool execution. Prefer the native request
                        // durations, which exclude tools, hooks, and client idle time.
                        let native_latency = first_token_ms
                            .zip(duration_ms(&event["llmStreamDurationMs"]))
                            .and_then(|(first, stream)| estimated_latency_ms(0, first + stream));
                        let end_ms = step.response_ms.or_else(|| {
                            (!step.has_tools)
                                .then(|| timestamp_ms(&record["time"]))
                                .flatten()
                        });
                        let latency_ms = native_latency
                            .or_else(|| estimated_latency_ms(step.request_ms?, end_ms?))
                            .unwrap_or(0);
                        // A fork can repeat a step without the original request events.
                        let info = self.messages.entry(id.into()).or_default();
                        info.model = step.model.or_else(|| info.model.take());
                        info.request_ms = step.request_ms.or(info.request_ms);
                        if latency_ms > 0 {
                            info.latency_ms = latency_ms;
                            info.first_token_ms = first_token_ms
                                .filter(|first| *first <= latency_ms)
                                .or(info.first_token_ms.filter(|first| *first <= latency_ms));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn get(&self, meta: &Value) -> Option<&RequestInfo> {
        self.messages.get(meta["messageId"].as_str()?)
    }

    pub fn model<'a>(&'a self, meta: &'a Value) -> Option<&'a str> {
        self.get(meta)
            .and_then(|info| info.model.as_deref())
            .or_else(|| model_name(&meta["model"]["model"]))
    }
}


fn estimated_latency_ms(start: i64, end: i64) -> Option<i64> {
    let elapsed = end.checked_sub(start)?;
    (100..=3_600_000).contains(&elapsed).then_some(elapsed)
}
