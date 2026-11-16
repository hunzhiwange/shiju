//! 验证技能工具的显式引用识别、可见性过滤、路径定位与提示序列化。
//! 测试区分用户授权、运行时临时选择和代理委派，防止技能能力被静默扩大。

use super::{
    SkillTool, block_on, explicitly_referenced_skill_names, filter_delegate_skills,
    skill_file_id, skill_is_explicitly_requested, skill_request_is_explicit,
};
use crate::app::agent::skills::Skill;
use crate::app::agent::tools::context::{ToolUseContext, scope_tool_use_context};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

fn expert_package_fixture() -> (tempfile::TempDir, Arc<crate::config::Config>) {
    let workspace = tempfile::tempdir().expect("expert workspace");
    for (id, body) in
        [("expert-main", "MAIN_WORKFLOW_ONLY"), ("expert-child", "CHILD_WORKFLOW_ONLY")]
    {
        let path = workspace.path().join("skills").join(id).join("SKILL.md");
        std::fs::create_dir_all(path.parent().expect("skill parent")).expect("skill directory");
        std::fs::write(
            path,
            format!("---\nname: {id}\ndescription: Expert package fixture\n---\n# {id}\n{body}\n"),
        )
        .expect("skill file");
    }
    (workspace, Arc::new(crate::config::Config::default()))
}

#[tokio::test]
async fn full_access_loads_model_selected_skill_without_permission_prompt() {
    let (workspace, config) = expert_package_fixture();
    let context = Arc::new(
        ToolUseContext::new("full-access-skill-test", None).with_full_access_enabled(true),
    );
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "full-access-skill-test".to_string(),
        true,
        workspace.path().to_path_buf(),
        config,
    )
    .with_skill_directory_context(&context);
    assert!(tool.description().contains("expert-main"));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        scope_tool_use_context(context, tool.invoke(json!({"skill": "expert-main"}))),
    )
    .await
    .expect("full access must not wait for skill approval");
    assert_eq!(result.data["success"], true, "{:?}", result.data);
    assert!(result.extra_messages[0].content.as_str().unwrap().contains("MAIN_WORKFLOW_ONLY"));
}

#[tokio::test]
async fn full_access_continuation_loads_skills_after_missing_skill_without_reselection() {
    use model_types::ChatMessage;

    let (workspace, config) = expert_package_fixture();
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "continuation-skill-test".to_string(),
        true,
        workspace.path().to_path_buf(),
        config,
    );
    // 复现历史中选过技能、当前仅发送“继续”的路径；完全访问不依赖再次点名。
    let context = Arc::new(
        ToolUseContext::new("continuation-skill-test", None)
            .with_full_access_enabled(true)
            .with_messages_view(vec![
                ChatMessage::user("使用 expert-main 技能"),
                ChatMessage::assistant("技能加载失败"),
                ChatMessage::user("继续"),
            ]),
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        scope_tool_use_context(context.clone(), async {
            assert!(!skill_request_is_explicit("expert-main"));
            let missing = tool.invoke(json!({"skill": "missing-skill"})).await;
            assert_eq!(missing.data["success"], false);
            assert!(missing.data["error"].as_str().unwrap().contains("未找到技能"));
        })
        .await;
        for name in ["expert-main", "expert-child", "expert-main"] {
            let result =
                scope_tool_use_context(context.clone(), tool.invoke(json!({"skill": name}))).await;
            assert_eq!(result.data["success"], true, "{:?}", result.data);
        }
    })
    .await
    .expect("continuation must not wait for repeated skill approval");
}

