//! CLI 顶层运行路由的行为测试。
//! 验证有消息与无消息时分别进入单轮和交互模式，并记录正确的可观测完成事件。
//! 初始化、Provider 或交互层失败时必须立即返回错误，不能伪造成功事件。

use super::run::run;
use super::setup::CliSetup;
use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::traits::ObserverMetric;
use crate::app::agent::observability::{Observer, ObserverEvent};
use crate::app::agent::providers::Provider;
use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;

const SETUP_ERROR_PROVIDER: &str = "setup-error-provider";
const SINGLE_MESSAGE_ERROR: &str = "single-message-error";
const INTERACTIVE_ERROR_MODEL: &str = "interactive-error-model";
const INTERACTIVE_OUTPUT: &str = "interactive output";

#[derive(Clone, Debug, PartialEq)]
enum RunCall {
    Setup { interactive: bool, provider_override: Option<String>, model_override: Option<String> },
    SingleMessage { message: String, temperature: f64 },
    Interactive,
}

thread_local! {
    static CALLS: RefCell<Vec<RunCall>> = const { RefCell::new(Vec::new()) };
    static EVENTS: RefCell<Vec<ObserverEvent>> = const { RefCell::new(Vec::new()) };
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

struct RecordingObserver;

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        EVENTS.with(|events| events.borrow_mut().push(event.clone()));
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(super) fn setup_cli(
    config: &Config,
    interactive: bool,
    provider_override: Option<&str>,
    model_override: Option<&str>,
) -> anyhow::Result<CliSetup> {
    CALLS.with(|calls| {
        calls.borrow_mut().push(RunCall::Setup {
            interactive,
            provider_override: provider_override.map(str::to_string),
            model_override: model_override.map(str::to_string),
        });
    });

    if provider_override == Some(SETUP_ERROR_PROVIDER) {
        anyhow::bail!("test setup rejected provider");
    }

    let provider_name = provider_override
        .or(config.default_provider.as_deref())
        .unwrap_or("test-provider")
        .to_string();
    let model_name =
        model_override.or(config.default_model.as_deref()).unwrap_or("test-model").to_string();

    Ok(CliSetup {
        observer: Arc::new(RecordingObserver),
        mem: Arc::new(NoneMemory::new()),
        provider: Box::new(StaticProvider),
        tools_registry: Vec::new(),
        provider_name,
        model_name,
        system_prompt: "test-system-prompt".to_string(),
        approval_manager: None,
        channel_name: "cli",
    })
}

pub(super) async fn run_single_message(
    _config: &Config,
    _setup: &CliSetup,
    message: String,
    temperature: f64,
) -> anyhow::Result<String> {
    CALLS.with(|calls| {
        calls.borrow_mut().push(RunCall::SingleMessage { message: message.clone(), temperature });
    });

    if message == SINGLE_MESSAGE_ERROR {
        anyhow::bail!("test single-message failure");
    }

    Ok(format!("single:{message}"))
}

pub(super) async fn run_interactive(
    _config: &Config,
    setup: &CliSetup,
    final_output: &mut String,
) -> anyhow::Result<()> {
    CALLS.with(|calls| calls.borrow_mut().push(RunCall::Interactive));

    if setup.model_name == INTERACTIVE_ERROR_MODEL {
        anyhow::bail!("test interactive failure");
    }

    final_output.push_str(INTERACTIVE_OUTPUT);
    Ok(())
}

fn reset_recording() {
    CALLS.with(|calls| calls.borrow_mut().clear());
    EVENTS.with(|events| events.borrow_mut().clear());
}

fn recorded_calls() -> Vec<RunCall> {
    CALLS.with(|calls| calls.borrow().clone())
}

fn recorded_events() -> Vec<ObserverEvent> {
    EVENTS.with(|events| events.borrow().clone())
}

fn assert_agent_end(event: &ObserverEvent, provider: &str, model: &str) {
    match event {
        ObserverEvent::AgentEnd {
            provider: actual_provider,
            model: actual_model,
            tokens_used,
            cost_usd,
            ..
        } => {
            assert_eq!(actual_provider, provider);
            assert_eq!(actual_model, model);
            assert_eq!(*tokens_used, None);
            assert_eq!(*cost_usd, None);
        }
        other => panic!("expected AgentEnd event, got {other:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn run_some_message_uses_single_message_mode_and_records_completion() {
    reset_recording();

    let output = run(
        Config::default(),
        Some("hello".to_string()),
        Some("override-provider".to_string()),
        Some("override-model".to_string()),
        0.25,
        vec!["ignored-peripheral".to_string()],
        false,
    )
    .await
    .expect("single-message run should succeed");

    assert_eq!(output, "single:hello");
    assert_eq!(
        recorded_calls(),
        vec![
            RunCall::Setup {
                interactive: false,
                provider_override: Some("override-provider".to_string()),
                model_override: Some("override-model".to_string()),
            },
            RunCall::SingleMessage { message: "hello".to_string(), temperature: 0.25 },
        ]
    );
    let events = recorded_events();
    assert_eq!(events.len(), 1);
    assert_agent_end(&events[0], "override-provider", "override-model");
}

#[tokio::test(flavor = "current_thread")]
async fn run_none_message_uses_interactive_mode_and_returns_accumulated_output() {
    reset_recording();

    let output = run(
        Config::default(),
        None,
        Some("interactive-provider".to_string()),
        Some("interactive-model".to_string()),
        0.7,
        Vec::new(),
        true,
    )
    .await
    .expect("interactive run should succeed");

    assert_eq!(output, INTERACTIVE_OUTPUT);
    assert_eq!(
        recorded_calls(),
        vec![
            RunCall::Setup {
                interactive: true,
                provider_override: Some("interactive-provider".to_string()),
                model_override: Some("interactive-model".to_string()),
            },
            RunCall::Interactive,
        ]
    );
    let events = recorded_events();
    assert_eq!(events.len(), 1);
    assert_agent_end(&events[0], "interactive-provider", "interactive-model");
}

#[tokio::test(flavor = "current_thread")]
async fn run_empty_message_still_uses_single_message_mode() {
    reset_recording();

    let output = run(Config::default(), Some(String::new()), None, None, 0.0, Vec::new(), false)
        .await
        .expect("empty message is still a single-message request");

    assert_eq!(output, "single:");
    assert!(matches!(
        recorded_calls().as_slice(),
        [RunCall::Setup { .. }, RunCall::SingleMessage { message, temperature }]
            if message.is_empty() && temperature.abs() < f64::EPSILON
    ));
    assert_eq!(recorded_events().len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn run_setup_error_short_circuits_without_completion_event() {
    reset_recording();

    let error = run(
        Config::default(),
        Some("unused".to_string()),
        Some(SETUP_ERROR_PROVIDER.to_string()),
        None,
        0.7,
        Vec::new(),
        false,
    )
    .await
    .expect_err("test setup should fail");

    assert_eq!(error.to_string(), "test setup rejected provider");
    assert!(matches!(recorded_calls().as_slice(), [RunCall::Setup { .. }]));
    assert!(recorded_events().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn run_single_message_error_propagates_without_completion_event() {
    reset_recording();

    let error = run(
        Config::default(),
        Some(SINGLE_MESSAGE_ERROR.to_string()),
        None,
        None,
        0.7,
        Vec::new(),
        false,
    )
    .await
    .expect_err("test single-message handler should fail");

    assert_eq!(error.to_string(), "test single-message failure");
    assert!(matches!(
        recorded_calls().as_slice(),
        [RunCall::Setup { .. }, RunCall::SingleMessage { message, .. }]
            if message == SINGLE_MESSAGE_ERROR
    ));
    assert!(recorded_events().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn run_interactive_error_propagates_without_completion_event() {
    reset_recording();

    let error = run(
        Config::default(),
        None,
        None,
        Some(INTERACTIVE_ERROR_MODEL.to_string()),
        0.7,
        Vec::new(),
        true,
    )
    .await
    .expect_err("test interactive handler should fail");

    assert_eq!(error.to_string(), "test interactive failure");
    assert!(matches!(recorded_calls().as_slice(), [RunCall::Setup { .. }, RunCall::Interactive]));
    assert!(recorded_events().is_empty());
}
