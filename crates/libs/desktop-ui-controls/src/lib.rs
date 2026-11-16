pub mod animated_text;
pub mod color_picker;
pub mod system_settings_common;
pub mod number_input;
pub mod overlays;
pub mod status_animation;

#[cfg(test)]
#[path = "status_animation_tests.rs"]
mod status_animation_tests;
pub mod svg_path;
pub mod single_line_label;

pub mod text_editor_context_menu;
pub mod text_editor_scroll_panel;
pub mod right_click_area;
#[cfg(test)] mod text_editor_context_menu_tests;

pub mod toast;
#[cfg(test)]
mod right_click_area_tests;

pub mod tooltip;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_18_tests;

pub mod icons;
