//! 指令来源与项目路径边界测试。

use super::*;
use std::ffi::OsString;
use agent_project::instance;

static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

fn unique_id(label: &str) -> String {
    format!("{label}-{}-{:?}", std::process::id(), std::thread::current().id())
}

#[tokio::test]
async fn resolve_relative_warns_to_empty_when_project_config_is_disabled_without_config_dir() {
    let _guard = ENV_LOCK.lock().await;
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::unset("VIBEWINDOW_CONFIG_DIR");

    assert!(resolve_relative("AGENTS.md").is_empty());
}

#[tokio::test]
// 项目配置关闭时，相对路径仍被约束在配置目录内。
async fn resolve_relative_when_project_config_is_disabled_stays_inside_config_dir() {
    let _guard = ENV_LOCK.lock().await;
    let config_dir = tempfile::tempdir().expect("config dir");
    let agents = config_dir.path().join("AGENTS.md");
    std::fs::write(&agents, "global instructions").expect("write agents");
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "true");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let found = resolve_relative("AGENTS.md");

    assert_eq!(found, vec![agents]);
}

#[tokio::test]
async fn resolve_relative_uses_project_instance_when_project_config_is_enabled() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let nested = project.path().join("nested");
    std::fs::create_dir_all(&nested).expect("nested dir");
    let local = nested.join("AGENTS.md");
    std::fs::write(&local, "local instructions").expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let found = instance::provide(project.path().join("nested"), None, move || {
        Box::pin(async move { resolve_relative("AGENTS.md") })
    })
    .await
    .expect("project instance should provide");

    assert_eq!(found, vec![local]);
}

#[tokio::test]
async fn system_paths_selects_first_project_instruction_family() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    std::fs::write(project.path().join("AGENTS.md"), "root agents").expect("write agents");
    std::fs::write(project.path().join("CLAUDE.md"), "root claude").expect("write claude");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let paths = instance::provide(project.path(), None, move || {
        Box::pin(async move { system_paths().await })
    })
    .await
    .expect("project instance should provide");

    assert!(paths.contains(&normalize_str(project.path().join("AGENTS.md"))));
    assert!(!paths.contains(&normalize_str(project.path().join("CLAUDE.md"))));
}

#[tokio::test]
async fn system_paths_ignores_legacy_project_instruction_names() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let context = project.path().join("CONTEXT.md");
    std::fs::write(&context, "root context").expect("write context");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let paths = instance::provide(project.path(), None, move || {
        Box::pin(async move { system_paths().await })
    })
    .await
    .expect("project instance should provide");

    assert!(!paths.contains(&normalize_str(context)));
}

#[tokio::test]
async fn system_paths_ignores_project_files_when_project_config_is_disabled() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let project_agents = project.path().join("AGENTS.md");
    std::fs::write(&project_agents, "project agents").expect("write project agents");
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let paths = instance::provide(project.path(), None, move || {
        Box::pin(async move { system_paths().await })
    })
    .await
    .expect("project instance should provide");

    assert!(!paths.contains(&normalize_str(project_agents)));
}

#[tokio::test]
async fn system_reads_first_existing_global_instruction_and_skips_empty_files() {
    let _guard = ENV_LOCK.lock().await;
    let config_dir = tempfile::tempdir().expect("config dir");
    let global_agents = config_dir.path().join("AGENTS.md");
    std::fs::write(&global_agents, "  global instructions\n").expect("write global agents");
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let paths = system_paths().await;
    let instructions = system().await;

    assert!(paths.contains(&normalize_str(&global_agents)));
    assert!(instructions.iter().any(|instruction| {
        instruction.contains(&normalize_str(&global_agents))
            && instruction.contains("global instructions")
    }));

    std::fs::write(&global_agents, "   \n").expect("rewrite empty global agents");
    assert!(
        !system()
            .await
            .iter()
            .any(|instruction| instruction.contains(&normalize_str(&global_agents)))
    );
}

#[tokio::test]
async fn system_skips_global_candidate_that_cannot_be_read_as_a_file() {
    let _guard = ENV_LOCK.lock().await;
    let config_dir = tempfile::tempdir().expect("config dir");
    std::fs::create_dir(config_dir.path().join("AGENTS.md")).expect("create agents directory");
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    assert!(system().await.is_empty());
}

#[tokio::test]
async fn project_agents_reads_only_selected_project_root() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let other = tempfile::tempdir().expect("other project");
    let selected_agents = selected.path().join("AGENTS.md");
    std::fs::write(&selected_agents, "selected project rules").expect("write selected agents");
    std::fs::write(other.path().join("AGENTS.md"), "other project rules")
        .expect("write other agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let config = config::Config::default();
    let resolved = project_agents_with_config(selected.path().to_str(), &config)
        .await
        .expect("selected agents");

    assert_eq!(resolved.filepath, normalize_str(selected_agents));
    assert!(resolved.content.contains("selected project rules"));
    assert!(!resolved.content.contains("other project rules"));
}

