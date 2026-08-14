use super::tools::{
    ToolGroup, capability_policy_label, preset_tool_count, tool_card_button_style,
    tool_english_name, tool_group_meta, tool_group_order, tool_in_preset, tool_matches_any,
    tool_meta, tool_policy_message, tool_preset_meta,
};
use crate::app::Message;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Color, Theme};
use vw_config_types::agent::DelegateCapabilityPolicy;

fn background_color(style: &button::Style) -> Color {
    match style.background.as_ref() {
        Some(Background::Color(color)) => *color,
        other => panic!("expected a solid background color, got {other:?}"),
    }
}

#[test]
fn tool_in_preset_rejects_unknown_preset() {
    assert!(!tool_in_preset("shell", "missing"));
    assert!(!tool_in_preset("", "minimal"));
    assert!(tool_in_preset("", "full"));
}

#[test]
fn tool_capability_policy_controls_use_typed_messages_and_expected_labels() {
    assert_eq!(capability_policy_label(DelegateCapabilityPolicy::Strict), "严格限制");
    assert_eq!(
        capability_policy_label(DelegateCapabilityPolicy::UserOverride),
        "限制但允许用户选择"
    );
    assert_eq!(capability_policy_label(DelegateCapabilityPolicy::Unrestricted), "不限制");
    assert!(matches!(
        tool_policy_message("main".to_string(), DelegateCapabilityPolicy::Unrestricted),
        Message::Settings(SettingsMessage::Agents(AgentsMessage::ToolPolicyChanged(
            ref agent,
            DelegateCapabilityPolicy::Unrestricted,
        ))) if agent == "main"
    ));
}

#[test]
fn tool_in_preset_accepts_declared_members_and_inheritance() {
    let minimal =
        ["ls", "read", "file_read", "pdf_read", "grep", "glob", "lsp", "memory_recall", "question"];
    let coding_only = ["write", "file_write", "apply_patch", "bash", "shell", "git_operations"];
    let research_only = [
        "browser",
        "browser_open",
        "http_request",
        "web_fetch",
        "web_search",
        "websearch",
        "web_search_tool",
        "screenshot",
        "image_info",
    ];
    let collaboration_only = [
        "AgentTool",
        "delegate_coordination_status",
        "schedule",
        "cron_add",
        "cron_list",
        "cron_remove",
        "cron_update",
        "cron_run",
        "cron_runs",
        "memory_store",
        "memory_forget",
        "todoread",
        "todowrite",
        "plan_enter",
        "plan_exit",
    ];

    for tool_id in minimal {
        for preset in ["minimal", "coding", "research", "collab", "full"] {
            assert!(tool_in_preset(tool_id, preset), "{tool_id} should be in {preset}");
        }
    }
    for tool_id in coding_only {
        assert!(tool_in_preset(tool_id, "coding"));
        assert!(tool_in_preset(tool_id, "full"));
        for preset in ["minimal", "research", "collab"] {
            assert!(!tool_in_preset(tool_id, preset), "{tool_id} should not be in {preset}");
        }
    }
    for tool_id in research_only {
        assert!(tool_in_preset(tool_id, "research"));
        assert!(tool_in_preset(tool_id, "full"));
        for preset in ["minimal", "coding", "collab"] {
            assert!(!tool_in_preset(tool_id, preset), "{tool_id} should not be in {preset}");
        }
    }
    for tool_id in collaboration_only {
        assert!(tool_in_preset(tool_id, "collab"));
        assert!(tool_in_preset(tool_id, "full"));
        for preset in ["minimal", "coding", "research"] {
            assert!(!tool_in_preset(tool_id, preset), "{tool_id} should not be in {preset}");
        }
    }

    assert!(!tool_in_preset("Read", "minimal"));
    assert!(!tool_in_preset("process", "coding"));
    assert!(tool_in_preset("totally_custom", "full"));
}

#[test]
fn preset_tool_count_counts_available_matches() {
    let tools = vec![
        "read".to_string(),
        "bash".to_string(),
        "web_search".to_string(),
        "unknown".to_string(),
    ];

    assert_eq!(preset_tool_count(&tools, "minimal"), 1);
    assert_eq!(preset_tool_count(&tools, "coding"), 2);
    assert_eq!(preset_tool_count(&tools, "research"), 2);
    assert_eq!(preset_tool_count(&tools, "full"), 4);
    assert_eq!(preset_tool_count(&tools, "missing"), 0);
}

