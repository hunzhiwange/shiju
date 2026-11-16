//! Session step projection and provider cost resolution.
use session_ui_types as models;
use gateway_chat_stream::{GatewayChatToolEvent, GatewayChatToolEventKind};
pub fn upsert_step_start(
    session: &mut models::ChatSession,
    step_index: u32,
    created_ms: u64,
    model: Option<String>,
    start_snapshot_path: Option<String>,
) {
    let message_model = model.clone();
    if let Some(step) =
        session.steps.iter_mut().find(|s| s.index == step_index && s.turn_id.is_none())
    {
        step.started_ms = created_ms;
        if model.is_some() {
            step.model = model;
        }
        if start_snapshot_path.is_some() {
            step.start_snapshot_path = start_snapshot_path;
        }
    } else {
        session.steps.push(models::ChatSessionStep {
            index: step_index,
            turn_id: None,
            started_ms: created_ms,
            finished_ms: None,
            start_snapshot_path,
            finish_snapshot_path: None,
            usage: models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model,
            tool_timings: Vec::new(),
        });
    }
    if let Some(model) = message_model.as_deref() {
        super::bind_message_model_to_step(&mut session.messages, step_index, model);
    }
    session.updated_ms = session.updated_ms.max(created_ms);
    session.steps.sort_by_key(|s| s.index);
}

pub fn upsert_step_finish(
    session: &mut models::ChatSession,
    step_index: u32,
    finished_ms: u64,
    usage: models::TokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
) {
    let message_model = model.clone();
    if let Some(step) =
        session.steps.iter_mut().find(|s| s.index == step_index && s.turn_id.is_none())
    {
        step.finished_ms = Some(finished_ms);
        step.usage = usage;
        if finish_reason.is_some() {
            step.finish_reason = finish_reason;
        }
        if model.is_some() {
            step.model = model;
        }
    } else {
        session.steps.push(models::ChatSessionStep {
            index: step_index,
            turn_id: None,
            started_ms: finished_ms,
            finished_ms: Some(finished_ms),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage,
            cost_usd: None,
            finish_reason,
            model,
            tool_timings: Vec::new(),
        });
    }
    if let Some(model) = message_model.as_deref() {
        super::bind_message_model_to_step(&mut session.messages, step_index, model);
    }
    session.updated_ms = session.updated_ms.max(finished_ms);
    session.steps.sort_by_key(|s| s.index);
}

pub fn merge_tool_lifecycle(session: &mut models::ChatSession, event: &GatewayChatToolEvent) {
    let step_position = session
        .steps
        .iter()
        .position(|step| step.index == event.step_index && step.turn_id.is_none());
    let step_position = step_position.unwrap_or_else(|| {
        session.steps.push(models::ChatSessionStep {
            index: event.step_index,
            turn_id: None,
            started_ms: event.occurred_ms,
            finished_ms: None,
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: None,
            tool_timings: Vec::new(),
        });
        session.steps.len() - 1
    });
    let step = &mut session.steps[step_position];
    let timing_position =
        step.tool_timings.iter().position(|timing| timing.call_id == event.call_id).unwrap_or_else(
            || {
                step.tool_timings.push(models::ToolTiming {
                    call_id: event.call_id.clone(),
                    name: event.name.clone(),
                    started_ms: None,
                    finished_ms: None,
                });
                step.tool_timings.len() - 1
            },
        );
    let timing = &mut step.tool_timings[timing_position];
    if timing.name.is_empty() && !event.name.is_empty() {
        timing.name.clone_from(&event.name);
    }
    match event.kind {
        GatewayChatToolEventKind::Start => {
            timing.started_ms.get_or_insert(event.occurred_ms);
        }
        GatewayChatToolEventKind::Update => {}
        GatewayChatToolEventKind::Complete => {
            timing.finished_ms.get_or_insert(event.occurred_ms);
        }
    }
    session.updated_ms = session.updated_ms.max(event.occurred_ms);
    session.steps.sort_by_key(|step| step.index);
}

pub async fn compute_step_cost(
    model: Option<String>,
    usage: models::TokenUsage,
) -> (Option<String>, Option<f64>) {
    use super::provider;

    fn pick_provider_id(candidates: &[String], model_id: &str) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }
        let first_segment = model_id.split('/').next().unwrap_or_default();
        let first_segment_lower = first_segment.to_ascii_lowercase();
        if !first_segment.is_empty() {
            if let Some(v) = candidates.iter().find(|id| id.as_str() == first_segment) {
                return Some(v.clone());
            }
            if let Some(v) =
                candidates.iter().find(|id| id.to_ascii_lowercase().contains(&first_segment_lower))
            {
                return Some(v.clone());
            }
        }

        let model_lower = model_id.to_ascii_lowercase();
        let prefer = if model_lower.starts_with("gpt-") || model_lower.starts_with('o') {
            Some("openai")
        } else if model_lower.contains("claude") {
            Some("anthropic")
        } else if model_lower.contains("deepseek") {
            Some("deepseek")
        } else {
            None
        };
        if let Some(substr) = prefer
            && let Some(v) = candidates.iter().find(|id| id.to_ascii_lowercase().contains(substr))
        {
            return Some(v.clone());
        }
        if model_lower.contains("deepseek")
            && let Some(v) =
                candidates.iter().find(|id| id.to_ascii_lowercase().contains("openrouter"))
        {
            return Some(v.clone());
        }
        Some(candidates[0].clone())
    }

    let parsed = match model {
        Some(s) => {
            if s.contains('/') {
                let parsed = provider::parse_model(&s);
                if provider::get_model(&parsed.provider_id, &parsed.model_id).await.is_ok() {
                    Some(parsed)
                } else {
                    let providers = provider::list().await;
                    let mut candidates = Vec::<String>::new();
                    for (provider_id, info) in providers {
                        if info.models.contains_key(&s) {
                            candidates.push(provider_id);
                        }
                    }
                    pick_provider_id(&candidates, &s).map(|provider_id| provider::ParsedModelRef {
                        provider_id,
                        model_id: s.clone(),
                    })
                }
            } else {
                let providers = provider::list().await;
                let mut candidates = Vec::<String>::new();
                for (provider_id, info) in providers {
                    if info.models.contains_key(&s) {
                        candidates.push(provider_id);
                    }
                }
                pick_provider_id(&candidates, &s).map(|provider_id| provider::ParsedModelRef {
                    provider_id,
                    model_id: s.clone(),
                })
            }
        }
        None => provider::default_model().await.ok(),
    };
    let Some(parsed) = parsed else {
        return (None, None);
    };
    let m = match provider::get_model(&parsed.provider_id, &parsed.model_id).await {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let input = (usage.input_tokens.max(0) as f64) / 1_000_000.0;
    let output = (usage.output_tokens.max(0) as f64) / 1_000_000.0;
    let cached = (usage.cached_tokens.max(0) as f64) / 1_000_000.0;
    let cost = input * m.cost.input + output * m.cost.output + cached * m.cost.cache.read;
    (Some(format!("{}/{}", parsed.provider_id, parsed.model_id)), Some(cost))
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod steps_tests;

pub fn start_llm_timer_at(lifecycle: &mut crate::message::chat::request::RequestLifecycle, now: web_time::Instant) {
    lifecycle.llm_timer.start_at(now);
}

pub fn pause_llm_timer_at(lifecycle: &mut crate::message::chat::request::RequestLifecycle, now: web_time::Instant) {
    lifecycle.llm_timer.pause_at(now);
}
