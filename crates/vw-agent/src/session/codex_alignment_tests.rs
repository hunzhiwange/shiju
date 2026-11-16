//! Domain acceptance assertions run against the real host processor, MCP and permission scope.
use session_engine::host_migration_23_acceptance as suite;
use super::processor;
use std::path::Path;
struct Host;
impl suite::AcceptanceHost for Host {
    type Approval = crate::approval::ApprovalManager;
    type Skills = crate::acp::mounts::SessionSkillSources;
    fn enqueue(events: Vec<super::llm::StreamEvent>) { processor::enqueue_test_llm_stream(events); }
    fn run(request: suite::Request<Self>) -> Vec<processor::StreamEvent> {
        let events=std::sync::Arc::new(std::sync::Mutex::new(Vec::new())); let captured=events.clone();
        processor::run(request,move |event| { captured.lock().unwrap().push(event);true });
        std::sync::Arc::try_unwrap(events).unwrap().into_inner().unwrap()
    }
    fn requests(session:&str)->Vec<suite::RecordedRequest> { processor::take_test_llm_messages(session).into_iter().map(|request|suite::RecordedRequest { messages:request.messages }).collect() }
    fn tool_ids()->Vec<Vec<String>> { processor::llm_runner::take_test_llm_stream_tool_ids() }
    fn limit_iterations(runtime:&tokio::runtime::Runtime) { runtime.block_on(crate::config::update(serde_json::json!({"agents":{"main":{"provider":"model-recovery","model":"original","max_iterations":2}}}))).unwrap(); }
}
fn with_fixture(run: impl FnOnce(&Path, &Path, &tokio::runtime::Runtime)) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _config = runtime.block_on(super::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    ));
    let _streams = processor::test_llm_stream_guard();
    let temp = tempfile::tempdir().unwrap();
    let workspace = suite::workspace(temp.path());
    run(&workspace, temp.path(), &runtime);
}

#[test]
fn codex_alignment_skill_search_mcp_compaction_and_new_turn_preserve_boundaries() { with_fixture(suite::codex_alignment_skill_search_mcp_compaction_and_new_turn_preserve_boundaries::<Host>); }
#[test]
fn codex_alignment_explicit_tools_search_disabled_and_disable_tools_keep_authorization() { with_fixture(suite::codex_alignment_explicit_tools_search_disabled_and_disable_tools_keep_authorization::<Host>); }
#[test]
fn codex_alignment_strict_skills_and_last_round_do_not_expand_capabilities() { with_fixture(suite::codex_alignment_strict_skills_and_last_round_do_not_expand_capabilities::<Host>); }
#[test]
fn codex_alignment_same_name_replacement_and_disabled_sources_cannot_replay_old_body() { with_fixture(suite::codex_alignment_same_name_replacement_and_disabled_sources_cannot_replay_old_body::<Host>); }
#[test]
fn codex_alignment_new_session_does_not_inherit_skill_or_tool_exposure() { with_fixture(suite::codex_alignment_new_session_does_not_inherit_skill_or_tool_exposure::<Host>); }
