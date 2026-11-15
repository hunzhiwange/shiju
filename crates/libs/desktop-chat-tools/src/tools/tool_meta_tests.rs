/// 重新导出 use super::{tool_emoji, tool_header_label, tool_icon, tool_inline_summary, tool_verb}，让上层模块通过稳定路径访问。
use super::{
    ToolVisualState, ToolVisualTone, tool_emoji, tool_header_label, tool_icon, tool_icon_badge,
    tool_icon_badge_color, tool_icon_badge_size,
    tool_verb, tool_visual_state,
};
/// 重新导出 use desktop_assets::assets::Icon，让上层模块通过稳定路径访问。
use desktop_assets::assets::Icon;
use iced::Theme;

#[test]
fn tool_icons_distinguish_common_activity_types() {
    for (tool_name, expected) in [
        ("read", Icon::FileText),
        ("file_write", Icon::Pencil),
        ("bash", Icon::Terminal),
        ("grep", Icon::Search),
        ("web_fetch", Icon::Globe),
        ("browser", Icon::Link),
        ("agent", Icon::Robot),
        ("mcp_linear", Icon::Box),
        ("task", Icon::Clipboard),
        ("unknown_tool", Icon::Wrench),
    ] {
        assert_eq!(tool_icon(tool_name), expected, "{tool_name}");
    }
}

#[test]
fn tool_visual_state_maps_all_supported_lifecycles() {
    for (raw, state, icon, label, is_animated, tone) in [
        ("waiting", ToolVisualState::Waiting, Icon::Clock, "等待中", true, ToolVisualTone::Waiting),
        ("queued", ToolVisualState::Waiting, Icon::Clock, "等待中", true, ToolVisualTone::Waiting),
        (
            "running",
            ToolVisualState::Running,
            Icon::ArrowRepeat,
            "运行中",
            true,
            ToolVisualTone::Accent,
        ),
        (
            "completed",
            ToolVisualState::Completed,
            Icon::Check,
            "已完成",
            false,
            ToolVisualTone::Success,
        ),
        (
            "success",
            ToolVisualState::Completed,
            Icon::Check,
            "已完成",
            false,
            ToolVisualTone::Success,
        ),
        ("error", ToolVisualState::Failed, Icon::X, "失败", false, ToolVisualTone::Danger),
        ("failed", ToolVisualState::Failed, Icon::X, "失败", false, ToolVisualTone::Danger),
        ("denied", ToolVisualState::Failed, Icon::X, "失败", false, ToolVisualTone::Danger),
        ("cancelled", ToolVisualState::Cancelled, Icon::X, "已取消", false, ToolVisualTone::Muted),
    ] {
        let visual = tool_visual_state(raw);
        assert_eq!(visual.state, state, "{raw}");
        assert_eq!(visual.icon, icon, "{raw}");
        assert_eq!(visual.label, label, "{raw}");
        assert_eq!(visual.is_animated, is_animated, "{raw}");
        assert_eq!(visual.tone, tone, "{raw}");
    }

    for theme in [Theme::Dark, Theme::Light] {
        assert_eq!(
            tool_visual_state("running").color(&theme),
            theme.extended_palette().primary.base.color
        );
        assert_eq!(
            tool_visual_state("failed").color(&theme),
            theme.extended_palette().danger.base.color
        );
    }

    assert_eq!(
        tool_visual_state("waiting").color(&Theme::Dark),
        iced::Color::from_rgb8(0xD6, 0xA8, 0x43)
    );
    assert_eq!(
        tool_visual_state("waiting").color(&Theme::Light),
        iced::Color::from_rgb8(0x97, 0x68, 0x14)
    );
    assert_ne!(
        tool_visual_state("completed").color(&Theme::Dark),
        tool_visual_state("completed").color(&Theme::Light)
    );
    assert_ne!(
        tool_visual_state("cancelled").color(&Theme::Dark),
        tool_visual_state("cancelled").color(&Theme::Light)
    );

    let running_badge = tool_icon_badge::<()>("bash", tool_visual_state("running"));
    std::hint::black_box(running_badge);
}

#[test]
fn file_tool_icons_use_activity_semantics() {
    assert_eq!(tool_icon("read"), Icon::FileText);
    assert_eq!(tool_icon("file_write"), Icon::Pencil);
}

/// 生成 file tool icons use regular badge size，用于工具卡片或状态行的简短说明。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn file_tool_icons_use_regular_badge_size() {
    assert_eq!(tool_icon_badge_size("read"), 14.0);
    assert_eq!(tool_icon_badge_size("file_write"), 14.0);
    assert_eq!(tool_icon_badge_size("bash"), 14.0);
}

#[test]
fn aliases_share_tool_metadata() {
    assert_eq!(tool_emoji("read_file"), "📖");
    assert_eq!(tool_icon("grep_search"), Icon::Search);
    assert_eq!(tool_verb("apply_patch"), "编辑");
    assert_eq!(tool_header_label("browser_open"), "浏览");
}

