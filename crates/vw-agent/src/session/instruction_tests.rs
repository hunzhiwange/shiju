use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use serde_json::{Map, Value, json};

use super::*;
use crate::app::agent::project::instance;
use crate::app::agent::session::message;

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

fn user_info(id: &str) -> message::Info {
    message::Info::User(Box::new(message::UserInfo {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        time: message::UserTime { created: 0 },
        summary: None,
        agent: "agent".to_string(),
        model: message::ModelRef {
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
        },
        system: None,
        tools: None,
        variant: None,
    }))
}

fn part_base(id: &str) -> message::PartBase {
    message::PartBase {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
    }
}

fn completed_tool_part(
    id: &str,
    tool: &str,
    loaded_value: Option<Value>,
    compacted: Option<u64>,
) -> message::Part {
    let mut metadata = Map::new();
    if let Some(loaded_value) = loaded_value {
        metadata.insert("loaded".to_string(), loaded_value);
    }

    message::Part::Tool(message::ToolPart {
        base: part_base(id),
        call_id: format!("call-{id}"),
        tool: tool.to_string(),
        state: message::ToolState::Completed(message::ToolStateCompleted {
            input: Map::new(),
            output: String::new(),
            title: String::new(),
            metadata,
            time: message::ToolStateCompletedTime { start: 1, end: 2, compacted },
            attachments: None,
        }),
        metadata: None,
    })
}

fn running_tool_part(id: &str, tool: &str) -> message::Part {
    message::Part::Tool(message::ToolPart {
        base: part_base(id),
        call_id: format!("call-{id}"),
        tool: tool.to_string(),
        state: message::ToolState::Running(message::ToolStateRunning {
            input: Map::new(),
            title: None,
            metadata: None,
            time: message::PartTime { start: 1, end: None },
        }),
        metadata: None,
    })
}

fn with_parts(parts: Vec<message::Part>) -> message::WithParts {
    message::WithParts { info: user_info("message-1"), parts }
}

fn serve_once(status: &str, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let addr = listener.local_addr().expect("listener address");
    let status = status.to_string();
    let body = body.to_string();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    });
    format!("http://{addr}/instructions")
}

#[test]
fn normalize_str_keeps_paths_comparable() {
    let path = PathBuf::from("alpha").join("beta").join("AGENTS.md");

    assert_eq!(normalize_str(&path), path.to_string_lossy().to_string());
}

#[tokio::test]
async fn resolve_relative_warns_to_empty_when_project_config_is_disabled_without_config_dir() {
    let _guard = ENV_LOCK.lock().await;
    let _disable = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config = EnvGuard::unset("VIBEWINDOW_CONFIG_DIR");

    assert!(resolve_relative("AGENTS.md").is_empty());
}

