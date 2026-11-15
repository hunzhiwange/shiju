//! 验证代理工具权限、分组展示与选择消息的边界。
//! 未知工具、继承权限和显式覆盖用于约束权限展示的优先级。

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
        ["ls", "read", "file_read", "pdf_read", "grep", "glob", "memory_recall", "question"];
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
    ];
    let collaboration_only = [
        "AgentTool",
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

    let removed_tool_id = ["image", "info"].join("_");
    for preset in ["minimal", "coding", "research", "collab"] {
        assert!(!tool_in_preset(&removed_tool_id, preset));
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
        ("create_thread", "创建任务", ToolGroup::Collaboration),
        ("fork_thread", "分叉任务", ToolGroup::Collaboration),
        ("list_threads", "列出任务", ToolGroup::Collaboration),
        ("read_thread", "读取任务", ToolGroup::Collaboration),
        ("send_message_to_thread", "续发任务消息", ToolGroup::Collaboration),
        ("handoff_thread", "交接任务", ToolGroup::Collaboration),
        ("set_thread_title", "修改任务标题", ToolGroup::Collaboration),
        ("set_thread_pinned", "置顶任务", ToolGroup::Collaboration),
        ("set_thread_archived", "归档任务", ToolGroup::Collaboration),
        ("wait_threads", "等待任务", ToolGroup::Collaboration),
        ("delegate", "委托执行", ToolGroup::Collaboration),
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
        ("enter_worktree", "进入 Worktree", ToolGroup::Execute),
        ("EnterWorktree", "进入 Worktree", ToolGroup::Execute),
        ("exit_worktree", "退出 Worktree", ToolGroup::Execute),
        ("ExitWorktree", "退出 Worktree", ToolGroup::Execute),
        ("question", "用户提问", ToolGroup::Collaboration),
        ("AskUserQuestion", "用户提问", ToolGroup::Collaboration),
        ("Sleep", "短暂等待", ToolGroup::Collaboration),
        ("skill", "加载技能", ToolGroup::Collaboration),
        ("memory_recall", "读取记忆", ToolGroup::Memory),
        ("memory_store", "写入记忆", ToolGroup::Memory),
        ("memory_forget", "删除记忆", ToolGroup::Memory),
        ("Config", "配置管理", ToolGroup::Integration),
        ("screenshot", "屏幕截图", ToolGroup::Web),
        ("image_asset_read", "重载图像资源", ToolGroup::Web),
        ("lsp", "语义分析", ToolGroup::Integration),
        ("McpStatus", "MCP 状态", ToolGroup::Integration),
        ("RemoteTrigger", "远端触发", ToolGroup::Integration),
        ("notebook_edit", "编辑笔记本", ToolGroup::Files),
        ("PowerShell", "PowerShell", ToolGroup::Execute),
        ("invalid", "无效调用", ToolGroup::Other),
        ("batch", "批处理", ToolGroup::Integration),
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
