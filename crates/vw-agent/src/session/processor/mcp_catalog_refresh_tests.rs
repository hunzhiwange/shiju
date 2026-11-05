use super::*;
use crate::tools::mcp_catalog_refresh_tests::Fixture;
use std::sync::{Arc, atomic::Ordering};

#[test]
fn dynamic_tools_mcp_refresh_processor_pins_current_request_and_updates_next_request() {
    run_refresh_processor(true);
}

#[test]
fn dynamic_tools_mcp_refresh_processor_initializes_native_catalog() {
    run_refresh_processor(false);
}

fn run_refresh_processor(injected_catalog: bool) {
    with_discovery_processor_test(|workspace, runtime| {
        let fixture = Arc::new(runtime.block_on(Fixture::start()));
        let catalog = runtime.block_on(fixture.catalog());
        let mut req = discovery_request(workspace.path());
        req.options["allowed_tools"] = serde_json::json!([
            "mcp__catalog__read",
            "mcp__catalog__removed",
            "mcp__catalog__added"
        ]);
        req.session_tools =
            crate::session::runtime::SessionToolset::new(catalog.snapshot().tools().to_vec())
                .with_mcp_connections(fixture.pool.clone());
        let session = req.session.clone();
        let context = Arc::new(
            crate::tools::ToolUseContext::new(session.clone(), req.root.clone())
                .with_live_tool_catalog(catalog.clone()),
        );
        discovery_stream(&[("old", "mcp__catalog__read", serde_json::json!({"value":"old"}))]);
        discovery_stream(&[("new", "mcp__catalog__read", serde_json::json!({"value":1}))]);
        discovery_stream(&[]);
        let request_fixture = fixture.clone();
        let runtime_handle = runtime.handle().clone();
        let run = move || {
            super::super::run(req, move |event| {
                if matches!(event, super::super::StreamEvent::StepStart { step_index: 1, .. }) {
                    request_fixture.service.generation.store(1, Ordering::SeqCst);
                    runtime_handle.block_on(request_fixture.changed());
                }
                true
            });
        };
        if injected_catalog {
            crate::tools::scope_tool_use_context_sync(context, run);
        } else {
            run();
        }
        let ids = super::super::llm_runner::take_test_llm_stream_tool_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids[0].contains(&"mcp__catalog__removed".into()));
        assert!(!ids[1].contains(&"mcp__catalog__removed".into()));
        assert!(!ids[1].contains(&"mcp__catalog__added".into()));
        assert_eq!(
            fixture.service.calls.load(Ordering::SeqCst),
            1,
            "stale call is rejected, next request uses the fresh executor"
        );
        if injected_catalog {
            assert_eq!(
                catalog.snapshot().specs()[0].input_schema["properties"]["value"]["type"],
                "integer"
            );
        }
    });
}
