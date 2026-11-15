//! Git 差异视图与局部交互。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTheme { GitHub, Monokai }
pub mod git_panel;
pub mod code_review;
#[cfg(test)]
mod code_review_tests;

pub mod message_git;

pub mod modal_overlays;
