//! 维护运行时工具选择器状态及其回归测试。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{
    AdvancedToolSurfaceState, SessionToolBucket, SessionToolGroup, SessionToolInventory,
    SessionToolSelectorState, SessionToolSelectorTab, ShijuSkillSelection, SkillsDirectoryScope,
    explicit_advanced_tool_surface_spec, tool_bucket, tool_display_name, tool_group,
};

#[test]
fn tool_selector_buckets_match_runtime_surfaces() {
    assert_eq!(tool_bucket("read"), SessionToolBucket::ReadOnly);
    assert_eq!(tool_bucket("file_edit"), SessionToolBucket::Edit);
    assert_eq!(tool_bucket("bash"), SessionToolBucket::Execution);
    assert_eq!(tool_bucket("browser"), SessionToolBucket::Browser);
    assert_eq!(tool_bucket("AgentTool"), SessionToolBucket::Agent);
    assert_eq!(tool_bucket("create_thread"), SessionToolBucket::Agent);
    assert_eq!(tool_group("create_thread"), SessionToolGroup::Collaboration);
    assert_eq!(tool_display_name("create_thread"), "创建任务");
}

#[test]
fn tool_selector_filters_tools_by_enabled_bucket() {
    let mut selector = SessionToolSelectorState::default();
    assert!(!selector.has_tool_filter());

    assert!(selector.toggle_bucket(SessionToolBucket::Browser));
    assert!(selector.has_tool_filter());

    let filtered = selector.filter_tools(&[
        "read".to_string(),
        "browser".to_string(),
        "AgentTool".to_string(),
    ]);

    assert_eq!(filtered, vec!["read".to_string(), "AgentTool".to_string()]);
}

#[test]
fn tool_selector_keeps_last_bucket_enabled() {
    let mut selector = SessionToolSelectorState::default();
    assert!(selector.toggle_bucket(SessionToolBucket::ReadOnly));
    assert!(selector.toggle_bucket(SessionToolBucket::Edit));
    assert!(selector.toggle_bucket(SessionToolBucket::Execution));
    assert!(selector.toggle_bucket(SessionToolBucket::Browser));
    assert!(selector.toggle_bucket(SessionToolBucket::Agent));
    assert!(!selector.toggle_bucket(SessionToolBucket::Other));
    assert!(selector.is_bucket_enabled(SessionToolBucket::Other));
}

#[test]
fn tool_selector_tracks_tab_and_group_collapse_state() {
    let mut selector = SessionToolSelectorState::default();

    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Agent);
    selector.select_tab(SessionToolSelectorTab::Tools);
    selector.toggle_group_collapsed(SessionToolGroup::Execute);

    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Tools);
    assert!(selector.is_group_collapsed(SessionToolGroup::Execute));
    assert_eq!(SessionToolSelectorTab::Skills.label(), "技能");
}

#[test]
fn tool_selector_tracks_search_and_skill_directory_scope() {
    let mut selector = SessionToolSelectorState::default();

    assert_eq!(selector.query(), "");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::All);

    selector.set_query("web".to_string());
    selector.select_skill_directory_scope(SkillsDirectoryScope::Global);

    assert_eq!(selector.query(), "web");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::Global);

    selector.reset();

    assert_eq!(selector.query(), "");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::Global);
}

#[test]
fn tool_selector_tracks_manual_context_selections() {
    let mut selector = SessionToolSelectorState::default();

    selector.toggle_manual_tool("web_search");
    selector.toggle_manual_tool("read");
    selector.toggle_manual_skill("writing-plans");

    assert!(selector.has_manual_context_selection());
    assert!(selector.is_manual_tool_selected("read"));
    assert!(selector.is_manual_skill_selected("writing-plans"));
    assert_eq!(
        selector.selected_manual_tools(),
        vec!["read".to_string(), "web_search".to_string()]
    );
    assert_eq!(selector.selected_manual_skills(), vec!["writing-plans".to_string()]);

    selector.toggle_manual_tool("read");
    assert!(!selector.is_manual_tool_selected("read"));

    selector.reset();
    assert!(!selector.has_manual_context_selection());
}

