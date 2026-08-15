use super::Icon;
use iced::widget::svg;
use std::collections::HashMap;

pub(super) fn register_icons(m: &mut HashMap<Icon, svg::Handle>) {
    // 光标/指针图标
    m.insert(
        Icon::Cursor,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/cursor.svg"
        )),
    );

    // 四向调整箭头图标
    m.insert(
        Icon::Arrows,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrows.svg"
        )),
    );

    // 移动箭头图标
    m.insert(
        Icon::ArrowsMove,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrows-move.svg"
        )),
    );

    // 全屏箭头图标
    m.insert(
        Icon::ArrowsFullscreen,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrows-fullscreen.svg"
        )),
    );

    // 向上箭头图标
    m.insert(
        Icon::ArrowUp,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-up.svg"
        )),
    );

    // 向下箭头图标
    m.insert(
        Icon::ArrowDown,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/arrow-down.svg"
        )),
    );

    // 无序列表图标
    m.insert(
        Icon::ListUl,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/list-ul.svg"
        )),
    );

    // 贝塞尔曲线图标
    m.insert(
        Icon::Bezier,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/bezier.svg"
        )),
    );

    // 链接图标
    m.insert(
        Icon::Link,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/link.svg")),
    );

    // 45度倾斜链接图标
    m.insert(
        Icon::Link45Deg,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/link-45deg.svg"
        )),
    );

    // 向右箭头（展开/导航）
    m.insert(
        Icon::ChevronRight,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/chevron-right.svg"
        )),
    );

    // 向左箭头（折叠/返回）
    m.insert(
        Icon::ChevronLeft,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/chevron-left.svg"
        )),
    );

    // 向下箭头（展开/下拉）
    m.insert(
        Icon::ChevronDown,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/chevron-down.svg"
        )),
    );

    // 垃圾桶/删除图标
    m.insert(
        Icon::Trash,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/trash.svg")),
    );

    // 正方形图标
    m.insert(
        Icon::Square,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/square.svg"
        )),
    );

    // 半选正方形图标
    m.insert(
        Icon::SquareHalf,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/square-half.svg"
        )),
    );

    // 加号/新增图标
    m.insert(
        Icon::Plus,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/plus.svg")),
    );

    // 铅笔/编辑图标
    m.insert(
        Icon::Pencil,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/pencil.svg"
        )),
    );

    // 圆形图标
    m.insert(
        Icon::Circle,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/circle.svg"
        )),
    );

    // 星形图标
    m.insert(
        Icon::Star,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/star.svg")),
    );

    // 三角形图标
    m.insert(
        Icon::Triangle,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/triangle.svg"
        )),
    );

    // 菱形图标
    m.insert(
        Icon::Diamond,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/diamond.svg"
        )),
    );

    // 五边形图标
    m.insert(
        Icon::Pentagon,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/pentagon.svg"
        )),
    );

    // 六边形图标
    m.insert(
        Icon::Hexagon,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/hexagon.svg"
        )),
    );

    // 胶囊图标
    m.insert(
        Icon::Capsule,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/capsule.svg"
        )),
    );

    // 平行四边形图标（复用四边形）
    m.insert(
        Icon::Parallelogram,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/square.svg"
        )),
    );

    // 梯形图标（复用四边形）
    m.insert(
        Icon::Trapezoid,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/square.svg"
        )),
    );

    // 速度计/性能图标
    m.insert(
        Icon::Speedometer2,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/speedometer2.svg"
        )),
    );

    // 图片图标
    m.insert(
        Icon::Image,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/image.svg")),
    );

    // 云下载图标
    m.insert(
        Icon::CloudDownload,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/cloud-download.svg"
        )),
    );

    // 笔/绘图图标
    m.insert(
        Icon::Pen,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/pen.svg")),
    );

    // 油漆桶/填充图标
    m.insert(
        Icon::PaintBucket,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/paint-bucket.svg"
        )),
    );

    // 文字/字体图标
    m.insert(
        Icon::Type,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/fonts.svg")),
    );

    // 文本窗口布局图标
    m.insert(
        Icon::LayoutTextWindow,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/layout-text-window.svg"
        )),
    );

    // 全屏图标
    m.insert(
        Icon::Fullscreen,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/fullscreen.svg"
        )),
    );

    // 退出全屏图标
    m.insert(
        Icon::FullscreenExit,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/fullscreen-exit.svg"
        )),
    );

    // 文本文件图标
    m.insert(
        Icon::FileText,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/file-text.svg"
        )),
    );

    // 手指索引/选择图标
    m.insert(
        Icon::HandIndex,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/hand-index.svg"
        )),
    );

    // 滑块/设置图标
    m.insert(
        Icon::Sliders,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/sliders.svg"
        )),
    );

    // 调色板图标
    m.insert(
        Icon::Palette,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/palette.svg"
        )),
    );

    // 键盘图标
    m.insert(
        Icon::Keyboard,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/keyboard.svg"
        )),
    );

    // 齿轮/设置图标
    m.insert(
        Icon::Gear,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/gear.svg")),
    );

    // 扳手/工具图标
    m.insert(
        Icon::Wrench,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/wrench.svg"
        )),
    );

    // 宽齿轮连接图标
    m.insert(
        Icon::GearWideConnected,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/gear-wide-connected.svg"
        )),
    );

    // 网关/网络存储图标
    m.insert(
        Icon::HddNetwork,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/hdd-network.svg"
        )),
    );

    // 花括号/代码块图标
    m.insert(
        Icon::Braces,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/braces.svg"
        )),
    );

    // 盒子/组件图标
    m.insert(
        Icon::Box,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/box.svg")),
    );

    // 文件图标
    m.insert(
        Icon::File,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/file.svg")),
    );

    // 打开的文件夹图标
    m.insert(
        Icon::FolderOpen,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/folder2-open.svg"
        )),
    );

    // 文件标记加号/新建文件图标
    m.insert(
        Icon::FileEarmarkPlus,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/file-earmark-plus.svg"
        )),
    );

    // 聊天文本填充图标
    m.insert(
        Icon::ChatTextFill,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/chat-text-fill.svg"
        )),
    );

    // 日志/笔记本图标
    m.insert(
        Icon::Journals,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/journals.svg"
        )),
    );

    // 云上传图标
    m.insert(
        Icon::CloudUpload,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/cloud-upload.svg"
        )),
    );
}
#[cfg(test)]
#[path = "icon_canvas_tests.rs"]
mod icon_canvas_tests;
