use super::computer_control::*;
use super::{Tool, ToolUseContext, execute_tool_from_registry};
use crate::computer_control::policy::{ComputerControlPolicy, ControlFeature};
use parking_lot::RwLock;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vw_api_types::computer_control::*;

#[derive(Default)]
struct OsBoundary {
    inputs: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
}
#[async_trait::async_trait]
impl NativeBackend for OsBoundary {
    async fn observe(
        &self,
        screenshot: bool,
    ) -> Result<ControlOutput, crate::computer_control::native::NativeObservationError> {
        let screenshots = if screenshot {
            vec![
                ScreenshotReference::new("fixture".into(), ScreenshotMimeType::Png, 68, 1, 1)
                    .unwrap(),
            ]
        } else {
            vec![]
        };
        Ok(ControlOutput::Observation { text: "fixture window".into(), screenshots })
    }
    async fn input(
        &self,
        _: &NativeInput,
    ) -> Result<(), crate::computer_control::native::NativeObservationError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        self.inputs.fetch_add(1, Ordering::SeqCst);
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
    fn resolve(
        &self,
        _: &ScreenshotReference,
    ) -> Result<String, crate::computer_control::native::NativeObservationError> {
        use base64::Engine;
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        ))
    }
}
fn fixture(allowed: bool) -> (Arc<NativeControlRuntime>, Arc<OsBoundary>, Vec<Box<dyn Tool>>) {
    let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));
    if allowed {
        policy
            .write()
            .set_enabled(
                &format!("uid:{}", unsafe { libc::geteuid() }),
                ControlFeature::NativeApps,
                true,
            )
            .unwrap();
    }
    let os = Arc::new(OsBoundary::default());
    let boundary = os.clone();
    let runtime = NativeControlRuntime::with_factory(
        policy,
        Arc::new(move |_, _, _, _, _, _| Ok(boundary.clone())),
    );
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(ComputerControlTool::new(runtime.clone(), Some("chat".into())))];
    (runtime, os, tools)
}
fn args(action: serde_json::Value) -> serde_json::Value {
    json!({"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":action})
}
fn context(session: &str) -> Arc<ToolUseContext> {
    Arc::new(ToolUseContext::new(session, None))
}