#[test]
fn tool_selector_tracks_multiple_shiju_functions_and_reset() {
    let mut selector = SessionToolSelectorState::default();
    let first = ShijuSkillSelection::new(7, "document-review".to_string(), "文档审查".to_string());
    let second = ShijuSkillSelection::new(9, "meeting-notes".to_string(), "会议纪要".to_string());

    selector.toggle_shiju_skill(first.clone());
    assert!(selector.is_shiju_skill_selected(first.skill_id()));

    selector.toggle_shiju_skill(second.clone());
    assert_eq!(selector.selected_shiju_skills(), vec![first.clone(), second.clone()]);

    selector.toggle_shiju_skill(second);
    assert_eq!(selector.selected_shiju_skills(), vec![first.clone()]);

    selector.toggle_manual_skill("local-skill");
    selector.toggle_shiju_skill(first.clone());
    assert_eq!(selector.selected_manual_skills(), vec!["local-skill".to_string()]);
    assert!(!selector.is_shiju_skill_selected(first.skill_id()));

    selector.reset();
    assert!(selector.selected_shiju_skills().is_empty());
    assert!(selector.selected_manual_skills().is_empty());
}

#[test]
fn expert_package_selection_is_visible_and_cancellable() {
    let mut selector = SessionToolSelectorState::default();
    let id = "skillhub--skillset--fixture-suite";
    selector.select_manual_skill(id);
    selector.select_manual_skill(id);
    assert_eq!(selector.selected_manual_skills(), vec![id.to_string()]);
    selector.toggle_manual_skill(id);
    assert!(selector.selected_manual_skills().is_empty());
}

#[test]
fn tool_selector_filters_tools_by_explicit_selection() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "apply_patch".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "bash"));

    assert_eq!(selector.filter_tools(&tools), vec!["read".to_string(), "apply_patch".to_string()]);
    assert!(selector.has_custom_tool_selection());
}

#[test]
fn thread_tools_can_be_disabled_individually() {
    let names = [
        "create_thread",
        "fork_thread",
        "list_threads",
        "read_thread",
        "send_message_to_thread",
        "handoff_thread",
        "set_thread_title",
        "set_thread_pinned",
        "set_thread_archived",
        "wait_threads",
    ];
    let mut tools = vec!["read".to_string()];
    tools.extend(names.iter().map(|name| (*name).to_string()));

    for name in names {
        let mut selector = SessionToolSelectorState::default();
        assert_eq!(tool_group(name), SessionToolGroup::Collaboration);
        assert_eq!(tool_bucket(name), SessionToolBucket::Agent);
        assert!(selector.toggle_tool(&tools, name), "could not disable {name}");
        let filtered = selector.filter_tools(&tools);
        assert!(!filtered.iter().any(|tool| tool == name));
        assert_eq!(filtered.len(), tools.len() - 1);
    }
}

#[test]
fn tool_selector_prevents_disabling_last_enabled_tool() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string()];

    assert!(!selector.toggle_tool(&tools, "read"));
    assert_eq!(selector.filter_tools(&tools), tools);
}

#[test]
fn tool_selector_group_toggle_respects_last_enabled_tool() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string()];

    assert!(selector.toggle_group_tools(&tools, SessionToolGroup::Files));
    assert_eq!(selector.filter_tools(&tools), vec!["bash".to_string()]);

    let single_tool = vec!["read".to_string()];
    assert!(!selector.toggle_group_tools(&single_tool, SessionToolGroup::Files));
}

#[test]
fn tool_selector_select_all_clears_explicit_selection() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "apply_patch".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "bash"));
    assert!(selector.has_custom_tool_selection());

    selector.select_all_tools(&tools);

    assert_eq!(selector.filter_tools(&tools), tools);
    assert!(!selector.has_custom_tool_selection());
}

#[test]
fn tool_selector_invert_uses_current_enabled_complement() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "apply_patch".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "bash"));
    assert!(selector.invert_tools(&tools));

    assert_eq!(selector.filter_tools(&tools), vec!["bash".to_string()]);
}

#[test]
fn tool_selector_invert_rejects_empty_result() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string()];

    assert!(!selector.invert_tools(&tools));
}