#[tokio::test]
async fn full_access_preserves_delegate_and_model_invocation_restrictions() {
    let (workspace, config) = expert_package_fixture();
    std::fs::write(
        workspace.path().join("skills/expert-main/SKILL.md"),
        "---\nname: expert-main\ndescription: Restricted fixture\ndisable-model-invocation: true\n---\n# Restricted\n",
    )
    .expect("restricted skill");
    let context = Arc::new(
        ToolUseContext::new("full-access-restricted-skill-test", None)
            .with_full_access_enabled(true)
            .with_delegate_skill_policy(
                config_agent::DelegateCapabilityPolicy::Strict,
                ["expert-main"],
            ),
    );
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "full-access-restricted-skill-test".to_string(),
        false,
        workspace.path().to_path_buf(),
        config,
    )
    .with_skill_directory_context(&context);
    assert!(!tool.description().contains("expert-child"));
    scope_tool_use_context(context, async {
        let main = tool.invoke(json!({"skill": "expert-main"})).await;
        assert_eq!(main.data["success"], false);
        assert!(main.data["error"].as_str().unwrap().contains("禁止模型调用"));
        let child = tool.invoke(json!({"skill": "expert-child"})).await;
        assert_eq!(child.data["success"], false);
        assert!(child.data["error"].as_str().unwrap().contains("未找到技能"));
    })
    .await;
}

#[tokio::test]
async fn expert_package_main_skill_is_explicitly_invocable() {
    let (workspace, config) = expert_package_fixture();
    let loaded = crate::app::agent::skills::load_skills_full_with_config(workspace.path(), &config);
    let main = loaded
        .iter()
        .find(|item| skill_file_id(item).as_deref() == Some("expert-main"))
        .expect("installed main skill");
    let requested = explicitly_referenced_skill_names(&loaded, ["请使用 expert-main 技能"]);
    assert!(requested.contains(&main.name));
    assert!(requested.contains("expert-main"));

    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "expert-test".to_string(),
        false,
        workspace.path().to_path_buf(),
        config,
    );
    let context = Arc::new(
        ToolUseContext::new("expert-test", Some(workspace.path().display().to_string()))
            .with_explicitly_requested_skills(requested),
    );
    let result =
        scope_tool_use_context(context, tool.invoke(json!({"skill": "expert-main"}))).await;
    assert_eq!(result.data["success"], true);
    assert!(
        result.extra_messages[0]
            .content
            .as_str()
            .is_some_and(|body| body.contains("MAIN_WORKFLOW_ONLY"))
    );
    assert!(
        !result.extra_messages[0]
            .content
            .as_str()
            .is_some_and(|body| body.contains("CHILD_WORKFLOW_ONLY"))
    );
    assert_eq!(result.context_updates[0].key, "skill.name");
}

#[tokio::test]
async fn expert_package_children_load_on_demand() {
    let (workspace, config) = expert_package_fixture();
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "expert-child-test".to_string(),
        false,
        workspace.path().to_path_buf(),
        config,
    );
    let context = Arc::new(
        ToolUseContext::new("expert-child-test", Some(workspace.path().display().to_string()))
            .with_explicitly_requested_skills(["expert-main", "expert-child"]),
    );
    scope_tool_use_context(context, async {
        let main = tool.invoke(json!({"skill": "expert-main"})).await;
        let child = tool.invoke(json!({"skill": "expert-child"})).await;
        assert_eq!(main.data["success"], true);
        assert_eq!(child.data["success"], true);
        assert!(
            !main.extra_messages[0]
                .content
                .as_str()
                .is_some_and(|body| body.contains("CHILD_WORKFLOW_ONLY"))
        );
        assert!(
            child.extra_messages[0]
                .content
                .as_str()
                .is_some_and(|body| body.contains("CHILD_WORKFLOW_ONLY"))
        );
        let missing = tool.invoke(json!({"skill": "expert-missing"})).await;
        assert_eq!(missing.data["success"], false);
    })
    .await;
}

#[tokio::test]
async fn expert_package_body_does_not_expand_permissions() {
    let (workspace, config) = expert_package_fixture();
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "expert-policy-test".to_string(),
        false,
        workspace.path().to_path_buf(),
        config,
    );
    let context = Arc::new(
        ToolUseContext::new("expert-policy-test", Some(workspace.path().display().to_string()))
            .with_explicitly_requested_skills(["expert-main"])
            .with_delegate_skill_policy(
                config_agent::DelegateCapabilityPolicy::Strict,
                ["expert-main"],
            ),
    );
    scope_tool_use_context(context, async {
        let main = tool.invoke(json!({"skill": "expert-main"})).await;
        assert_eq!(main.data["success"], true);
        let child = tool.invoke(json!({"skill": "expert-child"})).await;
        assert_eq!(child.data["success"], false);
        assert!(child.data["error"].as_str().is_some_and(|error| error.contains("未找到技能")));
    })
    .await;
}