#[tokio::test]
async fn cc_control_07_default_denial_never_reaches_os() {
    let (_, os, tools) = fixture(false);
    assert!(
        execute_tool_from_registry(
            &tools,
            "computer_control",
            args(json!({"action":"native_input","input":{"kind":"activate"}})),
            context("chat")
        )
        .await
        .is_err()
    );
    assert_eq!(os.inputs.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn cc_control_07_allowed_input_executes_through_host() {
    let (_, os, tools) = fixture(true);
    let result = execute_tool_from_registry(
        &tools,
        "computer_control",
        args(json!({"action":"native_input","input":{"kind":"type_text","text":"中文"}})),
        context("chat"),
    )
    .await
    .unwrap();
    assert!(result.result.is_success());
    assert_eq!(os.inputs.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn cc_control_07_observation_maps_image_for_model() {
    let (_, _, tools) = fixture(true);
    let result = execute_tool_from_registry(
        &tools,
        "computer_control",
        args(json!({"action":"native_observe","screenshot":true})),
        context("chat"),
    )
    .await
    .unwrap()
    .result;
    assert_eq!(result.extra_messages.len(), 1);
    let message = &result.extra_messages[0];
    assert_eq!(message.role, "user");
    let prepared = crate::multimodal::prepare_messages_for_provider(
        &[crate::providers::ChatMessage::user(message.content.as_str().unwrap())],
        &Default::default(),
    )
    .await
    .unwrap();
    assert!(prepared.contains_images);
    let (_, images) = crate::multimodal::parse_image_markers(&prepared.messages[0].content);
    assert_eq!(images.len(), 1);
    assert!(images[0].starts_with("data:image/png;base64,"));
    assert_eq!(result.to_dto().content.len(), 2);
}
#[tokio::test]
async fn cc_control_07_unknown_action_is_rejected_before_os() {
    let (_, os, tools) = fixture(true);
    assert!(
        execute_tool_from_registry(
            &tools,
            "computer_control",
            args(json!({"action":"run_script","script":"bad"})),
            context("chat")
        )
        .await
        .is_err()
    );
    assert_eq!(os.inputs.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn cc_control_07_global_inputs_do_not_overlap_across_sessions() {
    let (runtime, os, first) = fixture(true);
    let second: Vec<Box<dyn Tool>> =
        vec![Box::new(ComputerControlTool::new(runtime, Some("other".into())))];
    let input = args(json!({"action":"native_input","input":{"kind":"activate"}}));
    let (a, b) = tokio::join!(
        execute_tool_from_registry(&first, "computer_control", input.clone(), context("chat")),
        execute_tool_from_registry(&second, "computer_control", input, context("other"))
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(os.inputs.load(Ordering::SeqCst), 2);
    assert_eq!(os.peak.load(Ordering::SeqCst), 1);
    assert!(!first[0].spec().concurrency_safe);
    assert!(first[0].spec().destructive);
}
#[tokio::test]
async fn cc_control_07_revocation_and_cross_session_prevent_input() {
    let (runtime, os, tools) = fixture(true);
    let input = args(json!({"action":"native_input","input":{"kind":"activate"}}));
    assert!(
        execute_tool_from_registry(&tools, "computer_control", input.clone(), context("foreign"))
            .await
            .is_err()
    );
    execute_tool_from_registry(&tools, "computer_control", input.clone(), context("chat"))
        .await
        .unwrap();
    runtime
        .policy
        .write()
        .set_enabled(
            &format!("uid:{}", unsafe { libc::geteuid() }),
            ControlFeature::NativeApps,
            false,
        )
        .unwrap();
    assert!(
        execute_tool_from_registry(&tools, "computer_control", input, context("chat"))
            .await
            .is_err()
    );
    assert_eq!(os.inputs.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn cc_control_07_executor_denial_returns_permission_dto_without_input() {
    let (_, os, tools) = fixture(true);
    let approval =
        Arc::new(crate::approval::ApprovalManager::from_config(&crate::config::AutonomyConfig {
            level: crate::security::AutonomyLevel::Supervised,
            always_ask: vec!["computer_control".into()],
            ..Default::default()
        }));
    let ctx =
        Arc::new(ToolUseContext::new("chat", None).with_approval(approval).with_channel("slack"));
    let error = execute_tool_from_registry(
        &tools,
        "computer_control",
        args(json!({"action":"native_input","input":{"kind":"activate"}})),
        ctx,
    )
    .await
    .unwrap_err();
    assert!(error.permission_request().is_some());
    assert_eq!(os.inputs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cc_control_07_stop_cancels_in_flight_input() {
    let (runtime, os, tools) = fixture(true);
    let running = tokio::spawn(async move {
        execute_tool_from_registry(
            &tools,
            "computer_control",
            args(json!({"action":"native_input","input":{"kind":"activate"}})),
            context("chat"),
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while os.active.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let stop: Vec<Box<dyn Tool>> =
        vec![Box::new(ComputerControlTool::new(runtime, Some("chat".into())))];
    execute_tool_from_registry(
        &stop,
        "computer_control",
        args(json!({"action":"stop"})),
        context("chat"),
    )
    .await
    .unwrap();
    assert!(running.await.unwrap().is_err());
    assert_eq!(os.inputs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cc_control_07_task_abort_cancels_host_effects() {
    let (_runtime, os, tools) = fixture(true);
    let running = tokio::spawn(async move {
        execute_tool_from_registry(
            &tools,
            "computer_control",
            args(json!({"action":"native_input","input":{"kind":"activate"}})),
            context("chat"),
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while os.active.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    assert_eq!(os.inputs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cc_control_07_runtime_factory_registers_shared_authority() {
    let runtime = NativeControlRuntime::start(Default::default()).unwrap();
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
    assert_eq!(tools.iter().filter(|tool| tool.name() == "computer_control").count(), 1);
    assert!(Arc::ptr_eq(&runtime, &NativeControlRuntime::shared().unwrap()));
    assert!(
        execute_tool_from_registry(
            &tools,
            "computer_control",
            args(json!({"action":"native_input","input":{"kind":"activate"}})),
            context.tool_use_context()
        )
        .await
        .is_err()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "requires SHIJU_CC07_FIXTURE pointing to the disposable com.rsung.shiju.input-fixture Cocoa app executable and host Accessibility/Screen Recording permissions"]
async fn cc_control_07_live_registry_input_and_image_reach_model() {
    use crate::computer_control::{native::NativeObserver, policy::GrantScope};
    use vw_config_types::computer_control::GrantDecision;
    struct Fixture(std::process::Child);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let executable =
        std::env::var_os("SHIJU_CC07_FIXTURE").expect("explicit disposable fixture executable");
    let fixture = Fixture(std::process::Command::new(executable).spawn().unwrap());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let target = json!({"kind":"native_app","bundle_id":"com.rsung.shiju.input-fixture","process_id":fixture.0.id()});
    let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));
    let user = format!("uid:{}", unsafe { libc::geteuid() });
    policy
        .write()
        .set_grant(
            &user,
            GrantScope::Application("com.rsung.shiju.input-fixture".into()),
            Some(GrantDecision::Allow),
        )
        .unwrap();
    let runtime = NativeControlRuntime::with_factory(
        policy.clone(),
        Arc::new(|policy, locked, session, version, target, cancel| {
            Ok(Arc::new(NativeObserver::new(policy, locked, session, version, target, cancel)?))
        }),
    );
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(ComputerControlTool::new(runtime, Some("live".into())))];
    let invoke = |action| json!({"target":target,"action":action});
    execute_tool_from_registry(
        &tools,
        "computer_control",
        invoke(json!({"action":"native_input","input":{"kind":"activate"}})),
        context("live"),
    )
    .await
    .unwrap();
    let observed = execute_tool_from_registry(
        &tools,
        "computer_control",
        invoke(json!({"action":"native_observe","screenshot":true})),
        context("live"),
    )
    .await
    .unwrap()
    .result;
    let image_message = &observed.extra_messages[0];
    let prepared = crate::multimodal::prepare_messages_for_provider(
        &[crate::providers::ChatMessage::user(image_message.content.as_str().unwrap())],
        &Default::default(),
    )
    .await
    .unwrap();
    assert!(prepared.contains_images);
    let (_, images) = crate::multimodal::parse_image_markers(&prepared.messages[0].content);
    assert_eq!(images.len(), 1);
    let width = observed.data["output"]["screenshots"][0]["width"].as_u64().unwrap();
    let height = observed.data["output"]["screenshots"][0]["height"].as_u64().unwrap();
    execute_tool_from_registry(&tools,"computer_control",invoke(json!({"action":"native_input","input":{"kind":"click","x":width/2,"y":height/2,"button":"left"}})),context("live")).await.unwrap();
    execute_tool_from_registry(
        &tools,
        "computer_control",
        invoke(
            json!({"action":"native_input","input":{"kind":"type_text","text":"任务7原生工具"}}),
        ),
        context("live"),
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let readback = execute_tool_from_registry(
        &tools,
        "computer_control",
        invoke(json!({"action":"native_observe","screenshot":false})),
        context("live"),
    )
    .await
    .unwrap()
    .result;
    assert!(readback.model_text().contains("任务7原生工具"));
    policy
        .write()
        .set_grant(
            &user,
            GrantScope::Application("com.rsung.shiju.input-fixture".into()),
            Some(GrantDecision::Deny),
        )
        .unwrap();
    assert!(
        execute_tool_from_registry(
            &tools,
            "computer_control",
            invoke(json!({"action":"native_input","input":{"kind":"type_text","text":"不应输入"}})),
            context("live")
        )
        .await
        .is_err()
    );
    println!(
        "CC07_LIVE target={} screenshot={}x{} model_images={} readback=任务7原生工具 denied_input=true",
        target,
        width,
        height,
        images.len()
    );
}