#[tokio::test]
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
async fn claim_is_scoped_by_message_id_and_clear_removes_claims() {
    let message_id = unique_id("claim");
    let other_message_id = format!("{message_id}-other");
    let filepath = "/tmp/vw-instruction-claim";

    assert!(!is_claimed(&message_id, filepath).await);
    claim(&message_id, filepath).await;

    assert!(is_claimed(&message_id, filepath).await);
    assert!(!is_claimed(&other_message_id, filepath).await);

    clear(&message_id).await;
    assert!(!is_claimed(&message_id, filepath).await);
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
async fn agent_workspace_identity_reads_only_enabled_non_empty_known_files() {
    let _guard = ENV_LOCK.lock().await;
    let base = tempfile::tempdir().expect("agent workspace base");
    let base_path = base.path().join("workspace").to_string_lossy().into_owned();
    let reviewer_root = PathBuf::from(format!("{base_path}-reviewer"));
    std::fs::create_dir_all(&reviewer_root).expect("reviewer workspace");
    std::fs::write(reviewer_root.join("AGENTS.md"), "reviewer agents").expect("write agents");
    std::fs::write(reviewer_root.join("SOUL.md"), "reviewer soul").expect("write soul");
    std::fs::write(reviewer_root.join("TOOLS.md"), "  \n").expect("write empty tools");
    std::fs::write(reviewer_root.join("UNKNOWN.md"), "unknown identity")
        .expect("write unknown identity");
    let project = tempfile::tempdir().expect("selected project");
    let project_agents = project.path().join("AGENTS.md");
    std::fs::write(&project_agents, "selected project protocol").expect("write project agents");
    let _workspace = EnvGuard::set("VIBEWINDOW_WORKSPACE", &base_path);
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let enabled = vec!["SOUL.md".to_string(), "TOOLS.md".to_string(), "UNKNOWN.md".to_string()];

    let resolved = agent_workspace_identity(Some("reviewer"), &enabled);
    let paths = resolved.iter().map(|entry| entry.filepath.as_str()).collect::<HashSet<_>>();

    assert_eq!(resolved.len(), 1);
    assert!(paths.contains(normalize_str(reviewer_root.join("SOUL.md")).as_str()));
    assert!(!paths.contains(normalize_str(reviewer_root.join("AGENTS.md")).as_str()));
    assert!(!paths.contains(normalize_str(reviewer_root.join("TOOLS.md")).as_str()));
    assert!(!paths.contains(normalize_str(reviewer_root.join("UNKNOWN.md")).as_str()));

    let prompt = crate::session::prompt::system_with_agent_metadata(
        None,
        project.path().to_str(),
        Some("reviewer"),
        &enabled,
    );
    assert!(!prompt.content.contains("selected project protocol"));
    assert!(!prompt.content.contains("reviewer soul"));
    assert!(!prompt.content.contains("reviewer agents"));
    assert!(!prompt.content.contains("unknown identity"));
    assert_eq!(prompt.instruction_context().len(), 2);
    assert!(prompt.instruction_context()[0].content.contains("selected project protocol"));
    assert!(prompt.instruction_context()[1].content.contains("reviewer soul"));
    assert_eq!(
        prompt.visible_instruction_paths,
        vec![normalize_str(project_agents), normalize_str(reviewer_root.join("SOUL.md")),]
    );
}

#[tokio::test]
async fn agent_workspace_identity_empty_whitelist_loads_nothing() {
    let _guard = ENV_LOCK.lock().await;
    let base = tempfile::tempdir().expect("agent workspace base");
    let base_path = base.path().join("workspace").to_string_lossy().into_owned();
    let reviewer_root = PathBuf::from(format!("{base_path}-reviewer"));
    std::fs::create_dir_all(&reviewer_root).expect("reviewer workspace");
    std::fs::write(reviewer_root.join("AGENTS.md"), "reviewer agents").expect("write agents");
    std::fs::write(reviewer_root.join("SOUL.md"), "reviewer soul").expect("write soul");
    let _workspace = EnvGuard::set("VIBEWINDOW_WORKSPACE", &base_path);

    let resolved = agent_workspace_identity(Some("reviewer"), &[]);

    assert!(resolved.is_empty());
}

#[tokio::test]
async fn fetch_url_reads_successful_response_and_returns_empty_for_http_errors() {
    let ok_url = serve_once("200 OK", "remote instructions");
    let missing_url = serve_once("404 Not Found", "nope");

    assert_eq!(fetch_url(&ok_url).await, "remote instructions");
    assert_eq!(fetch_url(&missing_url).await, "");
}

#[tokio::test]
async fn fetch_url_returns_empty_for_invalid_url() {
    assert_eq!(fetch_url("not a valid url").await, "");
}

#[test]
fn loaded_collects_uncompacted_read_and_file_read_metadata() {
    let messages = vec![with_parts(vec![
        completed_tool_part("read-1", "read", Some(json!(["/a", "/b", 7])), None),
        completed_tool_part("read-2", "file_read", Some(json!(["/c"])), None),
        completed_tool_part("other-tool", "grep", Some(json!(["/skip-tool"])), None),
        completed_tool_part("compacted", "read", Some(json!(["/skip-compacted"])), Some(99)),
        completed_tool_part("missing", "read", None, None),
        running_tool_part("running", "read"),
    ])];

    let paths = loaded(&messages);

    assert_eq!(paths, HashSet::from(["/a".to_string(), "/b".to_string(), "/c".to_string()]));
}

#[test]
fn loaded_ignores_non_array_metadata_and_deduplicates_paths() {
    let messages = vec![with_parts(vec![
        completed_tool_part("not-array", "read", Some(json!("/ignored")), None),
        completed_tool_part(
            "duplicates",
            "file_read",
            Some(json!(["/same", "/same", null, false])),
            None,
        ),
    ])];

    assert_eq!(loaded(&messages), HashSet::from(["/same".to_string()]));
}

#[tokio::test]
async fn find_reads_only_agents_files() {
    let dir = tempfile::tempdir().expect("instruction dir");
    let claude = dir.path().join("CLAUDE.md");
    let context = dir.path().join("CONTEXT.md");
    std::fs::write(&claude, "claude").expect("write claude");
    std::fs::write(&context, "context").expect("write context");

    assert_eq!(find(&path_string(dir.path())).await, None);

    let agents = dir.path().join("AGENTS.md");
    std::fs::write(&agents, "agents").expect("write agents");
    assert_eq!(find(&path_string(dir.path())).await, Some(normalize_str(&agents)));
    assert_eq!(find(&path_string(dir.path().join("missing"))).await, None);
}

#[tokio::test]
async fn find_ignores_context_when_it_is_the_only_candidate() {
    let dir = tempfile::tempdir().expect("instruction dir");
    let context = dir.path().join("CONTEXT.md");
    std::fs::write(&context, "context").expect("write context");

    assert_eq!(find(&path_string(dir.path())).await, None);
}

#[tokio::test]
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

#[tokio::test]
async fn resolve_claims_unreadable_candidate_without_returning_content() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let leaf = project.path().join("nested");
    std::fs::create_dir_all(leaf.join("AGENTS.md")).expect("create unreadable candidate");
    let target = leaf.join("main.rs");
    std::fs::write(&target, "fn main() {}").expect("write target");
    let found = normalize_str(leaf.join("AGENTS.md"));
    let message_id = unique_id("unreadable-resolve");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let (resolved, claimed) = instance::provide(project.path(), None, move || {
        let target = path_string(target);
        let message_id = message_id.clone();
        Box::pin(async move {
            let resolved = resolve(&[], &target, &message_id).await;
            let claimed = is_claimed(&message_id, &found).await;
            clear(&message_id).await;
            (resolved, claimed)
        })
    })
    .await
    .expect("project instance should provide");

    assert!(resolved.is_empty());
    assert!(claimed);
}

