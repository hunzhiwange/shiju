//! 任务看板的局部视图、消息与操作。
pub mod draft;
pub mod pet_state;
pub mod pet_view;
pub mod task_pet;
pub mod model_selector;
pub mod bulk;
pub mod selector_styles;
pub mod helpers;
pub mod message;
pub mod update;
pub mod common;
pub mod board;

pub mod control;

pub mod drag;
pub mod modals;
pub mod panel;

#[cfg(test)]
mod slice_view_fixture;

pub mod page;

pub mod pet_interaction;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_20_tests;

#[cfg(test)]
mod desktop_split_30_tests;

#[cfg(test)]
mod host_module_10_tests;

#[cfg(test)]
mod host_module_15_tests;
