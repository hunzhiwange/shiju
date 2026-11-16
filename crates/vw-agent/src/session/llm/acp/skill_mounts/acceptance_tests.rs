//! Real host project/configuration injection for the shared CLI acceptance suite.
async fn run(value: &str) {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    crate::session::llm::prompt::ensure_host();
    let input = llm_runtime::acp::acceptance_support::skill_mounts::load_manifest(value);
    let root = std::path::Path::new(input["root"].as_str().unwrap());
    agent_env::set("VW_ACP_SKILL_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = crate::project::instance::Context { directory: root.to_string_lossy().into(), worktree: project.worktree.clone(), project };
    crate::project::instance::provide_context(context, llm_runtime::acp::acceptance_support::skill_mounts::run::<super::super::request::RuntimeHost>(input, value)).await;
}
use serde_json::json;
#[tokio::test]
async fn acp_skill_mount_harness_initial_prompt_contains_no_body() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let skill = root.path().join("skills/acceptance");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: acceptance\ndescription: fixture\n---\nPRIVATE_SKILL_BODY\n",
    )
    .unwrap();
    let mut config = crate::config::Config::default();
    config.agents.insert("acp-codex".into(), serde_json::from_value(json!({"provider":"", "model":"",
        "defaults":{"skills":{"mode":"fixed","value":["acceptance"]},"mcp":{"mode":"fixed","value":[]}}})).unwrap());
    let mut parsed = super::super::ParsedAcpOptions::default();
    let messages = super::super::config::apply_agent_defaults(
        &config,
        &json!({"agent":"acp-codex"}),
        "codex",
        root.path(),
        &mut parsed,
        &json!([{"role":"user","content":"use the skill"}]),
    )
    .await
    .unwrap();
    let prompt = super::super::replay::build_request_prompt(
        &messages,
        false,
        super::super::AcpReplayStrategy::Discard,
        3,
    );
    assert!(!prompt.contains("PRIVATE_SKILL_BODY"));
    assert_eq!(parsed.skill_mounts.len(), 1);
    assert_eq!(std::path::Path::new(&parsed.skill_mounts[0].path), skill.canonicalize().unwrap());
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_codex() {
    run("codex").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_claude() {
    run("claude").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_qwen() {
    run("qwen").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_gemini() {
    run("gemini").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_codebuddy() {
    run("codebuddy").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_goose() {
    run("goose").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_vibe() {
    run("vibe").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_hermes() {
    run("hermes").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_snow() {
    run("snow").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_copilot() {
    run("copilot").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kilocode() {
    run("kilocode").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kimi() {
    run("kimi").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kiro() {
    run("kiro").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_opencode() {
    run("opencode").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_openclaw() {
    run("openclaw").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_pi() {
    run("pi").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_auggie() {
    run("auggie").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_cursor() {
    run("cursor").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_droid() {
    run("droid").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_qoder() {
    run("qoder").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_trae() {
    run("trae").await;
}
#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_iflow() {
    run("iflow").await;
}