#[test]
fn preset_tool_count_preserves_duplicate_entries_and_handles_empty_input() {
    let tools = vec!["read".to_string(), "read".to_string(), "custom".to_string()];

    assert_eq!(preset_tool_count(&tools, "minimal"), 2);
    assert_eq!(preset_tool_count(&tools, "coding"), 2);
    assert_eq!(preset_tool_count(&tools, "full"), 3);
    assert_eq!(preset_tool_count(&[], "full"), 0);
}

#[test]
fn tool_group_metadata_and_order_are_exact() {
    let expected = [
        (ToolGroup::Files, "文件", "读取、写入和修改工作区文件。"),
        (ToolGroup::Search, "搜索", "查找文件、文本和代码位置。"),
        (ToolGroup::Execute, "执行", "运行命令、补丁和版本控制操作。"),
        (ToolGroup::Web, "联网", "浏览页面、联网请求与外部检索。"),
        (ToolGroup::Collaboration, "协作", "委托子代理、调度计划和任务流转。"),
        (ToolGroup::Memory, "记忆", "读写长期记忆和上下文数据。"),
        (ToolGroup::Integration, "系统", "代理路由、集成能力与系统辅助工具。"),
        (ToolGroup::Other, "其他", "当前未归类的可调用工具。"),
    ];

    assert_eq!(tool_group_order(), expected.map(|(group, _, _)| group));
    for (group, label, description) in expected {
        let meta = tool_group_meta(group);
        assert_eq!(meta.group, group);
        assert_eq!(meta.label, label);
        assert_eq!(meta.description, description);
    }
}

#[test]
fn tool_preset_metadata_is_exact_and_ordered() {
    let expected = [
        ("minimal", "最小", "只保留只读分析能力。"),
        ("coding", "编码", "补齐读写、编辑、命令和 Git。"),
        ("research", "研究", "补齐浏览器、联网和页面取证。"),
        ("collab", "协作", "补齐委托、调度和记忆写入。"),
        ("full", "全部", "启用当前可见的全部工具。"),
    ];

    for (meta, (key, label, description)) in tool_preset_meta().into_iter().zip(expected) {
        assert_eq!(meta.key, key);
        assert_eq!(meta.label, label);
        assert_eq!(meta.description, description);
    }
}