#[test]
fn tool_selector_group_and_display_name_match_known_tools() {
    assert_eq!(tool_group("read"), SessionToolGroup::Files);
    assert_eq!(tool_group("grep"), SessionToolGroup::Search);
    assert_eq!(tool_group("bash"), SessionToolGroup::Execute);
    assert_eq!(tool_group("browser"), SessionToolGroup::Web);
    assert_eq!(tool_group("memory_store"), SessionToolGroup::Memory);
    assert_eq!(tool_display_name("apply_patch"), "补丁编辑".to_string());
}

#[test]
fn labels_and_descriptions_cover_all_groups() {
    let labels = SessionToolGroup::ALL
        .into_iter()
        .map(|group| (group.label(), group.description()))
        .collect::<Vec<_>>();

    assert_eq!(labels.len(), 8);
    assert!(labels.contains(&("文件", "读取、写入和修改工作区文件。")));
    assert!(labels.contains(&("其他", "当前未归类的工具。")));
}

#[test]
fn manual_context_selection_trims_and_ignores_empty_values() {
    let mut selector = SessionToolSelectorState::default();

    selector.toggle_manual_tool("  bash  ");
    selector.toggle_manual_tool("   ");
    selector.toggle_manual_skill("  skill-a  ");
    selector.toggle_manual_skill("");

    assert_eq!(selector.selected_manual_tools(), vec!["bash".to_string()]);
    assert_eq!(selector.selected_manual_skills(), vec!["skill-a".to_string()]);
}

#[test]
fn reconcile_drops_explicit_tools_no_longer_available() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string(), "browser".to_string()];

    assert!(selector.toggle_tool(&tools, "browser"));
    assert_eq!(selector.filter_tools(&tools), vec!["read".to_string(), "bash".to_string()]);

    selector.reconcile_tools(&["read".to_string()]);

    assert_eq!(selector.filter_tools(&["read".to_string()]), vec!["read".to_string()]);
    assert!(!selector.has_custom_tool_selection());
}

#[test]
fn tool_display_name_title_cases_unknown_ids() {
    assert_eq!(tool_display_name("custom-tool_id"), "Custom Tool Id");
}

#[test]
fn advanced_surface_specs_cover_exact_mappings_and_reject_unknown_tools() {
    let expected = [
        ("enter_worktree", "进入 worktree", AdvancedToolSurfaceState::Available),
        ("exit_worktree", "退出 worktree", AdvancedToolSurfaceState::Available),
        ("mcp_linear", "mcp_* 集成", AdvancedToolSurfaceState::Planned),
    ];

    for (tool_id, expected_label, expected_state) in expected {
        let spec = explicit_advanced_tool_surface_spec(tool_id).expect("known advanced surface");
        assert_eq!(spec.label, expected_label);
        assert_eq!(spec.state, expected_state);
    }

    assert_eq!(AdvancedToolSurfaceState::Available.label(), "available");
    assert_eq!(AdvancedToolSurfaceState::Planned.label(), "planned");
    assert!(explicit_advanced_tool_surface_spec("mcp").is_none());
    assert!(explicit_advanced_tool_surface_spec("unknown_tool").is_none());
}

#[test]
fn tool_selector_defaults_enable_every_bucket_without_filters() {
    let selector = SessionToolSelectorState::default();

    for bucket in SessionToolBucket::ALL {
        assert!(selector.is_bucket_enabled(bucket));
    }
    assert!(!selector.has_tool_filter());
    assert!(!selector.has_manual_context_selection());
    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Agent);
    assert_eq!(selector.query(), "");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::All);

    let inventory = SessionToolInventory::default();
    assert!(inventory.base_tools.is_empty());
    assert!(!inventory.restricts_tools);
}

#[test]
fn tool_selector_toggle_bucket_reenables_removed_bucket() {
    let mut selector = SessionToolSelectorState::default();

    assert!(selector.toggle_bucket(SessionToolBucket::Browser));
    assert!(!selector.is_bucket_enabled(SessionToolBucket::Browser));
    assert!(selector.toggle_bucket(SessionToolBucket::Browser));
    assert!(selector.is_bucket_enabled(SessionToolBucket::Browser));
    assert!(!selector.has_tool_filter());
}

