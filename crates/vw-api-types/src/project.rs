//! 项目实体与项目级操作相关类型。
//!
//! 本模块定义项目域的外部表示，负责表达：
//! - 一个工作目录如何被识别为项目
//! - 项目的 Git 状态、分支状态与活跃工作树
//! - 项目列表、项目详情以及项目名片式摘要
//! - 面向项目级别的会话列表和变更记录
//!
//! 项目是多数其他域对象的上层归属单位，因此这里的类型经常被其他模块引用。

use crate::common::{PaginatedResponse, TimestampMs};
use crate::id::{ProjectId, WorktreeId};
use crate::session::SessionSummaryDto;
use serde::{Deserialize, Serialize};

/// 网关项目列表使用的平台/公司租户作用域。
pub type TenantScopeDto = crate::id::TenantScope;

/// 网关项目资源使用的平台/公司/项目作用域。
pub type ProjectScopeDto = crate::id::ProjectScope;

/// 项目状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    /// 项目已就绪，可正常访问相关能力。
    Ready,
    /// 项目正在初始化或建立索引。
    Indexing,
    /// 项目初始化或同步过程中出现异常。
    Error,
}

/// 项目 Git 状态摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectGitStateDto {
    /// 当前目录是否是 Git 仓库。
    pub is_repo: bool,
    /// 是否存在未提交变更。
    pub has_uncommitted_changes: bool,
    /// 相对上游分支领先的提交数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    /// 相对上游分支落后的提交数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
}

/// 项目详情。
///
/// 用于列表页、项目卡片和项目详情页的统一数据视图。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDto {
    /// 项目 ID。
    pub id: ProjectId,
    /// 项目名称。
    pub name: String,
    /// 项目根目录。
    pub directory: String,
    /// 适合 UI 展示的路径。
    pub display_path: String,
    /// 当前项目状态。
    pub status: ProjectStatus,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 最近更新时间。
    pub updated_at_ms: TimestampMs,
    /// 默认分支名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    /// 当前所在分支。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    /// Git 状态摘要。
    pub git: ProjectGitStateDto,
    /// 当前激活工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_worktree_id: Option<WorktreeId>,
    /// 关联会话数量。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_count: Option<u32>,
}

/// 项目摘要与详情结构保持一致。
pub type ProjectSummaryDto = ProjectDto;
/// 项目列表分页响应。
pub type ListProjectsResponse = PaginatedResponse<ProjectSummaryDto>;

/// 查询项目列表请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListProjectsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProjectStatus>,
}

/// 通过目录解析项目请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveProjectRequest {
    pub directory: String,
    #[serde(default)]
    pub create_if_missing: bool,
}

/// 解析项目响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveProjectResponse {
    pub project: ProjectDto,
}

/// 项目图标相关更新字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IconUpdateDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// 项目命令更新字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandsUpdateDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

/// 更新项目请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UpdateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_worktree_id: Option<WorktreeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconUpdateDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<CommandsUpdateDto>,
}

/// 获取单个项目响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetProjectResponse {
    pub project: ProjectDto,
}

/// 列出项目下会话响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListProjectSessionsResponse {
    pub items: Vec<SessionSummaryDto>,
}

/// 列出项目变更记录请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListProjectChangeRecordsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

/// 项目级变更记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectChangeRecordDto {
    pub path: String,
    pub patch: String,
}

/// 列出项目变更记录响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListProjectChangeRecordsResponse {
    pub items: Vec<ProjectChangeRecordDto>,
}