#[test]
fn tool_meta_classifies_every_declared_tool_and_alias() {
    let cases = [
        ("read", "读取文件", ToolGroup::Files),
        ("file_read", "读取文件", ToolGroup::Files),
        ("pdf_read", "读取文档", ToolGroup::Files),
        ("ls", "目录列表", ToolGroup::Files),
        ("write", "写入文件", ToolGroup::Files),
        ("file_write", "写入文件", ToolGroup::Files),
        ("file_edit", "编辑文件", ToolGroup::Files),
        ("edit", "编辑文件", ToolGroup::Files),
        ("apply_patch", "补丁编辑", ToolGroup::Execute),
        ("grep", "文本搜索", ToolGroup::Search),
        ("content_search", "内容搜索", ToolGroup::Search),
        ("glob", "文件匹配", ToolGroup::Search),
        ("glob_search", "组合搜索", ToolGroup::Search),
        ("code_search", "代码检索", ToolGroup::Search),
        ("codesearch", "代码检索", ToolGroup::Search),
        ("bash", "命令行", ToolGroup::Execute),
        ("shell", "命令行", ToolGroup::Execute),
        ("process", "后台进程", ToolGroup::Execute),
        ("git_operations", "Git 操作", ToolGroup::Execute),
        ("browser", "浏览器交互", ToolGroup::Web),
        ("Browser", "浏览器交互", ToolGroup::Web),
        ("browser_open", "打开页面", ToolGroup::Web),
        ("BrowserOpen", "打开页面", ToolGroup::Web),
        ("http_request", "网络请求", ToolGroup::Web),
        ("web_fetch", "网页抓取", ToolGroup::Web),
        ("WebFetch", "网页抓取", ToolGroup::Web),
        ("webfetch", "网页抓取", ToolGroup::Web),
        ("web_search", "联网搜索", ToolGroup::Web),
        ("websearch", "联网搜索", ToolGroup::Web),
        ("web_search_tool", "联网搜索", ToolGroup::Web),
        ("WebSearch", "联网搜索", ToolGroup::Web),
        ("AgentTool", "委托子代理", ToolGroup::Collaboration),
        ("delegate", "委托执行", ToolGroup::Collaboration),
        ("delegate_coordination_status", "委托状态", ToolGroup::Collaboration),
        ("schedule", "任务调度", ToolGroup::Collaboration),
        ("cron_add", "新增定时任务", ToolGroup::Collaboration),
        ("cron_list", "定时任务列表", ToolGroup::Collaboration),
        ("cron_remove", "删除定时任务", ToolGroup::Collaboration),
        ("cron_update", "更新定时任务", ToolGroup::Collaboration),
        ("cron_run", "执行定时任务", ToolGroup::Collaboration),
        ("cron_runs", "运行历史", ToolGroup::Collaboration),
        ("todoread", "读取任务计划", ToolGroup::Collaboration),
        ("TodoRead", "读取任务计划", ToolGroup::Collaboration),
        ("todowrite", "写入任务计划", ToolGroup::Collaboration),
        ("TodoWrite", "写入任务计划", ToolGroup::Collaboration),
        ("plan_enter", "进入规划模式", ToolGroup::Collaboration),
        ("EnterPlanMode", "进入规划模式", ToolGroup::Collaboration),
        ("enter_plan_mode", "进入规划模式", ToolGroup::Collaboration),
        ("plan_exit", "退出规划模式", ToolGroup::Collaboration),
        ("ExitPlanMode", "退出规划模式", ToolGroup::Collaboration),
        ("exit_plan_mode", "退出规划模式", ToolGroup::Collaboration),
        ("enter_worktree", "进入 Worktree", ToolGroup::Execute),
        ("EnterWorktree", "进入 Worktree", ToolGroup::Execute),
        ("exit_worktree", "退出 Worktree", ToolGroup::Execute),
        ("ExitWorktree", "退出 Worktree", ToolGroup::Execute),
        ("question", "用户提问", ToolGroup::Collaboration),
        ("AskUserQuestion", "用户提问", ToolGroup::Collaboration),
        ("Brief", "发送简报", ToolGroup::Collaboration),
        ("Sleep", "短暂等待", ToolGroup::Collaboration),
        ("SendMessage", "发送 IPC 消息", ToolGroup::Integration),
        ("SendUserFile", "准备用户附件", ToolGroup::Collaboration),
        ("TeamCreate", "创建团队", ToolGroup::Integration),
        ("TeamDelete", "删除团队", ToolGroup::Integration),
        ("verify_plan_execution", "验证计划执行", ToolGroup::Collaboration),
        ("VerifyPlanExecution", "验证计划执行", ToolGroup::Collaboration),
        ("skill", "加载技能", ToolGroup::Collaboration),
        ("memory_recall", "读取记忆", ToolGroup::Memory),
        ("memory_store", "写入记忆", ToolGroup::Memory),
        ("memory_forget", "删除记忆", ToolGroup::Memory),
        ("proxy_config", "代理配置", ToolGroup::Integration),
        ("model_routing_config", "模型路由", ToolGroup::Integration),
        ("Config", "配置管理", ToolGroup::Integration),
        ("tool_search", "工具搜索", ToolGroup::Integration),
        ("ToolSearch", "工具搜索", ToolGroup::Integration),
        ("composio", "外部集成", ToolGroup::Integration),
        ("pushover", "推送通知", ToolGroup::Integration),
        ("screenshot", "屏幕截图", ToolGroup::Web),
        ("image_info", "图像信息", ToolGroup::Web),
        ("image_asset_read", "重载图像资源", ToolGroup::Web),
        ("lsp", "语义分析", ToolGroup::Integration),
        ("ListMcpResources", "MCP 资源列表", ToolGroup::Integration),
        ("ReadMcpResource", "读取 MCP 资源", ToolGroup::Integration),
        ("ListMcpResourceTemplates", "MCP 资源模板", ToolGroup::Integration),
        ("ListMcpPrompts", "MCP Prompt 列表", ToolGroup::Integration),
        ("GetMcpPrompt", "获取 MCP Prompt", ToolGroup::Integration),
        ("McpAuth", "MCP 认证信息", ToolGroup::Integration),
        ("McpStatus", "MCP 状态", ToolGroup::Integration),
        ("RemoteTrigger", "远端触发", ToolGroup::Integration),
        ("notebook_edit", "编辑笔记本", ToolGroup::Files),
        ("PowerShell", "PowerShell", ToolGroup::Execute),
        ("invalid", "无效调用", ToolGroup::Other),
        ("sop_execute", "启动 SOP", ToolGroup::Collaboration),
        ("sop_advance", "推进 SOP", ToolGroup::Collaboration),
        ("sop_approve", "审批 SOP", ToolGroup::Collaboration),
        ("sop_list", "SOP 列表", ToolGroup::Collaboration),
        ("sop_status", "SOP 状态", ToolGroup::Collaboration),
        ("agents_list", "代理列表", ToolGroup::Integration),
        ("agents_send", "发送代理消息", ToolGroup::Integration),
        ("agents_inbox", "代理收件箱", ToolGroup::Integration),
        ("state_get", "读取状态", ToolGroup::Integration),
        ("state_set", "写入状态", ToolGroup::Integration),
        ("batch", "批处理", ToolGroup::Integration),
        ("wasm_module", "WASM 模块", ToolGroup::Integration),
    ];

    for (tool_id, expected_name, expected_group) in cases {
        let meta = tool_meta(tool_id);
        assert_eq!(meta.name, expected_name, "unexpected name for {tool_id}");
        assert_eq!(meta.group, expected_group, "unexpected group for {tool_id}");
        assert!(!meta.description.is_empty(), "missing description for {tool_id}");
        assert!(
            !meta.description.contains("当前未接入专门说明"),
            "placeholder description left for {tool_id}"
        );
    }
}

