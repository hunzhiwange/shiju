use super::*;
use crate::views::project::layout::work_sidebar;

#[test]
fn work_sidebar_navigation_matches_reference_grouping_and_icons() {
    assert_eq!(
        work_sidebar::navigation_specs(),
        [
            ("新建任务", Icon::FileEarmarkPlus, None),
            ("助手", Icon::Robot, None),
            ("项目", Icon::FolderOpen, None),
            ("识矩应用", Icon::Grid1x2, None),
            ("识矩技能", Icon::Star, None),
            ("专家 · 技能 · 连接器", Icon::GearWideConnected, None),
            ("定时任务", Icon::ArrowRepeat, None),
            ("识矩知识库", Icon::Journals, None),
            ("更多", Icon::Grid1x2, Some("知识库 · 应用")),
        ]
    );
    assert_eq!(
        work_sidebar::header_icon_specs(false),
        [
            (Icon::LayoutSidebar, "收起侧边栏"),
            (Icon::ArrowRepeat, "刷新全部任务"),
            (Icon::Grid1x2, "任务池"),
            (Icon::Search, "搜索"),
            (Icon::Funnel, "过滤"),
        ]
    );
    assert_eq!(
        work_sidebar::header_icon_specs(true)[0],
        (Icon::LayoutSidebarReverse, "展开侧边栏")
    );
    assert_eq!(
        work_sidebar::more_menu_item_specs(),
        [
            ("添加网址", Icon::Link45Deg),
            ("知识库", Icon::Journals),
            ("工作流", Icon::GitBranch),
            ("应用中心", Icon::Grid1x2),
        ]
    );
}

#[test]
fn work_sidebar_surface_is_pale_in_light_theme_and_dark_in_dark_theme() {
    let surface = |theme: &Theme| match work_sidebar::sidebar_style(theme).background {
        Some(iced::Background::Color(color)) => color,
        _ => panic!("work sidebar should have a background"),
    };
    let luminance = |color: iced::Color| 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;

    assert!(luminance(surface(&Theme::Light)) > 0.90);
    assert!(luminance(surface(&Theme::Dark)) < 0.50);
}
