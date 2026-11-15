use super::{Icon, icon_actions, icon_apps, icon_canvas, icon_files, icon_ui};
use iced::widget::svg;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// SVG 图标缓存映射表
///
/// 使用 `Lazy` 延迟初始化，在首次访问时从编译时嵌入的 SVG 文件加载所有图标。
/// 所有图标数据通过 `include_bytes!` 宏在编译时嵌入二进制文件。
///
/// # 性能
///
/// - 延迟加载：仅在首次调用 `get_icon` 时初始化
/// - 内存缓存：图标句柄缓存在 HashMap 中，避免重复解析 SVG
/// - 零运行时 IO：所有资源在编译时嵌入
static ICONS: Lazy<HashMap<Icon, svg::Handle>> = Lazy::new(|| {
    let mut m = HashMap::new();
    icon_actions::register_icons(&mut m);
    icon_ui::register_icons(&mut m);
    icon_canvas::register_icons(&mut m);
    icon_files::register_icons(&mut m);
    icon_apps::register_icons(&mut m);
    m
});

/// 获取指定的 SVG 图标句柄
///
/// 从缓存中获取图标，如果图标不存在则 panic。
///
/// # 参数
///
/// - `icon`: 要获取的图标类型
///
/// # 返回值
///
/// 返回对应的 SVG 图标句柄，可用于在 iced UI 中显示
///
/// # Panic
///
/// 如果请求的图标未在 `ICONS` 映射表中定义，将触发 panic。
/// 这通常表示代码与资源映射表不同步。
///
/// # 示例
///
/// ```ignore
/// use desktop_assets::assets::{get_icon, Icon};
///
/// let save_icon = get_icon(Icon::Save);
/// // 在 UI 中使用 save_icon
/// ```
pub fn get_icon(icon: Icon) -> svg::Handle {
    ICONS.get(&icon).cloned().expect("Icon missing in assets map")
}
#[cfg(test)]
#[path = "svg_icons_tests.rs"]
mod svg_icons_tests;