#[test]
fn tool_meta_uses_explicit_fallback_for_unknown_or_case_mismatched_ids() {
    for tool_id in ["some_new_tool", "Read", "", "agenttool"] {
        let meta = tool_meta(tool_id);
        assert_eq!(meta.name, "未分类工具");
        assert_eq!(meta.group, ToolGroup::Other);
        assert!(!meta.description.is_empty());
    }
}

#[test]
fn tool_matching_is_exact_and_handles_empty_candidates() {
    assert!(tool_matches_any("read", &["write", "read"]));
    assert!(!tool_matches_any("read", &["write"]));
    assert!(!tool_matches_any("read", &[]));
    assert!(!tool_matches_any("read", &["Read", "reader"]));
    assert!(tool_matches_any("", &[""]));
}

#[test]
fn tool_english_name_handles_separator_case_and_empty_boundaries() {
    assert_eq!(tool_english_name("web_search-tool"), "Web Search Tool");
    assert_eq!(tool_english_name("__read--file__"), "Read File");
    assert_eq!(tool_english_name("alreadyTitle"), "AlreadyTitle");
    assert_eq!(tool_english_name("agent-HTTP_tool"), "Agent HTTP Tool");
    assert_eq!(tool_english_name("中文_工具"), "中文 工具");
    assert_eq!(tool_english_name("___---"), "");
    assert_eq!(tool_english_name(""), "");
}

#[test]
fn tool_card_button_style_uses_selected_palette_in_both_themes() {
    for (theme, expected_alpha) in [(Theme::Light, 0.10), (Theme::Dark, 0.24)] {
        let active = tool_card_button_style(&theme, button::Status::Active, true);
        let hovered = tool_card_button_style(&theme, button::Status::Hovered, true);
        let expected_background = theme.palette().primary.scale_alpha(expected_alpha);

        assert_eq!(background_color(&active), expected_background);
        assert_eq!(background_color(&hovered), expected_background);
        assert_eq!(active.border.color, theme.palette().primary.scale_alpha(0.28));
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.width, 1.0);
        assert_eq!(active.border.radius.top_left, 16.0);
    }
}

