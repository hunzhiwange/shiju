use super::*;
use serde_json::json;

fn test_model(api_id: &str, provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": api_id,
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

#[test]
fn yes_no_cn_is_stable() {
    assert_eq!(yes_no_cn(true), "是");
    assert_eq!(yes_no_cn(false), "否");
}

#[test]
fn environment_text_contains_core_fields() {
    let text = environment_text("gpt-5", "openai", "/tmp/work", true, "no extra dirs");
    assert!(text.contains("gpt-5"));
    assert!(text.contains("openai/gpt-5"));
    assert!(text.contains("/tmp/work"));
    assert!(text.contains("是"));
    assert!(text.contains(platform()));
}

#[test]
fn provider_selects_prompt_family_from_model_id() {
    assert_eq!(provider(&test_model("gpt-5-mini", "openai")), vec![PROMPT_CODEX]);
    assert_eq!(provider(&test_model("gpt-4.1", "openai")), vec![PROMPT_BEAST]);
    assert_eq!(provider(&test_model("o3-mini", "openai")), vec![PROMPT_BEAST]);
    assert_eq!(provider(&test_model("gemini-2.5-pro", "google")), vec![PROMPT_GEMINI]);
    assert_eq!(provider(&test_model("claude-sonnet-4", "anthropic")), vec![PROMPT_ANTHROPIC]);
    assert_eq!(provider(&test_model("Trinity-v1", "local")), vec![PROMPT_TRINITY]);
    assert_eq!(provider(&test_model("qwen-plus", "qwen")), vec![PROMPT_ANTHROPIC_WITHOUT_TODO]);
}

#[test]
fn provider_prefers_the_first_matching_prompt_family() {
    assert_eq!(provider(&test_model("gpt-5-claude-gemini-Trinity", "mixed")), vec![PROMPT_CODEX]);
    assert_eq!(provider(&test_model("gpt-4-claude-gemini-Trinity", "mixed")), vec![PROMPT_BEAST]);
    assert_eq!(provider(&test_model("gemini-claude-Trinity", "mixed")), vec![PROMPT_GEMINI]);
    assert_eq!(provider(&test_model("claude-Trinity", "mixed")), vec![PROMPT_ANTHROPIC]);
}

#[test]
fn instructions_are_trimmed_codex_prompt() {
    let instructions = instructions();

    assert_eq!(instructions, PROMPT_CODEX.trim());
    assert!(!instructions.starts_with('\n'));
    assert!(!instructions.ends_with('\n'));
}

#[test]
fn resolve_cwd_prefers_non_empty_root_and_trims_it() {
    assert_eq!(resolve_cwd(Some("  /tmp/vw-work  ")), "/tmp/vw-work");
}

#[test]
fn environment_text_preserves_structure_for_non_git_workspace() {
    let text = environment_text("model", "provider", "/workspace", false, "");
    let lines = text.lines().collect::<Vec<_>>();

    assert_eq!(lines.len(), 11);
    assert_eq!(lines[0], "你正在使用模型 model。精确模型 ID: provider/model");
    assert_eq!(lines[1], "这里是一些运行环境信息：");
    assert_eq!(lines[2], "<env>");
    assert_eq!(lines[3], "  工作目录: /workspace");
    assert_eq!(lines[4], "  是否为 Git 仓库: 否");
    assert_eq!(lines[5], format!("  平台: {}", platform()));
    assert!(lines[6].starts_with("  今日日期: "));
    assert_eq!(lines[7], "</env>");
    assert_eq!(lines[8], "<directories>");
    assert_eq!(lines[9], "  ");
    assert_eq!(lines[10], "</directories>");
}

#[test]
fn today_date_string_uses_documented_four_component_format() {
    let date = today_date_string();
    let components = date.split_whitespace().collect::<Vec<_>>();

    assert_eq!(components.len(), 4);
    assert_eq!(components[0].len(), 3);
    assert_eq!(components[1].len(), 3);
    assert_eq!(components[3].len(), 4);
}

#[test]
fn detect_git_from_dir_walks_up_to_parent_repo() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();

    assert!(!detect_git_from_dir(nested.to_str().unwrap()));
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    assert!(detect_git_from_dir(nested.to_str().unwrap()));
    assert!(!detect_git_from_dir("   "));
}

#[test]
fn detect_git_from_dir_accepts_trimmed_path_and_git_file_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).expect("nested dir");
    std::fs::write(dir.path().join(".git"), "gitdir: /tmp/worktree-git-dir").expect("git marker");

    let padded = format!("  {}  ", nested.display());
    assert!(detect_git_from_dir(&padded));
}

#[test]
fn resolve_is_git_falls_back_to_directory_detection_without_project_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cwd = dir.path().to_string_lossy();

    assert!(!resolve_is_git(&cwd));
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    assert!(resolve_is_git(&cwd));
}

#[tokio::test]
async fn environment_from_ref_handles_missing_and_malformed_model_refs() {
    let root = "/tmp/vw-agent-env-test";

    let unknown = environment_from_ref(None, Some(root)).await;
    assert!(unknown.contains("unknown/unknown"));
    assert!(unknown.contains(root));

    let malformed = environment_from_ref(Some("not-a-model-ref"), Some(root)).await;
    assert!(malformed.contains("unknown/not-a-model-ref"));

    let missing_provider = environment_from_ref(Some("/orphan-model"), Some(root)).await;
    assert!(missing_provider.contains("精确模型 ID: unknown//orphan-model"));
}

#[tokio::test]
async fn environment_from_ref_preserves_parsed_ids_when_model_lookup_fails() {
    let root = "/tmp/vw-agent-env-missing-model";

    let text = environment_from_ref(Some("missing-provider/missing-model"), Some(root)).await;

    assert!(text.contains("精确模型 ID: missing-provider/missing-model"));
    assert!(text.contains(root));
}
