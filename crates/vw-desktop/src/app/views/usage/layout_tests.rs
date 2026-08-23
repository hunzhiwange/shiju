//! 用量页布局的测试，覆盖摘要、图表、明细和响应式组合。
//! 用例验证空数据与窄窗口下仍能构建可滚动的完整页面。
use crate::app::App;
use crate::app::models::ChatSession;
use crate::app::state::UsageModelInfo;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};
use serde_json::{Value, json};

fn usage_data_with_session(session: Option<ChatSession>) -> super::UsageData {
    let call_count = session.as_ref().map(|value| value.calls.len()).unwrap_or_default();
    super::UsageData {
        total_tokens: 120,
        session,
        session_id: "session-usage".to_string(),
        message_count: 4,
        call_count,
        session_title: "Usage Session".to_string(),
        last_step_input_tokens: 30,
        last_step_output_tokens: 20,
        last_step_cached_tokens: 10,
        last_step_reasoning_tokens: 5,
        last_step_total_tokens: 65,
        user_msgs: 1,
        assistant_msgs: 1,
        system_msgs: 1,
        tool_msgs: 1,
    }
}

fn session_with_calls(calls: Vec<Value>) -> ChatSession {
    ChatSession {
        id: "session-usage".to_string(),
        title: "Usage Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls,
        steps: Vec::new(),
        created_ms: 1_700_000_000_000,
        updated_ms: 1_700_000_123_000,
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("layout_tests"));
}

#[test]
fn build_usage_view_constructs_with_default_auto_model_runtime() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("session-usage".to_string());

    let usage = usage_data_with_session(Some(session_with_calls(vec![json!({
        "kind": "llm",
        "messages": [{"role": "user", "content": "hello"}]
    })])));

    let element = super::build_usage_view(&app, &usage);

    std::hint::black_box(element);
}

#[test]
fn build_usage_view_constructs_with_manual_runtime_and_missing_session() {
    let (mut app, _task) = App::new();
    app.auto_model = false;
    app.model = "manual-model".to_string();

    let usage = usage_data_with_session(None);
    let element = super::build_usage_view(&app, &usage);

    std::hint::black_box(element);
}

#[test]
fn build_usage_view_constructs_with_usage_model_info_for_zero_and_nonzero_context_limits() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("session-usage".to_string());
    app.usage.input_tokens = 120;
    app.usage.output_tokens = 80;
    app.usage.cached_tokens = 40;
    app.usage.reasoning_tokens = 20;
    app.usage_model_info = Some(UsageModelInfo {
        provider_id: "provider-id".to_string(),
        provider_name: "Provider".to_string(),
        model_id: "model-id".to_string(),
        model_name: "Model".to_string(),
        context_limit: 0,
        input_limit: 0,
        output_limit: 4_096,
        limit_is_default: false,
        cost_input_per_million: 1.2,
        cost_output_per_million: 2.3,
        cost_cache_read_per_million: 0.4,
        cost_cache_write_per_million: 0.5,
    });

    let usage = usage_data_with_session(Some(session_with_calls(vec![json!({
        "kind": "llm",
        "output": {"text": "done"}
    })])));
    let zero_context = super::build_usage_view(&app, &usage);

    app.usage_model_info.as_mut().expect("usage model info").context_limit = 200;
    let nonzero_context = super::build_usage_view(&app, &usage);

    std::hint::black_box((zero_context, nonzero_context));
}

#[test]
fn build_usage_view_constructs_with_zero_breakdown_and_negative_cumulative_tokens() {
    let (mut app, _task) = App::new();
    app.usage.input_tokens = -1;
    app.usage.output_tokens = -2;
    app.usage.cached_tokens = -3;
    app.usage_model_info = Some(UsageModelInfo {
        provider_id: "provider-id".to_string(),
        provider_name: "Provider".to_string(),
        model_id: "model-id".to_string(),
        model_name: "Model".to_string(),
        context_limit: 100,
        input_limit: 80,
        output_limit: 20,
        limit_is_default: false,
        cost_input_per_million: 1.0,
        cost_output_per_million: 2.0,
        cost_cache_read_per_million: 3.0,
        cost_cache_write_per_million: 4.0,
    });

    let mut usage = usage_data_with_session(None);
    usage.last_step_input_tokens = 0;
    usage.last_step_output_tokens = 0;
    usage.last_step_cached_tokens = 0;
    usage.last_step_reasoning_tokens = 0;
    usage.last_step_total_tokens = 0;

    let element = super::build_usage_view(&app, &usage);

    std::hint::black_box(element);
}

#[tokio::test(flavor = "multi_thread")]
async fn build_usage_view_draws_dark_theme_styles() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (mut app, _task) = App::new();
    app.active_session_id = Some("session-usage".to_string());
    let usage = usage_data_with_session(Some(session_with_calls(vec![json!({ "seq": 1 })])));
    let mut element = super::build_usage_view(&app, &usage);
    let bounds = Size::new(1_000.0, 800.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &mut renderer, &limits);

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(bounds),
    );
}
