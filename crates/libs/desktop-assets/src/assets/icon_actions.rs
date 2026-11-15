use super::Icon;
use iced::widget::svg;
use std::collections::HashMap;

pub(super) fn register_icons(m: &mut HashMap<Icon, svg::Handle>) {
    // 保存操作图标
    m.insert(
        Icon::Save,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/save.svg")),
    );

    // 撤销操作图标（逆时针箭头）
    m.insert(
        Icon::ArrowCounterClockwise,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-counterclockwise.svg"
        )),
    );

    // 重做操作图标（顺时针箭头）
    m.insert(
        Icon::ArrowClockwise,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-clockwise.svg"
        )),
    );

    // 搜索图标
    m.insert(
        Icon::Search,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/search.svg"
        )),
    );

    // 机器人/小宠物图标
    m.insert(
        Icon::Robot,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/robot.svg")),
    );

    // 橡皮擦/清除图标
    m.insert(
        Icon::Eraser,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/eraser.svg"
        )),
    );

    // 刷新/循环图标
    m.insert(
        Icon::ArrowRepeat,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-repeat.svg"
        )),
    );

    // 关闭/取消图标
    m.insert(
        Icon::X,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/x-lg.svg")),
    );

    // 终端图标
    m.insert(
        Icon::Terminal,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/terminal.svg"
        )),
    );

    // 1x2 网格布局图标
    m.insert(
        Icon::Grid1x2,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/grid-1x2.svg"
        )),
    );

    // 多列布局图标
    m.insert(
        Icon::Columns,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/columns.svg"
        )),
    );

    // 水平对称操作图标
    m.insert(
        Icon::SymmetryHorizontal,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/symmetry-horizontal.svg"
        )),
    );

    // 垂直对称操作图标
    m.insert(
        Icon::SymmetryVertical,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/symmetry-vertical.svg"
        )),
    );

    // 向上箭头（折叠/收起）
    m.insert(
        Icon::ChevronUp,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/chevron-up.svg"
        )),
    );

    // 剪贴板图标
    m.insert(
        Icon::Clipboard,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/clipboard.svg"
        )),
    );

    // 勾选/完成图标
    m.insert(
        Icon::Check,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/check.svg")),
    );

    // 勾选正方形图标
    m.insert(
        Icon::CheckSquare,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/check-square.svg"
        )),
    );
}
#[cfg(test)]
#[path = "icon_actions_tests.rs"]
mod icon_actions_tests;
