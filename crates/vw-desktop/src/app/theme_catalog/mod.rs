//! 桌面主题兼容入口与 App 缓存接线。

pub use desktop_theme::theme_catalog::*;

pub fn sync_active_theme_skin(app: &mut crate::app::App) {
    desktop_theme::sync_active_theme_skin(
        app.appearance.active_custom_theme_id.as_deref(),
        &mut app.appearance.active_theme_skin,
    );
}

pub fn clear_active_theme_skin(app: &mut crate::app::App) {
    desktop_theme::clear_active_theme_skin(&mut app.appearance.active_theme_skin);
}