#[tokio::test]
async fn project_agents_keeps_skill_paths_as_plain_agents_content() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let agents = selected.path().join("AGENTS.md");
    std::fs::write(&agents, "导航：\n- `./skills/rustcodegraph/SKILL.md`\n- fallback rg\n")
        .expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let resolved = project_agents(selected.path().to_str()).await.expect("selected agents");
    assert!(resolved.content.contains("./skills/rustcodegraph/SKILL.md"));
    assert!(!resolved.content.contains("<required_skills>"));
    assert!(!resolved.content.contains("<unresolved_skill_paths>"));
}

#[tokio::test]
// 项目指令中的技能引用只保留文本语义，不在此阶段隐式加载技能正文。
async fn project_agents_does_not_preload_referenced_skill_instructions() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let skill_dir = selected.path().join("skills/rustcodegraph");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: rustcodegraph\ndescription: graph navigation\n---\n\nMUST_USE_GRAPH_FIRST\n",
    )
    .expect("write skill");
    std::fs::write(
        selected.path().join("AGENTS.md"),
        "导航：先使用 `./skills/rustcodegraph/SKILL.md`。\n",
    )
    .expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let config = config::Config::default();
    let resolved = project_agents_with_config(selected.path().to_str(), &config)
        .await
        .expect("selected agents");

    assert!(!resolved.content.contains("<preloaded_skill_instructions>"));
    assert!(!resolved.content.contains("MUST_USE_GRAPH_FIRST"));
}

#[tokio::test]
async fn project_agents_does_not_preload_skill_referenced_only_by_name() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let skill_dir = selected.path().join("skills/rustcodegraph");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: rustcodegraph\ndescription: graph navigation\n---\n\nMUST_USE_NAMED_GRAPH_FIRST\n",
    )
    .expect("write skill");
    std::fs::write(
        selected.path().join("AGENTS.md"),
        "代码导航优先使用 rustcodegraph 技能；未覆盖时使用 rg。\n",
    )
    .expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let config = config::Config::default();
    let resolved = project_agents_with_config(selected.path().to_str(), &config)
        .await
        .expect("selected agents");

    assert!(!resolved.content.contains("<preloaded_skill_instructions>"));
    assert!(!resolved.content.contains("MUST_USE_NAMED_GRAPH_FIRST"));
}

#[tokio::test]
async fn project_agents_does_not_preload_skill_outside_project_root() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let outside = tempfile::tempdir().expect("outside project");
    let skill_dir = outside.path().join("skills/external");
    std::fs::create_dir_all(&skill_dir).expect("create external skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: external\ndescription: external skill\n---\n\nMUST_NOT_BE_PRELOADED\n",
    )
    .expect("write external skill");
    std::fs::write(
        selected.path().join("AGENTS.md"),
        format!("先使用 `{}`。\n", skill_path.display()),
    )
    .expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let config = config::Config::default();
    let resolved = project_agents_with_config(selected.path().to_str(), &config)
        .await
        .expect("selected agents");

    assert!(resolved.content.contains(&skill_path.display().to_string()));
    assert!(!resolved.content.contains("<unresolved_skill_paths>"));
    assert!(!resolved.content.contains("MUST_NOT_BE_PRELOADED"));
}

#[tokio::test]
async fn project_agents_does_not_synthesize_skill_blocks() {
    let _guard = ENV_LOCK.lock().await;
    let selected = tempfile::tempdir().expect("selected project");
    let agents = selected.path().join("AGENTS.md");
    std::fs::write(&agents, "no skill paths here").expect("write agents");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");

    let resolved = project_agents(selected.path().to_str()).await.expect("selected agents");

    assert!(!resolved.content.contains("<required_skills>"));
}

#[tokio::test]
// 目标路径先做项目根边界检查，拒绝从外部目录加载指令。
async fn resolve_rejects_target_outside_project_root() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let external = tempfile::tempdir().expect("external dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let external_agents = external.path().join("AGENTS.md");
    let target = external.path().join("main.rs");
    std::fs::write(&external_agents, "external instructions").expect("write external agents");
    std::fs::write(&target, "fn main() {}").expect("write external target");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());
    let message_id = unique_id("outside-root");

    let resolved = instance::provide(project.path(), None, move || {
        let target = path_string(target);
        Box::pin(async move { resolve(&[], &target, &message_id).await })
    })
    .await
    .expect("project instance should provide");

    assert!(resolved.is_empty());
}

#[tokio::test]
async fn resolve_rejects_empty_target_without_claiming_instructions() {
    let _guard = ENV_LOCK.lock().await;
    let config_dir = tempfile::tempdir().expect("config dir");
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());
    let message_id = unique_id("empty-target");

    let resolved = resolve(&[], "", &message_id).await;

    assert!(resolved.is_empty());
    clear(&message_id).await;
}