#[test]
fn tool_selector_reset_clears_filters_but_preserves_view_preferences() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string(), "browser".to_string()];

    assert!(selector.toggle_bucket(SessionToolBucket::Browser));
    assert!(selector.toggle_tool(&tools, "bash"));
    selector.toggle_manual_tool("read");
    selector.toggle_manual_skill("skill-a");
    selector.select_tab(SessionToolSelectorTab::Skills);
    selector.toggle_group_collapsed(SessionToolGroup::Files);
    selector.select_skill_directory_scope(SkillsDirectoryScope::Global);
    selector.set_query("read".to_string());

    selector.reset();

    assert_eq!(selector.filter_tools(&tools), tools);
    assert!(!selector.has_tool_filter());
    assert!(!selector.has_manual_context_selection());
    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Skills);
    assert!(selector.is_group_collapsed(SessionToolGroup::Files));
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::Global);
    assert_eq!(selector.query(), "");
}

#[test]
fn tool_selector_group_toggle_reenables_partial_group_and_ignores_empty_group() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "file_write".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "read"));
    assert_eq!(selector.filter_tools(&tools), vec!["file_write".to_string(), "bash".to_string()]);
    assert!(selector.toggle_group_tools(&tools, SessionToolGroup::Files));
    assert_eq!(selector.filter_tools(&tools), tools);
    assert!(selector.toggle_group_tools(&tools, SessionToolGroup::Integration));
    assert_eq!(selector.filter_tools(&tools), tools);
}

#[test]
fn tool_selector_falls_back_when_explicit_selection_misses_new_inventory() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "bash"));
    assert_eq!(selector.filter_tools(&tools), vec!["read".to_string()]);
    assert_eq!(selector.filter_tools(&["bash".to_string()]), vec!["bash".to_string()]);
}

#[test]
fn tool_selector_handles_empty_single_and_unknown_tool_boundaries() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "bash".to_string()];

    assert!(selector.toggle_tool(&tools, "bash"));
    assert!(selector.has_custom_tool_selection());
    assert!(selector.toggle_tool(&tools, "unknown_tool"));
    assert_eq!(selector.filter_tools(&tools), vec!["read".to_string()]);
    assert!(!selector.invert_tools(&[]));
    assert!(!selector.invert_tools(&["read".to_string()]));

    selector.select_all_tools(&[]);

    assert!(!selector.has_custom_tool_selection());
    assert_eq!(selector.filter_tools(&tools), tools);
}

#[test]
fn tool_selector_normalizes_explicit_selection_after_bucket_filter_changes() {
    let mut selector = SessionToolSelectorState::default();
    let tools = vec!["read".to_string(), "browser".to_string()];

    assert!(selector.toggle_tool(&tools, "browser"));
    assert_eq!(selector.filter_tools(&tools), vec!["read".to_string()]);

    assert!(selector.toggle_bucket(SessionToolBucket::ReadOnly));
    selector.reconcile_tools(&tools);

    assert!(!selector.has_custom_tool_selection());
    assert_eq!(selector.filter_tools(&tools), vec!["browser".to_string()]);

    selector.select_all_tools(&["read".to_string()]);
    assert_eq!(selector.filter_tools(&["read".to_string()]), Vec::<String>::new());
}

#[test]
fn tool_selector_toggles_group_collapse_and_manual_skill_off_again() {
    let mut selector = SessionToolSelectorState::default();

    selector.toggle_group_collapsed(SessionToolGroup::Memory);
    selector.toggle_manual_skill("skill-a");
    assert!(selector.is_group_collapsed(SessionToolGroup::Memory));
    assert!(selector.is_manual_skill_selected("skill-a"));

    selector.toggle_group_collapsed(SessionToolGroup::Memory);
    selector.toggle_manual_skill("skill-a");
    assert!(!selector.is_group_collapsed(SessionToolGroup::Memory));
    assert!(!selector.is_manual_skill_selected("skill-a"));
}

