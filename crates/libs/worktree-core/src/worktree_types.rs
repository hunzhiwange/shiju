use serde::{Deserialize, Serialize};

/// Worktree 操作错误类型
///
/// 定义所有可能的 worktree 操作失败场景
#[derive(Debug)]
pub enum Error {
    /// 非 Git 项目错误
    ///
    /// 当前项目未使用 Git 作为版本控制系统
    NotGit(String),

    /// 项目信息缺失错误
    ///
    /// 必需的项目上下文信息（如项目 ID、worktree 路径等）不存在
    MissingProject(String),

    /// 无效操作错误
    ///
    /// 操作参数不合法或操作本身不被允许
    Invalid(String),

    /// I/O 错误
    ///
    /// 文件系统操作失败
    Io(std::io::Error),

    /// 异步任务连接错误
    ///
    /// 仅在非 WASM 平台上可用，表示 tokio 任务执行失败
    #[cfg(not(target_arch = "wasm32"))]
    Join(tokio::task::JoinError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotGit(e) => write!(f, "{}", e),
            Error::MissingProject(e) => write!(f, "{}", e),
            Error::Invalid(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            #[cfg(not(target_arch = "wasm32"))]
            Error::Join(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<tokio::task::JoinError> for Error {
    fn from(value: tokio::task::JoinError) -> Self {
        Error::Join(value)
    }
}

/// Worktree 信息结构
///
/// 描述一个 worktree 的基本元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    /// Worktree 名称（同时也是目录名和分支后缀）
    pub name: String,

    /// Git 分支名（完整格式，如 `shiju/brave-eagle`）
    pub branch: String,

    /// Worktree 目录的绝对路径
    pub directory: String,
}

/// 创建 worktree 的输入参数
///
/// 用于自定义 worktree 创建行为
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateInput {
    /// 自定义 worktree 名称
    ///
    /// 如果未提供，将自动生成随机名称
    pub name: Option<String>,

    /// worktree 创建后执行的启动命令
    ///
    /// 在项目级 start 命令之后执行
    #[serde(rename = "startCommand")]
    pub start_command: Option<String>,
}

/// 删除 worktree 的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveInput {
    /// 要删除的 worktree 目录路径
    pub directory: String,

    /// 是否强制删除
    ///
    /// 即使有未提交的更改也会删除
    #[serde(default)]
    pub force: bool,
}

/// 重置 worktree 的输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetInput {
    /// 要重置的 worktree 目录路径
    pub directory: String,

    /// 基准引用（分支、标签或提交）
    ///
    /// 如果未提供，将使用项目的默认分支（main/master 或远程 HEAD）
    #[serde(rename = "baseRef")]
    pub base_ref: Option<String>,
}
