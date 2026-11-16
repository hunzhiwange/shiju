//! 从宿主迁入的通道行为测试；外部宿主接线仍由宿主测试覆盖。
use crate::manager::runtime_command::{parse_runtime_command, ChannelRuntimeCommand};
use crate::manager::health::classify_health_result;
use crate::manager::errors::is_context_window_overflow_error;
use crate::ChannelHealthState;
use channel_support::prompt::should_skip_memory_context_entry;
use model_types::ChatMessage;
use tool_types::{Tool, ToolResult};
use std::time::Duration;
use tempfile::TempDir;
const BOOTSTRAP_MAX_CHARS: usize = 20_000;
fn channel_system_prompt(
    workspace: &std::path::Path, model: &str, tools: &[(&str, &str)],
    skills: &[agent_skills::Skill], identity: Option<&agent_config::runtime_schema::IdentityConfig>,
    max: Option<usize>, timezone: chrono_tz::Tz,
) -> String {
    channel_support::prompt::build_system_prompt_with_mode(workspace, model, tools, skills, identity, max,
        false, agent_config::runtime_schema::SkillsPromptInjectionMode::Full, timezone, "")
}
fn build_runtime_tool_visibility_prompt(tools: &[Box<dyn Tool>], excluded: &[String], native: bool) -> String {
    channel_support::prompt::build_runtime_tool_visibility_prompt(tools, excluded, native,
        agent_loop::instructions::build_tool_instructions_from_specs)
}
fn make_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("SOUL.md"), "# Soul\nBe helpful.").unwrap();
    std::fs::write(tmp.path().join("IDENTITY.md"), "# Identity\nName: VibeWindow").unwrap();
    std::fs::write(tmp.path().join("USER.md"), "# User\nName: Test User").unwrap();
    std::fs::write(tmp.path().join("AGENTS.md"), "# Agents\nFollow instructions.").unwrap();
    std::fs::write(tmp.path().join("TOOLS.md"), "# Tools\nUse shell carefully.").unwrap();
    std::fs::write(tmp.path().join("HEARTBEAT.md"), "# Heartbeat\nCheck status.").unwrap();
    std::fs::write(tmp.path().join("MEMORY.md"), "# Memory\nUser likes Rust.").unwrap();
    tmp
}


// 原宿主 cfg(test) 夹具，不属于生产算法。
pub(crate) fn normalize_cached_channel_turns(turns: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut normalized = Vec::with_capacity(turns.len());
    // 状态标志：true 表示期望用户消息，false 表示期望助手消息
    let mut expecting_user = true;

    for turn in turns {
        match (expecting_user, turn.role.as_str()) {
            // 正常情况：收到期望的用户消息
            (true, "user") => {
                normalized.push(turn);
                expecting_user = false; // 下次期望助手消息
            }
            // 正常情况：收到期望的助手消息
            (false, "assistant") => {
                normalized.push(turn);
                expecting_user = true; // 下次期望用户消息
            }
            // 中断的通道轮次可能产生连续的用户消息（助手尚未持久化）
            // 采用合并策略而非丢弃，以保留完整的用户输入上下文
            (false, "user") | (true, "assistant") => {
                if let Some(last_turn) = normalized.last_mut() {
                    if !turn.content.is_empty() {
                        if !last_turn.content.is_empty() {
                            // 用双换行符分隔合并的内容，保持可读性
                            last_turn.content.push_str("\n\n");
                        }
                        last_turn.content.push_str(&turn.content);
                    }
                }
            }
            // 忽略其他角色（如 system）的消息
            _ => {}
        }
    }

    normalized
}


mod history_management;
mod session_history;
mod system_prompt;
mod command_parsing;
mod vision;
mod identity;
mod health_supervisor;
mod config_loading;
mod tool_execution;
mod routing;
mod approval_commands;
