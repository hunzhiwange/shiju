//! 预览标签栏的桌面布局投影。
pub(super) use desktop_editor::components::preview_panel::styles::{
    menu_button_style, small_icon_svg, file_tab_icon_svg, file_icon_for, truncate_title,
};
use desktop_editor::components::preview_panel::styles::{self, PreviewTabsLayout};
use crate::app::App;

fn layout(app: &App) -> PreviewTabsLayout {
    PreviewTabsLayout {
        window_width: app.window_size.0, show_file_manager: app.show_file_manager,
        file_manager_width: app.file_manager_width, show_diff: app.show_diff, split_ratio: app.split_ratio,
    }
}
#[cfg(test)]
pub(super) fn estimate_preview_tabs_viewport_width(app: &App) -> f32 {
    styles::estimate_preview_tabs_viewport_width(layout(app))
}
pub(super) fn should_show_preview_tabs_scrollbar(app: &App) -> bool {
    styles::should_show_preview_tabs_scrollbar(layout(app), app.preview_tabs.iter().map(|tab| tab.title.as_str()))
}
