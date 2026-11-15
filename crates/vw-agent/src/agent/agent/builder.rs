//! 原构建器路径的兼容出口。
pub type AgentBuilder = agent_loop::agent::AgentBuilder<crate::agent::loop_::core::skills::NativeCoreSkillHost>;
