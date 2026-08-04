use super::Icon;
use iced::widget::svg;
use std::collections::HashMap;

pub(super) fn register_icons(m: &mut HashMap<Icon, svg::Handle>) {
    m.insert(
        Icon::ArrowLeft,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-left.svg"
        )),
    );
    m.insert(
        Icon::ArrowRight,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-right.svg"
        )),
    );
    m.insert(
        Icon::Globe,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/globe.svg")),
    );

    // 帮助/问号圆圈图标
    m.insert(
        Icon::QuestionCircle,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/question-circle.svg"
        )),
    );

    // 返回图标
    m.insert(
        Icon::Back,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/back.svg")),
    );

    // 主页图标
    m.insert(
        Icon::Home,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/house.svg")),
    );

    // Git 分支图标
    m.insert(
        Icon::GitBranch,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/git.svg")),
    );

    // 时钟/历史记录图标
    m.insert(
        Icon::Clock,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/clock.svg")),
    );

    // 安全权限图标
    m.insert(
        Icon::ShieldLock,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/shield-lock.svg"
        )),
    );

    // 文字格式 - 粗体
    m.insert(
        Icon::TypeBold,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/type-bold.svg"
        )),
    );

    // 文字格式 - 斜体
    m.insert(
        Icon::TypeItalic,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/type-italic.svg"
        )),
    );

    // 文字格式 - 下划线
    m.insert(
        Icon::TypeUnderline,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/type-underline.svg"
        )),
    );

    // 文字格式 - 删除线
    m.insert(
        Icon::TypeStrikethrough,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/type-strikethrough.svg"
        )),
    );

    // 文字对齐 - 左对齐
    m.insert(
        Icon::TextLeft,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/text-left.svg"
        )),
    );

    // 文字对齐 - 居中
    m.insert(
        Icon::TextCenter,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/text-center.svg"
        )),
    );

    // 文字对齐 - 右对齐
    m.insert(
        Icon::TextRight,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/text-right.svg"
        )),
    );

    // 垂直对齐 - 顶部对齐
    m.insert(
        Icon::AlignTop,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/align-top.svg"
        )),
    );

    // 垂直对齐 - 垂直居中
    m.insert(
        Icon::AlignMiddle,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/align-middle.svg"
        )),
    );

    // 垂直对齐 - 底部对齐
    m.insert(
        Icon::AlignBottom,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/align-bottom.svg"
        )),
    );

    // 边框样式图标
    m.insert(
        Icon::BorderStyle,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/border-style.svg"
        )),
    );

    // 代码图标
    m.insert(
        Icon::Code,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/code.svg")),
    );

    // 侧边栏布局图标
    m.insert(
        Icon::LayoutSidebar,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/layout-sidebar.svg"
        )),
    );

    // 反向侧边栏布局图标
    m.insert(
        Icon::LayoutSidebarReverse,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/layout-sidebar-reverse.svg"
        )),
    );

    // 漏斗/过滤图标
    m.insert(
        Icon::Funnel,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/funnel.svg"
        )),
    );

    // 显示/可见图标
    m.insert(
        Icon::Eye,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/eye.svg")),
    );

    // 隐藏/不可见图标
    m.insert(
        Icon::EyeSlash,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/eye-slash.svg"
        )),
    );
}
#[cfg(test)]
#[path = "icon_ui_tests.rs"]
mod icon_ui_tests;
