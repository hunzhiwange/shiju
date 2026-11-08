//! 系统设置通用组件模块。
//!
//! 本模块仅作为稳定导出入口，内部按职责拆分为多个子模块，
//! 以降低单文件体积，同时保持既有外部接口不变。

mod help;
mod icons;
mod panels;
mod select;
mod styles;
mod tabs;
mod theme;
mod utils;

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
#[cfg(test)]
#[path = "icons_tests.rs"]
mod icons_tests;
#[cfg(test)]
#[path = "panels_tests.rs"]
mod panels_tests;
#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(in crate::app) mod tab_appearance_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;

pub use help::{
    settings_close_button, settings_help_button, settings_help_button_with_locale,
    settings_help_copy_state, with_settings_help_modal,
};
pub use icons::{icon_btn, icon_svg, provider_logo_svg};
pub(super) use panels::{
    settings_flexible_row, settings_row_layout, settings_row_with_flexible_control,
};
pub use panels::{
    settings_control_with_hint, settings_divider, settings_error_banner, settings_group_card,
    settings_group_heading, settings_modal_card, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_page_heading, settings_page_intro, settings_panel,
    settings_row, settings_section_card, settings_section_content, settings_success_banner,
    settings_value_badge,
};
pub(crate) use panels::{
    settings_modal_layout_with_scroll_id, settings_modal_overlay_with_backdrop_dismiss,
    settings_modal_overlay_with_escape,
};
pub(crate) use select::SettingsSelectLabel;
pub use select::settings_select;
pub use styles::{
    danger_action_btn_style, primary_action_btn_style, round_icon_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_modal_backdrop_style,
    settings_modal_card_style, settings_muted_text_style, settings_panel_style,
    settings_pick_list_menu_style, settings_pick_list_style, settings_segment_button_style,
    settings_text_editor_style, settings_text_editor_style_for, settings_text_input_style,
};
pub use tabs::{settings_tab_button, settings_tab_row, settings_tab_style};
pub use utils::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH, bool_support_label,
    format_context_limit, settings_field_label, settings_toggle, url_encode,
};

#[cfg(test)]
#[path = "mod_ui_refresh_01_tests.rs"]
mod ui_refresh_01_tests;
