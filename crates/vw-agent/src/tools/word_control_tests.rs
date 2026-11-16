//! Concrete factory binds Word to the shared gateway authority.
use super::execute_tool_from_registry;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn cc_control_16_factory_registers_word_with_shared_gateway_authority() {
    let runtime = super::computer_control::NativeControlRuntime::start(Default::default()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let config = crate::config::Config {
        workspace_dir: root.path().into(),
        config_path: root.path().join("config.toml"),
        ..Default::default()
    };
    let context =
        super::ToolRuntimeContext::new("registry", Some(root.path().to_string_lossy().into()))
            .with_test_config(config);
    let tools = super::toolset::execution_tools_for_context(&context);
    assert_eq!(tools.iter().filter(|tool| tool.name() == "word_control").count(), 1);
    let tool = tools.iter().find(|tool| tool.name() == "word_control").unwrap();
    assert!(tool.spec().destructive);
    assert!(!tool.spec().concurrency_safe);
    assert!(
        execute_tool_from_registry(
            &tools,
            "word_control",
            json!({"operation":"discover"}),
            context.tool_use_context()
        )
        .await
        .is_err()
    );
    assert!(Arc::ptr_eq(
        &runtime,
        &super::computer_control::NativeControlRuntime::shared().unwrap()
    ));
}