fn skill(name: &str, location: Option<PathBuf>, prompts: Vec<&str>) -> Skill {
    Skill {
        name: name.to_string(),
        description: format!("{name} description"),
        version: "1.0.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: prompts.into_iter().map(ToOwned::to_owned).collect(),
        location,
        ..Skill::default()
    }
}

// 基础 fixture 默认保持可调用，权限差异由各用例显式覆盖。
#[test]
fn schema_requires_skill_string() {
    let schema = SkillTool::schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["skill"]["type"], "string");
    assert_eq!(schema["properties"]["args"]["type"], "string");
    assert_eq!(schema["required"], json!(["skill"]));
}

#[tokio::test]
async fn trusted_runtime_context_marks_skill_as_explicitly_requested() {
    let context = Arc::new(
        ToolUseContext::new("session", Some("/workspace".to_string()))
            .with_explicitly_requested_skills(["rustcodegraph"]),
    );

    scope_tool_use_context(context, async {
        assert!(skill_request_is_explicit("RustCodeGraph"));
        assert!(!skill_request_is_explicit("other-skill"));
    })
    .await;
}

#[test]
fn explicit_skill_directory_access_accepts_display_name_and_file_id() {
    let item = skill(
        "Rust Code Graph",
        Some(PathBuf::from("/workspace/skills/rustcodegraph/SKILL.md")),
        vec![],
    );
    let display_context = ToolUseContext::new("session", Some("/workspace".to_string()))
        .with_explicitly_requested_skills(["rust code graph"]);
    let file_context = ToolUseContext::new("session", Some("/workspace".to_string()))
        .with_explicitly_requested_skills(["rustcodegraph"]);

    assert!(skill_is_explicitly_requested(&display_context, &item));
    assert!(skill_is_explicitly_requested(&file_context, &item));
}

#[tokio::test]
async fn strict_skill_policy_ignores_temporary_skill_selection() {
    let skills = vec![
        skill("default", None, vec![]),
        skill("trusted", None, vec![]),
        skill("other", None, vec![]),
    ];
    let context = Arc::new(
        ToolUseContext::new("child", Some("/workspace".to_string()))
            .with_trusted_user_selected_skills(["trusted"])
            .with_delegate_skill_policy(
                config_agent::DelegateCapabilityPolicy::Strict,
                ["default"],
            ),
    );

    scope_tool_use_context(context, async {
        let visible = filter_delegate_skills(skills);
        assert_eq!(
            visible.into_iter().map(|skill| skill.name).collect::<Vec<_>>(),
            vec!["default"],
        );
    })
    .await;
}

#[test]
fn block_on_runs_without_an_existing_tokio_runtime() {
    assert_eq!(block_on(async { 21 * 2 }), 42);
}

#[tokio::test]
async fn block_on_bridges_from_an_existing_tokio_runtime() {
    assert_eq!(block_on(async { "bridged".to_string() }), "bridged");
}

#[tokio::test]
async fn agcollab_inherited_skill_denial_cannot_be_overridden_by_child() {
    let (workspace, config) = expert_package_fixture();
    let context =
        Arc::new(ToolUseContext::new("child-skill-denial", None).with_delegated_skill_rules(vec![
            permission_types::Rule {
                permission: "skill".into(),
                pattern: "expert-child".into(),
                action: permission_types::Action::Deny,
            },
        ]));
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        "child-skill-denial".into(),
        false,
        workspace.path().to_path_buf(),
        config,
    )
    .with_skill_directory_context(&context);
    let result =
        scope_tool_use_context(context, tool.invoke(json!({"skill":"expert-child"}))).await;
    assert_eq!(result.data["success"], false);
    assert!(result.data.to_string().contains("Denied"), "{:?}", result.data);
}
