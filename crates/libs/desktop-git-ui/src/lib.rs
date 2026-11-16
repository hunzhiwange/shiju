//! Git 差异视图与局部交互。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTheme { GitHub, Monokai }
pub mod git_panel;
pub mod code_review;
#[cfg(test)]
mod code_review_tests;

pub mod message_git;

pub mod modal_overlays;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_10_tests;

#[cfg(test)]
mod desktop_split_18_tests;

#[cfg(test)]
mod desktop_split_27_tests;

#[cfg(test)]
mod host_module_13_tests;

pub mod state;

pub mod page;

#[cfg(test)]
mod test_support;