#[tokio::test]
async fn resolve_walks_up_from_target_directory_skips_loaded_files_and_claims_per_message() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let parent = project.path().join("a");
    let leaf = parent.join("b");
    std::fs::create_dir_all(&leaf).expect("leaf dir");
    let target = leaf.join("main.rs");
    let leaf_agents = leaf.join("AGENTS.md");
    let parent_claude = parent.join("AGENTS.md");
    std::fs::write(&target, "fn main() {}").expect("write target");
    std::fs::write(&leaf_agents, "leaf instructions").expect("write leaf agents");
    std::fs::write(&parent_claude, "parent instructions").expect("write parent claude");
    let loaded_parent = normalize_str(&parent_claude);
    let messages = vec![with_parts(vec![completed_tool_part(
        "loaded-parent",
        "read",
        Some(json!([loaded_parent])),
        None,
    )])];
    let message_id = unique_id("resolve");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let (first, second, third) = instance::provide(project.path(), None, move || {
        let messages = messages.clone();
        let target = path_string(&target);
        let message_id = message_id.clone();
        Box::pin(async move {
            let first = resolve(&messages, &target, &message_id).await;
            let second = resolve(&messages, &target, &message_id).await;
            clear(&message_id).await;
            let third = resolve(&messages, &target, &message_id).await;
            (first, second, third)
        })
    })
    .await
    .expect("project instance should provide");

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].filepath, normalize_str(&leaf_agents));
    assert!(first[0].content.contains("leaf instructions"));
    assert!(second.is_empty());
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].filepath, normalize_str(&leaf_agents));
}

#[tokio::test]
async fn resolve_skips_target_instruction_file_and_empty_instruction_content() {
    let _guard = ENV_LOCK.lock().await;
    let project = tempfile::tempdir().expect("project dir");
    let config_dir = tempfile::tempdir().expect("config dir");
    let leaf = project.path().join("nested");
    std::fs::create_dir_all(&leaf).expect("leaf dir");
    let empty_agents = leaf.join("AGENTS.md");
    std::fs::write(&empty_agents, "   \n").expect("write empty agents");
    let message_id = unique_id("empty-resolve");
    let _disable = EnvGuard::unset("VIBEWINDOW_DISABLE_PROJECT_CONFIG");
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.path().as_os_str());

    let (target_is_instruction, empty_result) =
        instance::provide(project.path(), None, move || {
            let empty_agents = path_string(&empty_agents);
            let message_id = message_id.clone();
            Box::pin(async move {
                let target_is_instruction = resolve(&[], &empty_agents, &message_id).await;
                clear(&message_id).await;
                let target = PathBuf::from(&empty_agents).with_file_name("main.rs");
                std::fs::write(&target, "fn main() {}").expect("write target");
                let empty_result = resolve(&[], &path_string(target), &message_id).await;
                (target_is_instruction, empty_result)
            })
        })
        .await
        .expect("project instance should provide");

    assert!(target_is_instruction.is_empty());
    assert!(empty_result.is_empty());
}
