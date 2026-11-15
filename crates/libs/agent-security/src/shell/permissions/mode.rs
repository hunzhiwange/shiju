//! Shell 权限模式定义，负责描述普通、接受编辑和自动接受模式的自动放行范围。

/// PermissionMode 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    #[default]
    Normal,
    AcceptEdits,
    AutoAccept,
}

const ACCEPT_EDITS_COMMANDS: &[&str] =
    &["mkdir", "touch", "rm", "rmdir", "mv", "cp", "chmod", "chown"];

impl PermissionMode {
    /// 执行 auto_allowed_commands 操作，并返回调用方需要的结果。
    pub fn auto_allowed_commands(&self) -> &'static [&'static str] {
        match self {
            Self::AcceptEdits => ACCEPT_EDITS_COMMANDS,
            Self::Normal | Self::AutoAccept => &[],
        }
    }

    /// 执行 auto_allows_command 操作，并返回调用方需要的结果。
    pub fn auto_allows_command(&self, command_name: &str) -> bool {
        match self {
            Self::AutoAccept => true,
            _ => self.auto_allowed_commands().contains(&command_name),
        }
    }
}
