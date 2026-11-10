use super::Icon;
use iced::widget::image;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// PNG 图片缓存映射表
///
/// 存储非 SVG 格式的图片资源（如 PNG）。
/// 某些应用程序图标需要使用 PNG 格式以保持原始外观。
///
/// # 性能
///
/// 与 `ICONS` 相同，使用延迟加载和内存缓存机制。
static IMAGES: Lazy<HashMap<Icon, image::Handle>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // 应用程序 Logo
    m.insert(
        Icon::Logo,
        image::Handle::from_bytes(include_bytes!("../../../../../assets/logo.png").as_slice()),
    );

    // Finder 应用图标（macOS 文件管理器）
    m.insert(
        Icon::AppFinder,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/finder.png").as_slice(),
        ),
    );

    // Terminal 应用图标（macOS 终端）
    m.insert(
        Icon::AppTerminal,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/terminal.png").as_slice(),
        ),
    );

    // TextMate 应用图标
    m.insert(
        Icon::AppTextMate,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/textmate.png").as_slice(),
        ),
    );

    // Xcode 应用图标
    m.insert(
        Icon::AppXcode,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/xcode.png").as_slice(),
        ),
    );

    // Windsurf 应用图标
    m.insert(
        Icon::AppWindsurf,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/windsurf.png").as_slice(),
        ),
    );

    m.insert(
        Icon::AppHermes,
        image::Handle::from_bytes(
            include_bytes!("../../../../../assets/icons/app/hermes.png").as_slice(),
        ),
    );

    m
});

/// 获取指定的 PNG 图片句柄
///
/// 从缓存中获取图片，如果图片不存在则 panic。
///
/// # 参数
///
/// - `icon`: 要获取的图片对应的图标类型
///
/// # 返回值
///
/// 返回对应的图片句柄，可用于在 iced UI 中显示
///
/// # Panic
///
/// 如果请求的图片未在 `IMAGES` 映射表中定义，将触发 panic。
///
/// # 示例
///
/// ```ignore
/// use crate::app::assets::{get_image, Icon};
///
/// let logo = get_image(Icon::Logo);
/// // 在 UI 中使用 logo
/// ```
pub fn get_image(icon: Icon) -> image::Handle {
    IMAGES.get(&icon).cloned().expect("Image missing in assets map")
}
#[cfg(test)]
#[path = "images_tests.rs"]
mod images_tests;
