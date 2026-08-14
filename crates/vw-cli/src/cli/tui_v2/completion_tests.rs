use std::fs;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use tempfile::tempdir;

use super::commands::registry::CommandSource;
use super::completion::{
    CompletionCatalogItem, CompletionKind, completion_notices, completion_suggestions,
    refresh_file_completion_index, sync_skill_completion_catalog,
};
use super::controller::{TuiController, TuiControllerCommand, TuiControllerEvent};
use super::model::integration::{
    IntegrationAction, IntegrationItem, IntegrationKind, IntegrationPanel,
};
use super::runtime::integrations::integration_command_specs;
use super::state::TuiState;

fn press(
    controller: &mut TuiController,
    state: &mut TuiState,
    code: KeyCode,
) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))),
    )
}

#[test]
fn tui_completion_merges_sources_and_resolves_key_priority_like_reference() {
    let workspace = tempdir().expect("workspace");
    fs::create_dir_all(workspace.path().join("src")).expect("src");
    fs::create_dir_all(workspace.path().join("docs space")).expect("docs");
    fs::write(workspace.path().join("src/main.rs"), "fn main() {}").expect("main");
    fs::write(workspace.path().join("docs space/guide.md"), "guide").expect("guide");
    fs::write(workspace.path().join(".hidden"), "hidden").expect("hidden");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", workspace.path().join("outside-link"))
            .expect("outside symlink");
    }

    let mut state = TuiState::default();
    state.project.workspace_root = Some(workspace.path().to_path_buf());
    refresh_file_completion_index(
        &mut state.completion_catalog,
        state.project.workspace_root.as_deref(),
    );
    state.completion_catalog.items = vec![
        CompletionCatalogItem {
            replacement: "@src/main.rs".to_string(),
            label: "main agent duplicate".to_string(),
            detail: Some("agent source wins by score".to_string()),
            kind: CompletionKind::Agent,
            score: 500,
        },
        CompletionCatalogItem {
            replacement: "@reviewer".to_string(),
            label: "reviewer".to_string(),
            detail: Some("agent".to_string()),
            kind: CompletionKind::Agent,
            score: 80,
        },
        CompletionCatalogItem {
            replacement: "$编译检查".to_string(),
            label: "编译检查".to_string(),
            detail: Some("skill".to_string()),
            kind: CompletionKind::Skill,
            score: 90,
        },
        CompletionCatalogItem {
            replacement: "mcp:repo/readme".to_string(),
            label: "repo/readme".to_string(),
            detail: Some("MCP resource".to_string()),
            kind: CompletionKind::McpResource,
            score: 70,
        },
        CompletionCatalogItem {
            replacement: "#fix".to_string(),
            label: "fix".to_string(),
            detail: None,
            kind: CompletionKind::Keyword,
            score: 40,
        },
    ];

    state.prompt.set_value("@ma");
    let merged = completion_suggestions(&state);
    assert_eq!(
        merged.iter().filter(|item| item.replacement == "@src/main.rs").count(),
        1,
        "same replacement is deduplicated across sources"
    );
    assert_eq!(merged.first().map(|item| item.kind), Some(CompletionKind::Agent));
    assert!(
        merged.iter().all(|item| !item.replacement.contains(".hidden")),
        "hidden files stay excluded"
    );
    assert!(
        merged.iter().all(|item| !item.replacement.contains("outside-link")),
        "symlinks cannot escape the allowed root"
    );

    state.prompt.set_value("@docs");
    let directory = completion_suggestions(&state)
        .into_iter()
        .find(|item| item.kind == CompletionKind::Directory)
        .expect("directory completion");
    assert_eq!(directory.replacement, "@\"docs space\"/");
    assert!(directory.continue_completion);

    state.prompt.set_value("/cd s");
    let cd = completion_suggestions(&state);
    assert!(cd.iter().any(|item| item.replacement == "/cd src/"));
    assert!(cd.iter().all(|item| !item.replacement.contains("outside-link")));

    state.prompt.set_value("$编");
    assert_eq!(
        completion_suggestions(&state).first().map(|item| item.replacement.as_str()),
        Some("$编译检查")
    );
    state.prompt.set_value("mcp:repo");
    assert_eq!(
        completion_suggestions(&state).first().map(|item| item.kind),
        Some(CompletionKind::McpResource)
    );
    state
        .completion_catalog
        .source_errors
        .insert(CompletionKind::McpResource, "offline".to_string());
    assert_eq!(completion_notices(&state), vec!["MCP 补全暂不可用: offline"]);

    state.prompt.set_value("@review");
    let mut controller = TuiController::default();
    assert!(matches!(
        press(&mut controller, &mut state, KeyCode::Enter),
        TuiControllerCommand::Continue
    ));
    assert_eq!(state.prompt.value, "@reviewer", "first Enter accepts instead of submitting");
    press(&mut controller, &mut state, KeyCode::Esc);
    assert!(state.prompt.completion_hidden, "Esc dismisses only the completion surface");
    state.prompt.insert_text("x");
    assert!(!state.prompt.completion_hidden);
    assert!(completion_suggestions(&state).is_empty(), "zero-result query is stable");
}

