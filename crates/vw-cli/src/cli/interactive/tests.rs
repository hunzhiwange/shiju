use super::run_interactive;
use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::sync::Arc;

use super::super::setup::CliSetup;
const FAILING_PROVIDER_NAME: &str = "interactive-test-failure";

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

fn setup(provider_name: &str) -> CliSetup {
    CliSetup {
        observer: Arc::new(NoopObserver),
        mem: Arc::new(NoneMemory::new()),
        provider: Box::new(StaticProvider),
        tools_registry: Vec::new(),
        provider_name: provider_name.to_string(),
        model_name: "test-model".to_string(),
        system_prompt: "test-system-prompt".to_string(),
        approval_manager: None,
        channel_name: "cli",
    }
}

pub(super) fn run_tui_v3(config: &Config, setup: &CliSetup) -> anyhow::Result<()> {
    if setup.provider_name == FAILING_PROVIDER_NAME {
        let model = config.default_model.as_deref().unwrap_or("<none>");
        anyhow::bail!("test TUI v3 rejected model {model}");
    }

    Ok(())
}

#[tokio::test]
async fn run_interactive_delegates_to_copied_and_ported_tui_v3() {
    let config = Config { default_model: Some("rejected-model".to_string()), ..Config::default() };
    let setup = setup(FAILING_PROVIDER_NAME);
    let mut final_output = "existing output".to_string();

    let error = run_interactive(&config, &setup, &mut final_output)
        .await
        .expect_err("the v3 substitute runner should receive the interactive call");

    assert_eq!(error.to_string(), "test TUI v3 rejected model rejected-model");
    assert_eq!(final_output, "existing output");
}

#[tokio::test]
async fn run_interactive_propagates_tui_error_without_changing_final_output() {
    let config = Config { default_model: Some("rejected-model".to_string()), ..Config::default() };
    let setup = setup(FAILING_PROVIDER_NAME);
    let mut final_output = "existing output".to_string();

    let error = run_interactive(&config, &setup, &mut final_output)
        .await
        .expect_err("the test TUI should reject the configured model");

    assert_eq!(error.to_string(), "test TUI v3 rejected model rejected-model");
    assert_eq!(final_output, "existing output");
}