#[test]
fn alias_mappings_cover_file_search_web_browser_cron_and_mcp_names() {
    for name in ["file_read", "pdf_read", "read_file"] {
        assert_eq!(tool_emoji(name), "📖", "{name}");
        assert_eq!(tool_icon(name), Icon::FileText, "{name}");
        assert_eq!(tool_verb(name), "读取", "{name}");
    }

    for name in ["file_write", "file_edit", "notebook_edit"] {
        assert_eq!(tool_emoji(name), "👁️", "{name}");
        assert_eq!(tool_icon(name), Icon::Pencil, "{name}");
    }
    assert_eq!(tool_verb("file_write"), "写入");
    assert_eq!(tool_verb("notebook_edit"), "编辑");

    for name in [
        "codesearch",
        "glob_search",
        "content_search",
        "searchcodebase",
        "file_search",
        "semantic_search",
        "github_repo",
        "copilot_getnotebooksummary",
        "vscode_listcodeusages",
    ] {
        assert_eq!(tool_emoji(name), "🔍", "{name}");
        assert_eq!(tool_icon(name), Icon::Search, "{name}");
        assert_eq!(tool_verb(name), "搜索", "{name}");
    }

    for name in ["webfetch", "http_request", "fetch_webpage"] {
        assert_eq!(tool_emoji(name), "🌐", "{name}");
        assert_eq!(tool_icon(name), Icon::Globe, "{name}");
        assert_eq!(tool_verb(name), "请求", "{name}");
    }

    for name in ["browser_open", "open_browser_page"] {
        assert_eq!(tool_emoji(name), "🖥️", "{name}");
        assert_eq!(tool_icon(name), Icon::Link, "{name}");
        assert_eq!(tool_verb(name), "浏览", "{name}");
    }

    for name in ["cron_add", "cron_list", "cron_run", "cron_update", "cron_remove"] {
        assert_eq!(tool_emoji(name), "⏰", "{name}");
        assert_eq!(tool_icon(name), Icon::Clock, "{name}");
        assert_eq!(tool_verb(name), "调度", "{name}");
    }

    for name in ["list_mcp_resources", "list_mcp_resource_templates", "read_mcp_resource"] {
        assert_eq!(tool_icon(name), Icon::Box, "{name}");
        assert_eq!(tool_verb(name), "MCP", "{name}");
    }
}

#[test]
fn emoji_mappings_cover_specialized_tools_and_fallback() {
    for (tool_name, expected) in [
        ("config", "⚙️"),
        ("read", "📖"),
        ("write", "👁️"),
        ("apply_patch", "📝"),
        ("lsp", "🔍"),
        ("bash", "⚡"),
        ("ls", "📁"),
        ("web_fetch", "🌐"),
        ("browser", "🖥️"),
        ("skill", "🎯"),
        ("task", "📋"),
        ("question", "❓"),
        ("screenshot", "📸"),
        ("agent", "🛰️"),
        ("todowrite", "✅"),
        ("schedule", "⏰"),
        ("memory_store", "🧠"),
        ("git_diff", "🔀"),
        ("view_image", "🖼️"),
        ("get_errors", "🧪"),
        ("get_changed_files", "🔀"),
        ("process", "💻"),
        ("workflow_node", "🔁"),
        ("unknown_tool", "🔧"),
    ] {
        assert_eq!(tool_emoji(tool_name), expected, "{tool_name}");
    }
}

#[test]
fn icon_mappings_cover_specialized_tools_mcp_and_fallback() {
    for (tool_name, expected) in [
        ("config", Icon::Sliders),
        ("read", Icon::FileText),
        ("write", Icon::Pencil),
        ("apply_patch", Icon::Pencil),
        ("lsp", Icon::Search),
        ("bash", Icon::Terminal),
        ("todowrite", Icon::Clipboard),
        ("ls", Icon::FolderOpen),
        ("web_fetch", Icon::Globe),
        ("browser", Icon::Link),
        ("skill", Icon::Star),
        ("task", Icon::Clipboard),
        ("agent", Icon::Robot),
        ("screenshot", Icon::Image),
        ("get_errors", Icon::Code),
        ("schedule", Icon::Clock),
        ("memory_recall", Icon::Box),
        ("git_diff", Icon::GitBranch),
        ("view_image", Icon::Image),
        ("workflow_node", Icon::Grid1x2),
        ("mcp_linear", Icon::Box),
        ("unknown_tool", Icon::Wrench),
    ] {
        assert_eq!(tool_icon(tool_name), expected, "{tool_name}");
    }
}

#[test]
fn badge_colors_distinguish_error_compact_and_regular_tools() {
    let theme = Theme::Dark;

    assert_eq!(
        tool_icon_badge_color(&theme, false, tool_visual_state("failed")),
        theme.extended_palette().danger.base.color
    );
    assert_eq!(
        tool_icon_badge_color(&theme, true, tool_visual_state("")),
        super::chat_secondary_subtle_text_color(&theme)
    );
    assert_eq!(
        tool_icon_badge_color(&theme, false, tool_visual_state("")),
        super::chat_secondary_text_color(&theme)
    );
}

#[test]
fn header_components_construct_for_skill_file_and_error_states() {
    assert_eq!(tool_header_label("skill"), "技能");

    let skill_badge = tool_icon_badge::<()>("skill", tool_visual_state(""));
    let file_error_badge = tool_icon_badge::<()>("read", tool_visual_state("failed"));
    std::hint::black_box((skill_badge, file_error_badge));
}