#[test]
fn at_completion_uses_cached_index_without_rescanning_workspace() {
    let workspace = tempdir().expect("workspace");
    let additional = tempdir().expect("additional workspace");
    fs::create_dir_all(workspace.path().join("src")).expect("src");
    fs::create_dir_all(workspace.path().join("target/debug")).expect("ignored directory");
    let cached_file = workspace.path().join("src/cached.rs");
    fs::write(&cached_file, "cached").expect("cached file");
    fs::write(workspace.path().join(".hidden.rs"), "hidden").expect("hidden file");
    fs::write(workspace.path().join("target/debug/ignored.rs"), "ignored").expect("ignored file");
    fs::write(additional.path().join("extra.md"), "extra").expect("additional file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        additional.path().join("extra.md"),
        workspace.path().join("linked.md"),
    )
    .expect("file symlink");

    let mut state = TuiState::default();
    state.project.workspace_root = Some(workspace.path().to_path_buf());
    refresh_file_completion_index(
        &mut state.completion_catalog,
        state.project.workspace_root.as_deref(),
    );
    let initial_revision = state.completion_catalog.file_index.revision;
    state.prompt.set_value("@cached");

    assert!(
        completion_suggestions(&state).iter().any(|item| item.replacement == "@src/cached.rs"),
        "initial workspace contents are indexed"
    );

    fs::remove_file(cached_file).expect("remove indexed file");
    fs::write(workspace.path().join("src/fresh.rs"), "fresh").expect("fresh file");

    assert!(
        completion_suggestions(&state).iter().any(|item| item.replacement == "@src/cached.rs"),
        "queries reuse the existing index instead of rescanning the workspace"
    );
    state.prompt.set_value("@fresh");
    assert!(
        completion_suggestions(&state).is_empty(),
        "new files stay invisible until an explicit refresh"
    );

    state.completion_catalog.added_directories.push(additional.path().to_path_buf());
    refresh_file_completion_index(
        &mut state.completion_catalog,
        state.project.workspace_root.as_deref(),
    );
    assert_eq!(state.completion_catalog.file_index.revision, initial_revision + 1);
    assert_eq!(state.completion_catalog.file_index.roots.len(), 2);
    assert!(
        completion_suggestions(&state).iter().any(|item| item.replacement == "@src/fresh.rs"),
        "explicit refresh replaces the indexed workspace contents"
    );

    state.prompt.set_value("@");
    let refreshed = completion_suggestions(&state);
    assert!(refreshed.iter().any(|item| item.replacement == "@extra.md"));
    assert!(refreshed.iter().all(|item| !item.replacement.contains("cached.rs")));
    assert!(refreshed.iter().all(|item| !item.replacement.contains(".hidden")));
    assert!(refreshed.iter().all(|item| !item.replacement.contains("target/")));
    assert!(refreshed.iter().all(|item| !item.replacement.contains("linked.md")));
}

