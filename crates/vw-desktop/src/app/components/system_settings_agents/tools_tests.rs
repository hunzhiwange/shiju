use super::tools::{
    ToolGroup, preset_tool_count, tool_card_button_style, tool_english_name, tool_group_meta,
    tool_group_order, tool_in_preset, tool_matches_any, tool_meta, tool_preset_meta,
};
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Color, Theme};

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
        ("browser_open", "打开页面", ToolGroup::Web),
        ("http_request", "网络请求", ToolGroup::Web),
        ("web_fetch", "网页抓取", ToolGroup::Web),
        ("web_search", "联网搜索", ToolGroup::Web),
        ("websearch", "联网搜索", ToolGroup::Web),
        ("web_search_tool", "联网搜索", ToolGroup::Web),
        ("AgentTool", "AgentTool", ToolGroup::Collaboration),
        ("delegate_coordination_status", "委托状态", ToolGroup::Collaboration),
        ("schedule", "任务调度", ToolGroup::Collaboration),
        ("cron_add", "新增定时任务", ToolGroup::Collaboration),
        ("cron_list", "定时任务列表", ToolGroup::Collaboration),
        ("cron_remove", "删除定时任务", ToolGroup::Collaboration),
        ("cron_update", "更新定时任务", ToolGroup::Collaboration),
        ("cron_run", "执行定时任务", ToolGroup::Collaboration),
        ("cron_runs", "运行历史", ToolGroup::Collaboration),
        ("todoread", "读取任务计划", ToolGroup::Collaboration),
        ("todowrite", "写入任务计划", ToolGroup::Collaboration),
        ("plan_enter", "进入规划模式", ToolGroup::Collaboration),
        ("plan_exit", "退出规划模式", ToolGroup::Collaboration),
        ("question", "用户提问", ToolGroup::Collaboration),
        ("skill", "加载技能", ToolGroup::Collaboration),
        ("memory_recall", "读取记忆", ToolGroup::Memory),
        ("memory_store", "写入记忆", ToolGroup::Memory),
        ("memory_forget", "删除记忆", ToolGroup::Memory),
        ("proxy_config", "代理配置", ToolGroup::Integration),
        ("model_routing_config", "模型路由", ToolGroup::Integration),
        ("composio", "外部集成", ToolGroup::Integration),
        ("pushover", "推送通知", ToolGroup::Integration),
        ("screenshot", "屏幕截图", ToolGroup::Web),
        ("image_info", "图像信息", ToolGroup::Web),
        ("lsp", "语义分析", ToolGroup::Integration),
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

    let empty_tree = {
        let empty = super::tools::view(&app, &entry);
        Tree::new(empty.as_widget())
    };
    assert_eq!(empty_tree.children.len(), 2);
    assert!(empty_tree.children[1].children.is_empty());

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
    let tools_list = &grouped_tree.children[1];

    assert_eq!(tools_list.children.len(), 4);
    assert_eq!(tools_list.children[0].children.len(), 3);
    assert_eq!(tools_list.children[2].children.len(), 5);
    assert_eq!(tools_list.children[3].children.len(), 8);
}

#[test]
fn tools_view_keeps_zero_match_presets_visible_without_grouping_them() {
    let (mut app, _) = crate::app::App::new();
    let entry = app.agents_settings.entries[0].clone();
    app.agents_settings.available_tools = vec!["custom_tool".to_string()];

    let view = super::tools::view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[1];

    assert_eq!(tools_list.children.len(), 4);
    assert_eq!(tools_list.children[2].children.len(), 5);
    assert_eq!(tools_list.children[3].children.len(), 1);
    assert_eq!(tools_list.children[3].children[0].children[2].children.len(), 1);
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
    let tools_list = &tree.children[1];
    let file_cards = &tools_list.children[3].children[0].children[2];

    assert_eq!(tools_list.children[0].children.len(), 3);
    assert_eq!(file_cards.children.len(), 3);
}
