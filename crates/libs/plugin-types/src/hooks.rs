//! 已审计插件钩子的共享数据契约。
use anyhow::{Result, bail};
use std::path::PathBuf;

/// 运行时支持的插件钩子生命周期事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginHookEvent {
    BeforeToolCall,
    PermissionRequest,
    AfterToolCall,
    BeforeLlmCall,
    PostModelRequest,
    SubagentStart,
    SubagentStop,
    Stop,
    Interrupt,
    PreCompact,
    PostCompact,
    UserPromptSubmit,
    InstructionsLoaded,
    SessionStart,
    SessionEnd,
}

impl PluginHookEvent {
    /// 返回清单与信任摘要使用的稳定事件标识。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PermissionRequest => "permission_request",
            Self::BeforeToolCall => "before_tool_call",
            Self::AfterToolCall => "after_tool_call",
            Self::BeforeLlmCall => "before_llm_call",
            Self::SubagentStart => "subagent_start",
            Self::Stop => "stop",
            Self::Interrupt => "interrupt",
            Self::SubagentStop => "subagent_stop",
            Self::PreCompact => "pre_compact",
            Self::PostCompact => "post_compact",
            Self::PostModelRequest => "post_model_request",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::InstructionsLoaded => "instructions_loaded",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
        }
    }
}

/// 插件钩子声明的执行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginHookMode {
    Blocking,
}

impl PluginHookMode {
    pub fn parse(raw: Option<String>) -> Result<Self> {
        let Some(raw) = raw else {
            return Ok(Self::Blocking);
        };
        match raw.trim() {
            "" | "blocking" => Ok(Self::Blocking),
            other => bail!("unsupported plugin hook mode: {other}"),
        }
    }

    /// 返回清单使用的稳定模式标识。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
        }
    }
}

/// 已通过模式解析与静态路径审计的钩子声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHook {
    pub event: PluginHookEvent,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_secs: Option<u64>,
    pub working_dir: PathBuf,
    pub mode: PluginHookMode,
    pub manifest_path: PathBuf,
}