#[test]
fn completion_markers_trigger_from_trailing_token_and_preserve_prefix() {
    let mut state = TuiState::default();
    state.completion_catalog.items = vec![
        CompletionCatalogItem {
            replacement: "@src/lib.rs".to_string(),
            label: "src/lib.rs".to_string(),
            detail: None,
            kind: CompletionKind::File,
            score: 100,
        },
        CompletionCatalogItem {
            replacement: "$review".to_string(),
            label: "review".to_string(),
            detail: None,
            kind: CompletionKind::Skill,
            score: 100,
        },
    ];

    state.prompt.set_value("请检查 /mod");
    assert!(
        completion_suggestions(&state).iter().any(|item| item.replacement == "请检查 /model "),
        "slash completion is available from the trailing token"
    );

    state.prompt.set_value("查看 @src");
    assert!(
        completion_suggestions(&state).iter().any(|item| item.replacement == "查看 @src/lib.rs")
    );

    state.prompt.set_value("使用 $rev");
    assert!(completion_suggestions(&state).iter().any(|item| item.replacement == "使用 $review"));

    state.prompt.set_value("使用 $rev ");
    assert!(
        completion_suggestions(&state).is_empty(),
        "ending the active token with whitespace closes completion"
    );
}

#[test]
fn enabled_skills_are_selectable_from_dollar_completion() {
    let toggle_action = |installed| IntegrationAction {
        label: "启用/停用".to_string(),
        enabled: installed,
        disabled_reason: (!installed).then(|| "未安装".to_string()),
    };
    let skills = vec![
        IntegrationItem {
            id: "review-code".to_string(),
            title: "Review Code".to_string(),
            detail: "Review changes".to_string(),
            enabled: true,
            actions: vec![toggle_action(true)],
        },
        IntegrationItem {
            id: "disabled-skill".to_string(),
            title: "Disabled".to_string(),
            detail: "disabled".to_string(),
            enabled: false,
            actions: vec![toggle_action(true)],
        },
        IntegrationItem {
            id: "external-installed".to_string(),
            title: "External installed".to_string(),
            detail: "installed on an external platform".to_string(),
            enabled: true,
            actions: Vec::new(),
        },
        IntegrationItem {
            id: "unsafe\nskill".to_string(),
            title: "Unsafe".to_string(),
            detail: "unsafe".to_string(),
            enabled: true,
            actions: vec![toggle_action(true)],
        },
    ];
    let mut state = TuiState::default();
    state.command_registry = state.command_registry.replace_source(
        CommandSource::Skill,
        Ok(integration_command_specs(IntegrationKind::Skill, &skills)),
    );
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    panel.finish_refresh(Ok(skills));
    sync_skill_completion_catalog(&mut state.completion_catalog, &panel);
    assert!(
        state.completion_catalog.items.iter().any(|item| item.replacement == "$external-installed")
    );
    state.prompt.set_value("$rev");

    let suggestions = completion_suggestions(&state);
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].replacement, "$review-code");
    let mut controller = TuiController::default();
    assert_eq!(press(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "$review-code ");

    panel.finish_refresh(Err("gateway token=secret disconnected".to_string()));
    sync_skill_completion_catalog(&mut state.completion_catalog, &panel);
    state.prompt.set_value("$rev");
    assert_eq!(
        completion_suggestions(&state).len(),
        1,
        "refresh error preserves safe stale skills"
    );
    let notices = completion_notices(&state).join("\n");
    assert!(notices.contains("[redacted]"));
    assert!(!notices.contains("secret"));

    panel.finish_refresh(Ok(Vec::new()));
    sync_skill_completion_catalog(&mut state.completion_catalog, &panel);
    assert!(completion_suggestions(&state).is_empty());
}

#[test]
fn skill_completion_confirmation_closes_picker_and_keeps_prompt_editable() {
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    panel.finish_refresh(Ok(vec![IntegrationItem {
        id: "review-code".to_string(),
        title: "Review Code".to_string(),
        detail: "Review changes".to_string(),
        enabled: true,
        actions: Vec::new(),
    }]));
    let mut state = TuiState::default();
    sync_skill_completion_catalog(&mut state.completion_catalog, &panel);
    state.prompt.set_value("$rev");
    let mut controller = TuiController::default();

    assert_eq!(press(&mut controller, &mut state, KeyCode::Enter), TuiControllerCommand::Continue);
    assert_eq!(state.prompt.value, "$review-code ");
    assert_eq!(state.prompt.cursor.char_index, "$review-code ".chars().count());
    assert!(
        completion_suggestions(&state).is_empty(),
        "the trailing word boundary closes the skill picker"
    );

    assert_eq!(
        press(&mut controller, &mut state, KeyCode::Char('p')),
        TuiControllerCommand::Continue
    );
    assert_eq!(state.prompt.value, "$review-code p");
    assert_eq!(state.prompt.cursor.char_index, "$review-code p".chars().count());
}
