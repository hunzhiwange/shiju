//! 会话生命周期的输入、作用域和分类错误。

use std::{fmt, io};
use std::path::{Path, PathBuf};
use vw_api_transport::id::TenantScope;
use permission_types as permission_next;
use session_info as session;
use message_types as message;

/// 已验证的原生会话访问作用域。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionScope {
    tenant: TenantScope,
    directory: PathBuf,
}

impl SessionScope {
    /// 从租户和现有绝对目录构造作用域，并保存目录的规范化形式。
    pub fn new(tenant: TenantScope, directory: impl AsRef<Path>) -> Result<Self, LifecycleError> {
        let directory = directory.as_ref().to_path_buf();
        if !directory.is_absolute() {
            return Err(LifecycleError::DirectoryMustBeAbsolute { directory });
        }

        let canonical = std::fs::canonicalize(&directory).map_err(|source| {
            LifecycleError::DirectoryUnavailable { directory: directory.clone(), source }
        })?;
        let metadata = std::fs::metadata(&canonical).map_err(|source| {
            LifecycleError::DirectoryUnavailable { directory: canonical.clone(), source }
        })?;
        if !metadata.is_dir() {
            return Err(LifecycleError::DirectoryNotDirectory { directory: canonical });
        }

        Ok(Self { tenant, directory: canonical })
    }

    /// 返回当前作用域的租户。
    pub fn tenant(&self) -> &TenantScope {
        &self.tenant
    }

    /// 返回规范化后的绝对工作目录。
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

/// 创建原生会话所需的完整输入。
#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    /// 已验证的租户和目录作用域。
    pub scope: SessionScope,
    /// 可选父会话 ID；父会话必须属于同一作用域。
    pub parent_id: Option<String>,
    /// 可选标题；为空时沿用原生默认标题。
    pub title: Option<String>,
    /// 可选原生权限规则。
    pub permission: Option<permission_next::Ruleset>,
}

/// 已完成归属校验的原生会话快照。
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    /// 原生会话元数据，保留原始 ID、slug 与项目 ID。
    pub info: session::Info,
    /// 校验归属后读取的完整原生消息历史。
    pub history: Vec<message::WithParts>,
    /// Saved specifications only; current authenticated host must rebind executors and grants.
    #[cfg(not(target_arch = "wasm32"))]
    pub dynamic_tools: Option<crate::PersistedDynamicTools>,
}

/// 会话生命周期边界的分类错误。
#[derive(Debug)]
pub enum LifecycleError {
    /// 调用方传入了相对目录。
    DirectoryMustBeAbsolute {
        /// 被拒绝的目录。
        directory: PathBuf,
    },
    /// 目录不存在、不可访问或无法规范化。
    DirectoryUnavailable {
        /// 被拒绝的目录。
        directory: PathBuf,
        /// 文件系统返回的具体原因。
        source: io::Error,
    },
    /// 路径存在但不是目录。
    DirectoryNotDirectory {
        /// 被拒绝的路径。
        directory: PathBuf,
    },
    /// 项目发现或项目元数据持久化失败。
    Project(project_types::Error),
    /// 原生会话操作失败。
    Session(session::Error),
    /// 当前租户中不存在目标会话。
    SessionNotFound {
        /// 未找到的会话 ID。
        session_id: String,
    },
    /// 会话目录或项目不属于请求的作用域。
    SessionOwnershipMismatch {
        /// 被拒绝的会话 ID。
        session_id: String,
        /// 请求作用域的规范化目录。
        expected_directory: PathBuf,
        /// 会话记录中的目录。
        actual_directory: PathBuf,
        /// 请求作用域对应的项目 ID。
        expected_project_id: String,
        /// 会话记录中的项目 ID。
        actual_project_id: String,
    },
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectoryMustBeAbsolute { directory } => {
                write!(f, "session directory must be absolute: {}", directory.display())
            }
            Self::DirectoryUnavailable { directory, source } => {
                write!(f, "session directory is unavailable ({}): {source}", directory.display())
            }
            Self::DirectoryNotDirectory { directory } => {
                write!(f, "session directory is not a directory: {}", directory.display())
            }
            Self::Project(error) => write!(f, "session project resolution failed: {error}"),
            Self::Session(error) => write!(f, "native session operation failed: {error}"),
            Self::SessionNotFound { session_id } => {
                write!(f, "session not found in the requested tenant scope: {session_id}")
            }
            Self::SessionOwnershipMismatch {
                session_id,
                expected_directory,
                actual_directory,
                expected_project_id,
                actual_project_id,
            } => write!(
                f,
                "session {session_id} does not belong to directory {} and project {expected_project_id} (recorded directory: {}, project: {actual_project_id})",
                expected_directory.display(),
                actual_directory.display()
            ),
        }
    }
}

impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DirectoryUnavailable { source, .. } => Some(source),
            Self::Project(error) => Some(error),
            Self::Session(error) => Some(error),
            _ => None,
        }
    }
}

impl From<project_types::Error> for LifecycleError {
    fn from(value: project_types::Error) -> Self {
        Self::Project(value)
    }
}

impl From<session::Error> for LifecycleError {
    fn from(value: session::Error) -> Self {
        Self::Session(value)
    }
}
