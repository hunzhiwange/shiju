use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// 以斜杠开头的消息可调用的内置命令集合。
/// Commands that can be invoked by starting a message with a leading slash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // 禁止按字母排序：枚举顺序就是弹窗展示顺序，高频命令应排在前面。
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Ide,
    Permissions,
    Keymap,
    Vim,
    #[strum(serialize = "setup-default-sandbox")]
    ElevateSandbox,
    #[strum(serialize = "sandbox-add-read-dir")]
    SandboxReadRoot,
    Experimental,
    #[strum(to_string = "approve")]
    AutoReview,
    Memories,
    Skills,
    Import,
    Hooks,
    Review,
    Rename,
    New,
    Archive,
    Delete,
    Resume,
    Fork,
    App,
    Init,
    Compact,
    Plan,
    Goal,
    Agents,
    Side,
    Btw,
    Copy,
    Export,
    Raw,
    Diff,
    Mention,
    Status,
    Cd,
    #[strum(to_string = "pwd", serialize = "cwd")]
    Pwd,
    Usage,
    DebugConfig,
    Title,
    Statusline,
    Theme,
    #[strum(to_string = "pets", serialize = "pet")]
    Pets,
    Mcp,
    Apps,
    Plugins,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    #[strum(to_string = "stop", serialize = "clean")]
    Stop,
    Clear,
    Personality,
    TestApproval,
    #[strum(serialize = "subagents")]
    MultiAgents,
    // 调试命令不应作为普通用户工作流展示。
    // Debugging commands.
    #[strum(serialize = "debug-m-drop")]
    MemoryDrop,
    #[strum(serialize = "debug-m-update")]
    MemoryUpdate,
}

