#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 桌面终端支持启动的交互式 Shell。
pub enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    pub fn all() -> [Shell; 2] {
        [Shell::Bash, Shell::Zsh]
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::Bash
    }
}

impl From<Shell> for &'static str {
    fn from(shell: Shell) -> Self {
        match shell {
            Shell::Bash => "/bin/bash",
            Shell::Zsh => "/bin/zsh",
        }
    }
}
