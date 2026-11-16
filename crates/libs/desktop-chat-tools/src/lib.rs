//! 聊天工具元数据、选择器、只读文本与高级卡片。
pub mod tool_names;
pub mod tools;
pub mod path;

#[cfg(test)]
mod tool_names_tests;
pub mod advanced_input;
pub mod card_style;
pub mod selector_input;
pub mod selector_style;
pub mod state;
pub mod text_layout;
pub mod text_safety;
pub mod tool_presentation;
pub mod tool_selector;
pub mod tool_text_support;
#[cfg(test)]
mod tool_text_support_tests;
#[cfg(test)]
mod tool_selector_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod tool_presentation_tests;

pub mod card_input;

#[cfg(test)]
mod desktop_split_06_tests;

#[cfg(test)]
mod desktop_split_05_tests;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_22_tests;

#[cfg(test)]
mod host_module_39_locale_tests;