#[test]
fn tool_card_button_style_covers_unselected_light_states() {
    let theme = Theme::Light;
    let active = tool_card_button_style(&theme, button::Status::Active, false);
    let hovered = tool_card_button_style(&theme, button::Status::Hovered, false);
    let pressed = tool_card_button_style(&theme, button::Status::Pressed, false);
    let disabled = tool_card_button_style(&theme, button::Status::Disabled, false);

    assert_eq!(background_color(&active), Color::from_rgba8(255, 255, 255, 0.88));
    assert_eq!(background_color(&disabled), background_color(&active));
    assert_eq!(background_color(&hovered), Color::WHITE.scale_alpha(0.88));
    assert_eq!(
        background_color(&pressed),
        theme.extended_palette().background.weak.color.scale_alpha(0.92)
    );
    assert_eq!(active.border.color, Color::from_rgba8(15, 23, 42, 0.08));
}

#[test]
fn tool_card_button_style_covers_unselected_dark_states() {
    let theme = Theme::Dark;
    let active = tool_card_button_style(&theme, button::Status::Active, false);
    let hovered = tool_card_button_style(&theme, button::Status::Hovered, false);
    let pressed = tool_card_button_style(&theme, button::Status::Pressed, false);
    let disabled = tool_card_button_style(&theme, button::Status::Disabled, false);
    let palette = theme.extended_palette();

    assert_eq!(background_color(&active), palette.background.base.color.scale_alpha(0.92));
    assert_eq!(background_color(&disabled), background_color(&active));
    assert_eq!(background_color(&hovered), palette.background.weak.color.scale_alpha(0.30));
    assert_eq!(background_color(&pressed), palette.background.strong.color.scale_alpha(0.82));
    assert_eq!(active.border.color, palette.background.strong.color.scale_alpha(0.78));
}

#[test]
fn tools_view_distinguishes_empty_and_all_grouped_tool_lists() {
    let (mut app, _) = crate::app::App::new();
    let mut entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools.clear();

    let empty_tree = {
        let empty = super::tools::view(&app, &entry);
        Tree::new(empty.as_widget())
    };
    assert_eq!(empty_tree.children.len(), 3);
    assert!(empty_tree.children[2].children.is_empty());

    app.agents_settings.available_tools = [
        "read",
        "grep",
        "bash",
        "web_search",
        "AgentTool",
        "memory_store",
        "proxy_config",
        "custom_tool",
    ]
    .map(str::to_string)
    .to_vec();
    entry.allowed_tools = vec!["read".to_string(), "custom_tool".to_string()];

    let grouped = super::tools::view(&app, &entry);
    let grouped_tree = Tree::new(grouped.as_widget());
    let tools_list = &grouped_tree.children[2];

    // tabs + select-all + preset section + presets + optional groups
    assert_eq!(tools_list.children.len(), 5);
    assert_eq!(tools_list.children[0].children.len(), 2);
    assert_eq!(tools_list.children[1].children.len(), 3);
    assert_eq!(tools_list.children[3].children.len(), 5);
    // optional only: read(Files), proxy_config(Integration), custom_tool(Other)
    assert_eq!(tools_list.children[4].children.len(), 3);
}

#[test]
fn tools_view_keeps_zero_match_presets_visible_without_grouping_them() {
    let (mut app, _) = crate::app::App::new();
    let entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools = vec!["custom_tool".to_string()];

    let view = super::tools::view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[2];

    assert_eq!(tools_list.children.len(), 5);
    assert_eq!(tools_list.children[3].children.len(), 5);
    assert_eq!(tools_list.children[4].children.len(), 1);
    assert_eq!(tools_list.children[4].children[0].children[2].children.len(), 1);
}

#[test]
fn tools_view_preserves_duplicate_available_tools_and_selection_state() {
    let (mut app, _) = crate::app::App::new();
    let mut entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools =
        vec!["read".to_string(), "read".to_string(), "write".to_string()];
    entry.allowed_tools = vec!["read".to_string(), "ghost_tool".to_string()];

    let view = super::tools::view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[2];
    let file_cards = &tools_list.children[4].children[0].children[2];

    assert_eq!(tools_list.children[1].children.len(), 3);
    assert_eq!(file_cards.children.len(), 3);
}