impl SlashCommand {
    /// 弹窗中展示给用户的命令说明。
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "send logs to maintainers",
            SlashCommand::New => "start a new chat during a conversation",
            SlashCommand::Init => "create an AGENTS.md file with instructions for Codex",
            SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
            SlashCommand::Review => "review my current changes and find issues",
            SlashCommand::Rename => "rename the current thread",
            SlashCommand::Resume => "resume a saved chat",
            SlashCommand::Archive => "archive this session and exit",
            SlashCommand::Delete => "permanently delete this session and exit",
            SlashCommand::Clear => "clear the terminal and start a new chat",
            SlashCommand::Fork => "fork the current chat",
            SlashCommand::App => "continue this session in the Desktop app",
            SlashCommand::Quit | SlashCommand::Exit => "exit Codex",
            SlashCommand::Copy => "copy the last response, code block, or quote",
            SlashCommand::Export => "export the conversation as markdown",
            SlashCommand::Raw => "toggle raw scrollback mode for copy-friendly terminal selection",
            SlashCommand::Diff => "show git diff (including untracked files)",
            SlashCommand::Mention => "mention a file",
            SlashCommand::Skills => "use skills to improve how Codex performs specific tasks",
            SlashCommand::Import => "import setup, this project, and recent chats from Claude Code",
            SlashCommand::Hooks => "view and manage lifecycle hooks",
            SlashCommand::Status => "show current session configuration and token usage",
            SlashCommand::Cd => "change the current working directory",
            SlashCommand::Pwd => "show the current working directory",
            SlashCommand::Usage => "view account usage or use a usage limit reset",
            SlashCommand::DebugConfig => "show config layers and requirement sources for debugging",
            SlashCommand::Title => "configure which items appear in the terminal title",
            SlashCommand::Statusline => "configure which items appear in the status line",
            SlashCommand::Theme => "choose a syntax highlighting theme",
            SlashCommand::Pets => "choose or hide the terminal pet",
            SlashCommand::Ps => "list background terminals",
            SlashCommand::Stop => "stop all background terminals",
            SlashCommand::MemoryDrop => "DO NOT USE",
            SlashCommand::MemoryUpdate => "DO NOT USE",
            SlashCommand::Model => "choose what model and reasoning effort to use",
            SlashCommand::Ide => {
                "include current selection, open files, and other context from your IDE"
            }
            SlashCommand::Personality => "choose a communication style for Codex",
            SlashCommand::Plan => "switch to Plan mode",
            SlashCommand::Goal => "set or view the goal for a long-running task",
            SlashCommand::Agents => "view and switch between all active agent sessions",
            SlashCommand::MultiAgents => "switch between this session's subagents",
            SlashCommand::Side | SlashCommand::Btw => {
                "start a side conversation in an ephemeral fork"
            }
            SlashCommand::Permissions => "choose what Codex is allowed to do",
            SlashCommand::Keymap => "remap TUI shortcuts",
            SlashCommand::Vim => "toggle Vim mode for the composer",
            SlashCommand::ElevateSandbox => "set up elevated agent sandbox",
            SlashCommand::SandboxReadRoot => {
                "let sandbox read a directory: /sandbox-add-read-dir <absolute_path>"
            }
            SlashCommand::Experimental => "toggle experimental features",
            SlashCommand::AutoReview => "approve one retry of a recent auto-review denial",
            SlashCommand::Memories => "configure memory use and generation",
            SlashCommand::Mcp => "list configured MCP tools; use /mcp verbose for details",
            SlashCommand::Apps => "manage apps",
            SlashCommand::Plugins => "browse plugins",
            SlashCommand::Logout => "log out of Codex",
            SlashCommand::Rollout => "print the rollout file path",
            SlashCommand::TestApproval => "test approval request",
        }
    }

    /// 返回不含前导 `/` 的命令字符串；保留 `command()` 名称以兼容既有调用方。
    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// 指示命令是否支持行内参数，例如 `/review ...`。
    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review
                | SlashCommand::Rename
                | SlashCommand::New
                | SlashCommand::Clear
                | SlashCommand::Fork
                | SlashCommand::Plan
                | SlashCommand::Goal
                | SlashCommand::Ide
                | SlashCommand::Keymap
                | SlashCommand::Mcp
                | SlashCommand::Export
                | SlashCommand::Raw
                | SlashCommand::Cd
                | SlashCommand::Pwd
                | SlashCommand::Usage
                | SlashCommand::Pets
                | SlashCommand::Side
                | SlashCommand::Btw
                | SlashCommand::Resume
                | SlashCommand::SandboxReadRoot
        )
    }

    /// 指示命令在活动侧会话中是否仍可使用。
    /// Whether this command remains available inside an active side conversation.
    pub fn available_in_side_conversation(self) -> bool {
        matches!(
            self,
            SlashCommand::Copy
                | SlashCommand::Agents
                | SlashCommand::Export
                | SlashCommand::Raw
                | SlashCommand::Diff
                | SlashCommand::Mention
                | SlashCommand::Status
                | SlashCommand::Pwd
                | SlashCommand::Usage
                | SlashCommand::Ide
        )
    }

    /// 指示命令在任务执行期间是否可运行。
    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Archive
            | SlashCommand::Delete
            | SlashCommand::Fork
            | SlashCommand::Init
            | SlashCommand::Compact
            | SlashCommand::Export
            | SlashCommand::Keymap
            | SlashCommand::Vim
            | SlashCommand::ElevateSandbox
            | SlashCommand::SandboxReadRoot
            | SlashCommand::Experimental
            | SlashCommand::Memories
            | SlashCommand::Import
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Cd
            | SlashCommand::Clear
            | SlashCommand::Logout
            | SlashCommand::MemoryDrop
            | SlashCommand::MemoryUpdate => false,
            SlashCommand::Diff
            | SlashCommand::Resume
            | SlashCommand::Model
            | SlashCommand::Personality
            | SlashCommand::Permissions
            | SlashCommand::Copy
            | SlashCommand::Raw
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Hooks
            | SlashCommand::Status
            | SlashCommand::Pwd
            | SlashCommand::Usage
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Stop
            | SlashCommand::App
            | SlashCommand::Goal
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Plugins
            | SlashCommand::Title
            | SlashCommand::Statusline
            | SlashCommand::AutoReview
            | SlashCommand::Feedback
            | SlashCommand::Ide
            | SlashCommand::Quit
            | SlashCommand::Exit
            | SlashCommand::Side
            | SlashCommand::Btw => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Agents | SlashCommand::MultiAgents => true,
            SlashCommand::Theme | SlashCommand::Pets => false,
        }
    }

    fn is_visible(self) -> bool {
        // 平台专用和调试命令在构建期条件不满足时从候选列表中隐藏。
        match self {
            SlashCommand::SandboxReadRoot => cfg!(target_os = "windows"),
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::App => cfg!(any(target_os = "macos", target_os = "windows")),
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// 返回所有可见内置命令及其命令字符串的配对列表。
/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::SlashCommand;

    #[test]
    // `/stop` 是对外规范名称。
    fn stop_command_is_canonical_name() {
        assert_eq!(SlashCommand::Stop.command(), "stop");
    }

    #[test]
    // 旧 `/clean` 名称仅作为解析别名映射到 `/stop`。
    fn clean_alias_parses_to_stop_command() {
        assert_eq!(SlashCommand::from_str("clean"), Ok(SlashCommand::Stop));
    }

    #[test]
    // `/pets` 保持规范复数名，同时接受旧单数别名。
    fn pet_alias_parses_to_pets_command() {
        assert_eq!(SlashCommand::Pets.command(), "pets");
        assert_eq!(SlashCommand::from_str("pet"), Ok(SlashCommand::Pets));
    }

    #[test]
    // 任务进行中允许的命令还应保持各自的侧会话和参数能力声明。
    fn certain_commands_are_available_during_task() {
        assert!(SlashCommand::Goal.available_during_task());
        assert!(SlashCommand::Ide.available_during_task());
        assert!(SlashCommand::Title.available_during_task());
        assert!(SlashCommand::Statusline.available_during_task());
        assert!(SlashCommand::Raw.available_during_task());
        assert!(SlashCommand::Raw.available_in_side_conversation());
        assert!(SlashCommand::Raw.supports_inline_args());
        assert!(SlashCommand::App.available_during_task());
    }

    #[test]
    // 自动审查重试对外使用 `/approve` 名称并可由该名称解析。
    fn auto_review_command_is_approve() {
        assert_eq!(SlashCommand::AutoReview.command(), "approve");
        assert_eq!(
            SlashCommand::from_str("approve"),
            Ok(SlashCommand::AutoReview)
        );
    }
}
//! 定义斜杠命令的展示顺序、名称别名、可见性与运行时可用条件。
