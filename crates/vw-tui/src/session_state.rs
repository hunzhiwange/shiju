//! Canonical TUI session state shared across app-server routing, chat display, and status UI.
//! 应用服务器路由、聊天展示和状态界面共享的规范 TUI 会话状态。
//!
//! 应用服务器 API 是会话生命周期事件的边界；响应进入 TUI 后，
//! 本模块保存应用编排和组件共同使用的精简内部状态。
//! The app-server API is the boundary for session lifecycle events. Once those responses enter
//! TUI, this module holds the small internal state shape used by app orchestration and widgets.

use std::path::PathBuf;

use codex_app_server_protocol::AskForApproval;
use codex_protocol::ThreadId;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::Personality;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::PermissionProfile;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionNetworkProxyRuntime {
    /// 会话当前可用的 HTTP 代理监听地址。
    pub(crate) http_addr: String,
    pub(crate) socks_addr: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MessageHistoryMetadata {
    pub(crate) log_id: u64,
    pub(crate) entry_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadSessionState {
    /// 当前状态所属的稳定线程标识。
    pub(crate) thread_id: ThreadId,
    pub(crate) forked_from_id: Option<ThreadId>,
    pub(crate) fork_parent_title: Option<String>,
    pub(crate) thread_name: Option<String>,
    pub(crate) model: String,
    pub(crate) model_provider_id: String,
    pub(crate) service_tier: Option<String>,
    pub(crate) approval_policy: AskForApproval,
    pub(crate) approvals_reviewer: codex_protocol::config_types::ApprovalsReviewer,
    /// 供 TUI 展示界面使用的权限快照；旧版服务器响应会在摄取时依据响应 cwd 转换为配置，
    /// 从而避免缓存会话重新解释与 cwd 绑定的授权。除非用户在 TUI 中显式修改权限，
    /// 轮次请求不得把该快照当作本地权限覆盖。
    /// Permission snapshot used by TUI display surfaces. Legacy app-server
    /// responses are converted to a profile at ingestion time using the
    /// response cwd so cached sessions do not reinterpret cwd-bound grants.
    /// Turn requests must not treat this snapshot as a local permission
    /// override unless the user explicitly changed permissions in the TUI.
    pub(crate) permission_profile: PermissionProfile,
    /// 服务器已知时，记录生成 `permission_profile` 的命名配置或隐式内置配置。
    /// Named or implicit built-in profile that produced `permission_profile`,
    /// when the server knows it.
    pub(crate) active_permission_profile: Option<ActivePermissionProfile>,
    pub(crate) cwd: AbsolutePathBuf,
    pub(crate) runtime_workspace_roots: Vec<AbsolutePathBuf>,
    pub(crate) instruction_source_paths: Vec<PathUri>,
    pub(crate) reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    pub(crate) collaboration_mode: Option<Box<CollaborationMode>>,
    pub(crate) personality: Option<Personality>,
    pub(crate) message_history: Option<MessageHistoryMetadata>,
    pub(crate) network_proxy: Option<SessionNetworkProxyRuntime>,
    pub(crate) rollout_path: Option<PathBuf>,
}

impl ThreadSessionState {
    pub(crate) fn set_cwd_retargeting_implicit_runtime_workspace_root(
        &mut self,
        cwd: AbsolutePathBuf,
    ) {
        // 只有旧 cwd 本就是隐式运行时根目录时才随 cwd 迁移；显式根目录保持不变。
        let previous_cwd = std::mem::replace(&mut self.cwd, cwd.clone());
        if !self.runtime_workspace_roots.contains(&previous_cwd) {
            return;
        }

        // 新 cwd 置于首位，其余显式根目录按原顺序去重保留。
        let previous_roots = std::mem::take(&mut self.runtime_workspace_roots);
        self.runtime_workspace_roots.push(cwd);
        for root in previous_roots {
            if root != previous_cwd && !self.runtime_workspace_roots.contains(&root) {
                self.runtime_workspace_roots.push(root);
            }
        }
    }
}