#[test]
fn tools_view_splits_optional_and_builtin_tabs() {
    use crate::app::state::{AGENT_TOOLS_BUILTIN_TAB, AGENT_TOOLS_OPTIONAL_TAB};
    use vw_config_types::tools::is_builtin_required_tool_id;

    let (mut app, _) = crate::app::App::new();
    let entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools = vec![
        "bash".to_string(),
        "custom_optional".to_string(),
        "AgentTool".to_string(),
        "proxy_config".to_string(),
    ];

    assert_eq!(app.agents_settings.active_tools_list_tab, AGENT_TOOLS_OPTIONAL_TAB);

    let optional_ids = super::tools::tools_for_list_tab(
        &app.agents_settings.available_tools,
        &app.agents_settings.active_tools_list_tab,
    );
    assert_eq!(optional_ids, vec!["custom_optional", "proxy_config"]);
    assert!(optional_ids.iter().all(|id| !is_builtin_required_tool_id(id)));

    {
        let optional_view = super::tools::view(&app, &entry);
        let optional_tree = Tree::new(optional_view.as_widget());
        let optional_list = &optional_tree.children[2];
        // tabs + select-all row + preset section + preset buttons + grouped cards
        assert!(optional_list.children.len() >= 4, "optional tab should keep preset controls");
        assert_eq!(optional_list.children[0].children.len(), 2, "optional/builtin tab buttons");
    }

    app.agents_settings.active_tools_list_tab = AGENT_TOOLS_BUILTIN_TAB.to_string();
    let builtin_ids = super::tools::tools_for_list_tab(
        &app.agents_settings.available_tools,
        &app.agents_settings.active_tools_list_tab,
    );
    assert_eq!(builtin_ids, vec!["bash", "AgentTool"]);
    assert!(builtin_ids.iter().all(|id| is_builtin_required_tool_id(id)));

    let builtin_view = super::tools::view(&app, &entry);
    let builtin_tree = Tree::new(builtin_view.as_widget());
    let builtin_list = &builtin_tree.children[2];
    assert_eq!(
        builtin_list.children.len(),
        2,
        "builtin tab should only keep tabs row and grouped cards (no presets)"
    );
}

#[test]
fn builtin_tool_cards_are_locked_and_badged() {
    use crate::app::Message;
    use crate::app::message::settings::{AgentsMessage, SettingsMessage};
    use crate::app::state::AGENT_TOOLS_BUILTIN_TAB;

    let (mut app, _) = crate::app::App::new();
    let mut entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools = vec!["bash".to_string(), "custom_optional".to_string()];
    app.agents_settings.active_tools_list_tab = AGENT_TOOLS_BUILTIN_TAB.to_string();
    entry.allowed_tools = vec!["bash".to_string()];

    assert!(super::tools::tool_card_is_locked("bash"));
    assert!(!super::tools::tool_card_is_locked("custom_optional"));
    assert_eq!(super::tools::builtin_tool_badge_label(), "内置");

    assert!(
        super::tools::tool_card_press_message("main".to_string(), "bash".to_string(), true, true)
            .is_none(),
        "locked builtin cards must not emit cancel toggle"
    );
    let optional_press = super::tools::tool_card_press_message(
        "main".to_string(),
        "custom_optional".to_string(),
        false,
        false,
    );
    assert!(matches!(
        optional_press,
        Some(Message::Settings(SettingsMessage::Agents(
            AgentsMessage::AllowedToolToggled(ref agent, ref tool, true)
        ))) if agent == "main" && tool == "custom_optional"
    ));

    let view = super::tools::view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[2];
    // tabs + groups; first group card title row includes badge child
    let groups = &tools_list.children[1];
    let first_group_cards = &groups.children[0].children[2];
    assert_eq!(first_group_cards.children.len(), 1);
    let card_content = &first_group_cards.children[0].children[0];
    let title_row = &card_content.children[0];
    assert_eq!(title_row.children.len(), 2, "builtin card title should include name + 内置 badge");
}
