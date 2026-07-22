use super::usage::{UsageRing, get_usage_details, get_usage_rate_percent};
use crate::app::App;
use crate::app::models::{ChatSessionStep, TokenUsage};
use crate::app::state::UsageModelInfo;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program;
use iced::{Font, Pixels, Rectangle, Size, Theme};

fn test_app() -> App {
    App::new().0
}

fn model_info(context_limit: u64) -> UsageModelInfo {
    UsageModelInfo {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        model_id: "gpt-test".to_string(),
        model_name: "GPT Test".to_string(),
        context_limit,
        output_limit: 4096,
        cost_input_per_million: 2.0,
        cost_output_per_million: 6.0,
        cost_cache_read_per_million: 0.5,
        cost_cache_write_per_million: 1.0,
    }
}

fn step(input_tokens: i64) -> ChatSessionStep {
    ChatSessionStep {
        index: 1,
        started_ms: 1,
        finished_ms: Some(2),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: TokenUsage {
            input_tokens,
            output_tokens: 20,
            cached_tokens: 3,
            reasoning_tokens: 4,
        },
        cost_usd: None,
        finish_reason: None,
        model: Some("gpt-test".to_string()),
    }
}

#[test]
fn task_742_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("usage_tests.rs"));
}

#[test]
fn usage_rate_is_zero_without_model_info_or_with_zero_context_limit() {
    let mut app = test_app();
    app.active_session_view_state.steps.push(step(500));

    assert_eq!(get_usage_rate_percent(&app), 0.0);

    app.usage_model_info = Some(model_info(0));
    assert_eq!(get_usage_rate_percent(&app), 0.0);
}

#[test]
fn usage_rate_uses_last_step_input_tokens() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(2_000));
    app.active_session_view_state.steps.push(step(250));
    app.active_session_view_state.steps.push(step(500));

    assert_eq!(get_usage_rate_percent(&app), 25.0);
}

#[test]
fn usage_rate_is_zero_with_model_info_but_without_steps() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(2_000));

    assert_eq!(get_usage_rate_percent(&app), 0.0);
}

#[test]
fn usage_rate_reports_raw_ratio_above_context_limit() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(1_000));
    app.active_session_view_state.steps.push(step(1_500));

    assert_eq!(get_usage_rate_percent(&app), 150.0);
}

#[test]
fn usage_rate_preserves_fractional_percent_precision() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(3));
    app.active_session_view_state.steps.push(step(1));

    assert!((get_usage_rate_percent(&app) - (100.0 / 3.0)).abs() < f32::EPSILON);
}

#[test]
fn usage_details_report_last_step_context_cost_and_total_tokens() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(4_000));
    app.active_session_view_state.steps.push(step(777));
    app.usage.input_tokens = 100;
    app.usage.output_tokens = 200;
    app.usage.cached_tokens = 30;
    app.usage.reasoning_tokens = 70;

    let (last_input, context_limit, estimated_cost, total_tokens) = get_usage_details(&app);

    assert_eq!(last_input, 777);
    assert_eq!(context_limit, 4_000);
    assert_eq!(total_tokens, 400);
    assert!((estimated_cost - 0.0016).abs() < 1e-12);
}

#[test]
fn usage_details_default_to_zero_when_model_and_steps_are_missing() {
    let app = test_app();

    assert_eq!(get_usage_details(&app), (0, 0, 0.0, 0));
}

#[test]
fn usage_details_keep_step_and_totals_without_model_pricing() {
    let mut app = test_app();
    app.active_session_view_state.steps.push(step(321));
    app.usage.input_tokens = 10;
    app.usage.output_tokens = 20;
    app.usage.cached_tokens = 3;
    app.usage.reasoning_tokens = 4;

    assert_eq!(get_usage_details(&app), (321, 0, 0.0, 37));
}

#[test]
fn usage_details_apply_model_pricing_without_session_steps() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info(8_000));
    app.usage.input_tokens = 400_000;
    app.usage.output_tokens = 300_000;
    app.usage.cached_tokens = 200_000;
    app.usage.reasoning_tokens = 100_000;

    let (last_input, context_limit, estimated_cost, total_tokens) = get_usage_details(&app);

    assert_eq!(last_input, 0);
    assert_eq!(context_limit, 8_000);
    assert_eq!(total_tokens, 1_000_000);
    assert!((estimated_cost - 4.0).abs() < 1e-12);
}

#[test]
fn usage_ring_is_copyable_and_preserves_percent_input() {
    let low = UsageRing { percent: -10.0 };
    let mid = UsageRing { percent: 75.0 };
    let high = UsageRing { percent: 150.0 };

    assert_eq!(low.percent, -10.0);
    assert_eq!(mid.percent, 75.0);
    assert_eq!(high.percent, 150.0);
    assert_eq!(format!("{mid:?}"), "UsageRing { percent: 75.0 }");
}

#[tokio::test]
async fn usage_ring_draws_empty_bounds_and_all_percent_color_ranges() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");

    let empty = UsageRing { percent: 50.0 }.draw(
        &(),
        &renderer,
        &Theme::Dark,
        Rectangle::with_size(Size::new(0.0, 24.0)),
        iced::mouse::Cursor::Unavailable,
    );
    assert_eq!(empty.len(), 1);

    for percent in [-10.0, 0.0, 50.0, 70.0, 89.9, 90.0, 120.0] {
        let geometries = UsageRing { percent }.draw(
            &(),
            &renderer,
            &Theme::Light,
            Rectangle::with_size(Size::new(40.0, 32.0)),
            iced::mouse::Cursor::Unavailable,
        );

        assert_eq!(geometries.len(), 1, "percent {percent} should draw one geometry");
    }
}