#[test]
fn selector_tab_labels_and_tool_groups_cover_every_variant() {
    assert_eq!(SessionToolSelectorTab::Agent.label(), "代理");
    assert_eq!(SessionToolSelectorTab::Tools.label(), "工具");
    assert_eq!(SessionToolSelectorTab::Skills.label(), "技能");

    let cases = [
        ("file_write", SessionToolGroup::Files),
        ("code_search", SessionToolGroup::Search),
        ("git_operations", SessionToolGroup::Execute),
        ("http_request", SessionToolGroup::Web),
        ("question", SessionToolGroup::Collaboration),
        ("memory_forget", SessionToolGroup::Memory),
        ("state_get", SessionToolGroup::Integration),
        ("unregistered_tool", SessionToolGroup::Other),
    ];

    for (tool_id, expected) in cases {
        assert_eq!(tool_group(tool_id), expected);
    }
    assert_eq!(tool_bucket("unregistered_tool"), SessionToolBucket::Other);
}

#[test]
fn legacy_read_file_alias_keeps_read_only_bucket_and_existing_other_group() {
    assert_eq!(tool_bucket("read_file"), SessionToolBucket::ReadOnly);
    assert_eq!(tool_group("read_file"), SessionToolGroup::Other);
    assert_eq!(tool_display_name("read_file"), "Read File");
}

#[test]
fn tool_buckets_cover_supported_runtime_aliases() {
    let cases = [
        ("file_read", SessionToolBucket::ReadOnly),
        ("pdf_read", SessionToolBucket::ReadOnly),
        ("ls", SessionToolBucket::ReadOnly),
        ("lsp", SessionToolBucket::ReadOnly),
        ("write", SessionToolBucket::Edit),
        ("edit", SessionToolBucket::Edit),
        ("edit_file", SessionToolBucket::Edit),
        ("editfile", SessionToolBucket::Edit),
        ("notebook_edit", SessionToolBucket::Edit),
        ("shell", SessionToolBucket::Execution),
        ("process", SessionToolBucket::Execution),
        ("browser_open", SessionToolBucket::Browser),
        ("fetch_webpage", SessionToolBucket::Browser),
        ("screenshot", SessionToolBucket::Browser),
        ("cron_add", SessionToolBucket::Agent),
        ("cron_runs", SessionToolBucket::Agent),
        ("todoread", SessionToolBucket::Agent),
    ];

    for (tool_id, expected) in cases {
        assert_eq!(tool_bucket(tool_id), expected);
    }
}

#[test]
fn tool_groups_cover_supported_runtime_aliases() {
    let cases = [
        ("write", SessionToolGroup::Files),
        ("lsp", SessionToolGroup::Search),
        ("edit", SessionToolGroup::Execute),
        ("notebook_edit", SessionToolGroup::Execute),
        ("fetch_webpage", SessionToolGroup::Web),
        ("screenshot", SessionToolGroup::Web),
        ("Agent", SessionToolGroup::Collaboration),
        ("cron_update", SessionToolGroup::Collaboration),
        ("todoread", SessionToolGroup::Collaboration),
    ];

    for (tool_id, expected) in cases {
        assert_eq!(tool_group(tool_id), expected);
    }
}

#[test]
fn tool_display_names_cover_known_runtime_surfaces() {
    let cases = [
        ("read", "读取文件"),
        ("pdf_read", "读取文档"),
        ("ls", "目录列表"),
        ("file_write", "写入文件"),
        ("apply_patch", "补丁编辑"),
        ("grep", "文本搜索"),
        ("content_search", "内容搜索"),
        ("glob", "文件匹配"),
        ("glob_search", "组合搜索"),
        ("codesearch", "代码检索"),
        ("shell", "命令行"),
        ("process", "后台进程"),
        ("git_operations", "Git 操作"),
        ("browser", "浏览器交互"),
        ("browser_open", "打开页面"),
        ("http_request", "网络请求"),
        ("web_fetch", "网页抓取"),
        ("web_search_tool", "联网搜索"),
        ("agent", "委托代理"),
        ("question", "用户提问"),
        ("skill", "加载技能"),
        ("memory_recall", "读取记忆"),
        ("memory_store", "写入记忆"),
        ("memory_forget", "删除记忆"),
    ];

    for (tool_id, expected) in cases {
        assert_eq!(tool_display_name(tool_id), expected);
    }
    assert_eq!(tool_display_name("__custom--tool__"), "Custom Tool");
    assert_eq!(tool_display_name("---"), "");
}
