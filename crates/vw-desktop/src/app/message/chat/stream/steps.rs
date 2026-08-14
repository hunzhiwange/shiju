//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::{
    ChatMessage, load_session_or_default, now_ms, save_session_task, session_directory_for_save,
};
use crate::app::{App, Message, models};
use iced::Task;

fn upsert_step_start(
    session: &mut models::ChatSession,
    step_index: u32,
    created_ms: u64,
    model: Option<String>,
    start_snapshot_path: Option<String>,
) {
    let message_model = model.clone();
    if let Some(step) = session.steps.iter_mut().find(|s| s.index == step_index) {
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
            started_ms: created_ms,
            finished_ms: None,
            start_snapshot_path,
            finish_snapshot_path: None,
            usage: models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model,
        });
    }
    if let Some(model) = message_model.as_deref() {
        crate::app::session::bind_message_model_to_step(&mut session.messages, step_index, model);
    }
    session.updated_ms = session.updated_ms.max(created_ms);
    session.steps.sort_by_key(|s| s.index);
}

fn upsert_step_finish(
    session: &mut models::ChatSession,
    step_index: u32,
    finished_ms: u64,
    usage: models::TokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
) {
    let message_model = model.clone();
    if let Some(step) = session.steps.iter_mut().find(|s| s.index == step_index) {
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
            started_ms: finished_ms,
            finished_ms: Some(finished_ms),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage,
            cost_usd: None,
            finish_reason,
            model,
        });
    }
    if let Some(model) = message_model.as_deref() {
        crate::app::session::bind_message_model_to_step(&mut session.messages, step_index, model);
    }
    session.updated_ms = session.updated_ms.max(finished_ms);
    session.steps.sort_by_key(|s| s.index);
}

async fn compute_step_cost(
    model: Option<String>,
    usage: models::TokenUsage,
) -> (Option<String>, Option<f64>) {
    use crate::app::provider::provider;

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

/// 模块内可见函数，执行 handle_agent_step_start 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn start_llm_timer_at(
    runtime: &mut crate::app::state::SessionRuntimeState,
    now: web_time::Instant,
) {
    runtime.llm_timer.start_at(now);
}

pub(super) fn pause_llm_timer_at(
    runtime: &mut crate::app::state::SessionRuntimeState,
    now: web_time::Instant,
) {
    runtime.llm_timer.pause_at(now);
}

pub(super) fn handle_agent_step_start(
    app: &mut App,
    id: u64,
    session_id: String,
    step_index: u32,
    created_ms: u64,
    model: Option<String>,
) -> Task<Message> {
    let Some(_found_session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if session_id.is_empty() {
        return Task::none();
    }
    start_llm_timer_at(app.get_session_runtime_mut(&session_id), web_time::Instant::now());
    let mut session = load_session_or_default(app, session_id.clone());
    upsert_step_start(&mut session, step_index, created_ms, model, None);
    let save_task =
        save_session_task(session.clone(), session_directory_for_save(app, &session_id));
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = session
            .steps
            .iter()
            .find(|step| step.index == step_index)
            .and_then(|step| step.model.as_deref())
        {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }
    save_task
}

/// 模块内可见函数，执行 handle_agent_step_finish 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_step_finish(
    app: &mut App,
    id: u64,
    session_id: String,
    step_index: u32,
    finished_ms: u64,
    usage: models::TokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
) -> Task<Message> {
    let Some(_found_session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if session_id.is_empty() {
        return Task::none();
    }
    pause_llm_timer_at(app.get_session_runtime_mut(&session_id), web_time::Instant::now());

    if app.active_session_id.as_ref() == Some(&session_id) {
        app.usage.input_tokens += usage.input_tokens;
        app.usage.output_tokens += usage.output_tokens;
        app.usage.cached_tokens += usage.cached_tokens;
        app.usage.reasoning_tokens += usage.reasoning_tokens;
    }

    let mut session = load_session_or_default(app, session_id.clone());
    upsert_step_finish(
        &mut session,
        step_index,
        finished_ms,
        usage.clone(),
        finish_reason,
        model.clone(),
    );
    let save_task =
        save_session_task(session.clone(), session_directory_for_save(app, &session_id));
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = session
            .steps
            .iter()
            .find(|step| step.index == step_index)
            .and_then(|step| step.model.as_deref())
        {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }

    Task::batch(vec![
        save_task,
        Task::perform(compute_step_cost(model, usage), move |(resolved_model, cost)| {
            Message::Chat(ChatMessage::AgentStepCostLoaded(
                id,
                session_id,
                step_index,
                resolved_model,
                cost,
            ))
        }),
    ])
}

/// 模块内可见函数，执行 handle_agent_step_cost_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_step_cost_loaded(
    app: &mut App,
    session_id: String,
    step_index: u32,
    resolved_model: Option<String>,
    cost: Option<f64>,
) -> Task<Message> {
    if session_id.is_empty() {
        return Task::none();
    }
    let mut session = load_session_or_default(app, session_id.clone());
    let message_model = resolved_model.clone();
    if let Some(step) = session.steps.iter_mut().find(|s| s.index == step_index) {
        if resolved_model.is_some() {
            step.model = resolved_model;
        }
        if cost.is_some() {
            step.cost_usd = cost;
        }
    } else if resolved_model.is_some() || cost.is_some() {
        session.steps.push(models::ChatSessionStep {
            index: step_index,
            started_ms: now_ms(),
            finished_ms: None,
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: models::TokenUsage::default(),
            cost_usd: cost,
            finish_reason: None,
            model: resolved_model,
        });
        session.steps.sort_by_key(|s| s.index);
    }
    if let Some(model) = message_model.as_deref() {
        crate::app::session::bind_message_model_to_step(&mut session.messages, step_index, model);
    }
    session.updated_ms = session.updated_ms.max(now_ms());
    let save_task =
        save_session_task(session.clone(), session_directory_for_save(app, &session_id));
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = message_model.as_deref() {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }
    save_task
}
#[cfg(test)]
#[path = "steps_tests.rs"]
mod steps_tests;
