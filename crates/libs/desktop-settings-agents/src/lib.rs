//! 智能体设置的领域状态、局部消息、视图和配置校验。
pub mod state;
pub mod acp_state;
pub mod message;
pub mod defaults;
pub mod acp;
pub mod identity;
pub mod basic;
pub mod basic_profile;
pub mod catalog;
pub mod mcp;
pub mod acp_view;
pub mod acp_diagnostic;
pub mod shared;
pub mod help;
pub mod tools;
pub mod skills;
pub mod profile_modal;
#[cfg(test)] mod tools_tests;
#[cfg(test)] mod shared_tests;
#[cfg(test)] mod skills_tests;

pub mod autonomy_inputs;
pub mod autonomy_message;
pub mod autonomy_help;
pub mod system_settings_autonomy;
pub mod dialogue_flow_inputs;
pub mod dialogue_flow_message;
pub mod dialogue_flow_help;
pub mod system_settings_dialogue_flow;
pub mod acp_update;
pub mod acp_update_message;

pub mod agents_state;
pub mod agents_update;
pub mod agents_update_message;
pub mod autonomy_update;

pub mod dialogue_flow_update;
pub mod agents_view;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_03_tests;

#[cfg(test)]
mod desktop_split_12_tests;

#[cfg(test)]
mod desktop_split_17_tests;
