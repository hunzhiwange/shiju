//! Assertion drivers for native cron adapters. Callers supply actual authentication,
//! delivery ports and the composed router; this module never substitutes admission.
use crate::{handlers::*, requests::{CronAddBody, CronUpdateBody}, state::CronState};
use axum::{body::to_bytes, extract::{State, Path}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Json as JsonResponse}};
use agent_config::runtime::Config;
use std::sync::Arc;

async fn response_json<R: IntoResponse>(response: R) -> (StatusCode, serde_json::Value) {
    let response = response.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, value)
}

fn add_body() -> CronAddBody {
    CronAddBody {
        name: Some(" Nightly ".to_string()),
        job_type: None,
        schedule_kind: None,
        schedule: Some("0 0 * * * *".to_string()),
        at: None,
        every_ms: None,
        command: Some(" echo ok ".to_string()),
        prompt: None,
        session_target: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: None,
        fallbacks: None,
        full_access: None,
        task_pool: None,
        delivery_mode: None,
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: None,
        delete_after_run: None,
        ..Default::default()
    }
}

fn update_body() -> CronUpdateBody {
    CronUpdateBody {
        name: None,
        job_type: None,
        schedule_kind: None,
        schedule: None,
        at: None,
        every_ms: None,
        command: None,
        prompt: None,
        session_target: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: None,
        fallbacks: None,
        full_access: None,
        task_pool: None,
        delivery_mode: None,
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: None,
        delete_after_run: None,
        enabled: None,
        ..Default::default()
    }
}


mod denial;
mod delivery;
mod routes;
pub use denial::*;
pub use delivery::*;
pub use routes::*;
