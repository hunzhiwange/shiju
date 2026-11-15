//! 工具运行时上下文。
//!
//! `ToolUseContext` 是 Claude Tools V2 的厚上下文容器，用来承载工具执行阶段
//! 需要共享的运行时能力与状态，而不是继续把权限、审批、hook、读取缓存等状态
//! 零散地塞在调用栈上。
//!
//! 由于当前仓库里仍有大量旧工具沿用 `Tool::call(input)` / `Tool::execute(args)`
//! 签名，本模块同时提供一个 task-local 作用域，让具体工具在不改 trait 签名的
//! 前提下，也能按需读取当前上下文。

use crate::read_state::{FileReadStateCache, FileReadStateEntry, FileSnapshot};
use crate::host::ContextHost;
use model_types::ChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use config_agent::DelegateCapabilityPolicy;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeBindingState {
    pub directory: Option<String>,
    pub name: Option<String>,
    pub branch: Option<String>,
    pub entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowState {
    pub worktree: WorktreeBindingState,
    /// The live workflow owns usage; snapshots and serialized metadata do not.
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    worktree_use: Option<Arc<std::fs::File>>,
}

pub use crate::{MAX_PENDING_IMAGE_RELOADS, PendingImageReloadQueue};

/// Claude Tools V2 共享运行时上下文。
#[derive(Clone)]
pub struct ToolUseContext<H: ContextHost> {
    #[cfg(not(target_arch = "wasm32"))]
    run_worktree_owner: Option<crate::run_worktree::RunWorktreeOwner>,
    session: String,
    session_skills: H::SessionSkillSources,
    tenant_scope: vw_api_transport::TenantScope,
    hook_input: Option<serde_json::Value>,
    hook_contexts: Arc<Mutex<Vec<String>>>,
    pub async_hooks: Arc<H::AsyncHooks>,
    agent_name: Option<String>,
    root: Option<String>,
    message_id: Option<String>,
    tool_call_id: Option<String>,
    full_access_enabled: bool,
    security: Option<Arc<H::SecurityPolicy>>,
    delegated_security: Option<Arc<H::SecurityPolicy>>,
    delegated_tools: Option<Arc<BTreeSet<String>>>,
    delegated_skill_rules: Vec<permission_types::Ruleset>,
    approval: Option<Arc<H::ApprovalManager>>,
    messages_view: Option<Arc<Vec<ChatMessage>>>,
    explicitly_requested_skills: Arc<BTreeSet<String>>,
    trusted_user_selected_skills: Arc<BTreeSet<String>>,
    native_mcp_selection: Option<Arc<BTreeSet<String>>>,
    native_permissions: Vec<Arc<H::EffectiveDefaults>>,
    delegate_skill_policy: Option<DelegateCapabilityPolicy>,
    delegate_default_allowed_skills: Arc<BTreeSet<String>>,
    read_state: Arc<Mutex<FileReadStateCache>>,
    observer: Option<Arc<H::Observer>>,
    progress_tx: Option<Sender<String>>,
    abort_token: Option<CancellationToken>,
    hook_runner: Option<Arc<H::HookRunner>>,
    plugin_snapshot: Option<Arc<H::PluginSessionSnapshot>>,
    non_cli_approval_context: Option<crate::approval::NonCliApprovalContext>,
    channel: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    model_selected_by_auto: bool,
    turn_id: Option<String>,
    iteration: usize,
    bypass_non_cli_approval_for_turn: bool,
    workflow: Arc<Mutex<WorkflowState>>,
    pending_image_reloads: Arc<Mutex<PendingImageReloadQueue>>,
    tool_discovery: Option<Arc<H::ToolDiscoverySession>>,
    tool_exposure_snapshot: Option<Arc<BTreeSet<String>>>,
    tool_catalog: Option<Arc<H::ToolCatalogSnapshot>>,
    live_tool_catalog: Option<Arc<H::ToolCatalog>>,
    invoked_skills: Arc<Mutex<Vec<H::InvokedSkillRecord>>>,
}

impl<H: ContextHost> Default for ToolUseContext<H> {
    fn default() -> Self {
        Self::new(
            "default",
            std::env::current_dir().ok().map(|path| path.to_string_lossy().to_string()),
        )
    }
}

impl<H: ContextHost> fmt::Debug for ToolUseContext<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolUseContext")
            .field("session", &self.session)
            .field("root", &self.root)
            .field("message_id", &self.message_id)
            .field("tool_call_id", &self.tool_call_id)
            .field("full_access_enabled", &self.full_access_enabled)
            .field("channel", &self.channel)
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("model_selected_by_auto", &self.model_selected_by_auto)
            .field("turn_id", &self.turn_id)
            .field("iteration", &self.iteration)
            .field("has_security", &self.security.is_some())
            .field("has_approval", &self.approval.is_some())
            .field("has_messages_view", &self.messages_view.is_some())
            .field("has_observer", &self.observer.is_some())
            .field("has_progress_tx", &self.progress_tx.is_some())
            .field("has_abort_token", &self.abort_token.is_some())
            .field("has_hook_runner", &self.hook_runner.is_some())
            .field("has_plugin_snapshot", &self.plugin_snapshot.is_some())
            .field("has_non_cli_approval_context", &self.non_cli_approval_context.is_some())
            .finish()
    }
}

impl<H: ContextHost> ToolUseContext<H> {
    pub fn with_session_skills(
        mut self,
        sources: H::SessionSkillSources,
    ) -> Self {
        self.session_skills = sources;
        self
    }
    pub fn session_skills(&self) -> &H::SessionSkillSources {
        &self.session_skills
    }

    /// Only the real skill tool writes this typed, request-local provenance.
    pub fn record_invoked_skill(&self, record: H::InvokedSkillRecord) {
        self.invoked_skills.lock().unwrap_or_else(|error| error.into_inner()).push(record);
    }

    pub fn take_invoked_skills(&self) -> Vec<H::InvokedSkillRecord> {
        std::mem::take(&mut *self.invoked_skills.lock().unwrap_or_else(|error| error.into_inner()))
    }

    /// 创建新的工具运行时上下文。
    pub fn new(session: impl Into<String>, root: Option<String>) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            run_worktree_owner: None,
            session: session.into(),
            tenant_scope: H::current_tenant_scope(),
            hook_input: None,
            hook_contexts: Arc::new(Mutex::new(Vec::new())),
            async_hooks: Arc::default(),
            agent_name: None,
            root,
            message_id: None,
            tool_call_id: None,
            full_access_enabled: false,
            security: None,
            delegated_security: None,
            delegated_tools: None,
            delegated_skill_rules: Vec::new(),
            approval: None,
            messages_view: None,
            explicitly_requested_skills: Arc::new(BTreeSet::new()),
            trusted_user_selected_skills: Arc::new(BTreeSet::new()),
            native_mcp_selection: None,
            native_permissions: Vec::new(),
            delegate_skill_policy: None,
            delegate_default_allowed_skills: Arc::new(BTreeSet::new()),
            read_state: Arc::new(Mutex::new(FileReadStateCache::default())),
            observer: None,
            progress_tx: None,
            abort_token: None,
            hook_runner: None,
            plugin_snapshot: None,
            non_cli_approval_context: None,
            channel: None,
            provider: None,
            model: None,
            model_selected_by_auto: false,
            turn_id: None,
            iteration: 0,
            bypass_non_cli_approval_for_turn: false,
            workflow: Arc::new(Mutex::new(WorkflowState::default())),
            pending_image_reloads: Arc::new(Mutex::new(PendingImageReloadQueue::default())),
            tool_discovery: None,
            tool_exposure_snapshot: None,
            tool_catalog: None,
            live_tool_catalog: None,
            session_skills: Default::default(),
            invoked_skills: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_hook_input(mut self, input: serde_json::Value) -> Self {
        self.hook_input = Some(input);
        self
    }

    pub fn hook_input(&self) -> Option<&serde_json::Value> {
        self.hook_input.as_ref()
    }

    /// Buffer context for the owning turn only.
    pub fn push_hook_context(&self, text: String) -> anyhow::Result<()> {
        let mut pending =
            self.hook_contexts.lock().map_err(|_| anyhow::anyhow!("hook context lock poisoned"))?;
        anyhow::ensure!(pending.len() < 64, "too many pending hook contexts");
        pending.push(text);
        Ok(())
    }

    pub fn take_hook_contexts(&self) -> Vec<String> {
        self.hook_contexts
            .lock()
            .map(|mut pending| std::mem::take(&mut *pending))
            .unwrap_or_default()
    }

    pub fn for_child(mut self, session: String, tools: Vec<String>) -> Self {
        self.session = session.clone();
        self.delegated_tools = Some(Arc::new(tools.into_iter().collect()));
        self.agent_name = None;
        self.delegated_security = self.security.clone().or(self.delegated_security.clone());
        self.messages_view = None;
        self.with_turn_id(session)
    }

    pub fn inherit_delegated_authority(mut self) -> Self {
        if let Some(parent) = current_tool_use_context::<H>()
            && parent.session() == self.session()
            && parent.tenant_scope == self.tenant_scope
        {
            self.delegated_security = parent.delegated_security.clone();
            self.delegated_tools = parent.delegated_tools.clone();
            self.delegated_skill_rules = parent.delegated_skill_rules.clone();
            self.native_mcp_selection = parent.native_mcp_selection.clone();
            self.native_permissions = parent.native_permissions.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.run_worktree_owner = parent.run_worktree_owner.clone();
            }
            self.workflow = parent.workflow.clone();
        }
        self
    }

    pub fn native_mcp_selection(&self) -> Option<&BTreeSet<String>> {
        self.native_mcp_selection.as_deref()
    }

    pub fn with_native_mcp_selection(mut self, mut ids: BTreeSet<String>) -> Self {
        if let Some(parent) = &self.native_mcp_selection {
            ids.retain(|id| parent.contains(id));
        }
        self.native_mcp_selection = Some(Arc::new(ids));
        self
    }

    pub fn with_native_permission(
        mut self,
        policy: Arc<H::EffectiveDefaults>,
    ) -> Self {
        self.native_permissions.push(policy);
        self
    }

    pub fn native_permission_action(
        &self,
        permission: &str,
        aliases: &[String],
        pattern: &str,
    ) -> permission_types::Action {
        use permission_types::Action;
        self.native_permissions.iter().fold(Action::Allow, |current, policy| {
            match (current, H::permission_action(policy, permission, aliases, pattern)) {
                (Action::Deny, _) | (_, Action::Deny) => Action::Deny,
                (Action::Ask, _) | (_, Action::Ask) => Action::Ask,
                _ => Action::Allow,
            }
        })
    }

    pub fn delegated_tool_is_allowed(&self, id: &str) -> bool {
        self.delegated_tools.as_ref().is_none_or(|tools| tools.contains(id))
    }

    pub fn with_delegated_skill_rules(
        mut self,
        rules: permission_types::Ruleset,
    ) -> Self {
        self.delegated_skill_rules.push(rules);
        self
    }

    pub fn delegated_skill_rules(&self) -> &[permission_types::Ruleset] {
        &self.delegated_skill_rules
    }

    pub fn delegated_security(&self) -> Option<&H::SecurityPolicy> {
        self.delegated_security.as_deref()
    }

    pub fn tenant_scope(&self) -> vw_api_transport::TenantScope {
        self.tenant_scope.clone()
    }

    /// 返回当前会话 ID。
    pub fn session(&self) -> &str {
        &self.session
    }

    /// 返回请求边界确定的代理，用于技能权限查询。
    pub fn agent_name(&self) -> Option<&str> {
        self.agent_name.as_deref()
    }

    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }

    /// 返回当前工作区根路径字符串。
    pub fn root(&self) -> Option<&str> {
        self.root.as_deref()
    }

    /// 返回工作区根路径对象。
    pub fn root_path(&self) -> Option<PathBuf> {
        self.root.as_ref().map(PathBuf::from)
    }

    /// 返回当前工具所属消息 ID。
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// 返回当前工具调用 ID。
    pub fn tool_call_id(&self) -> Option<&str> {
        self.tool_call_id.as_deref()
    }

    /// Shared authorized catalog and exposure state for this user turn only.
    pub fn tool_discovery(&self) -> Option<Arc<H::ToolDiscoverySession>> {
        self.tool_discovery.clone()
    }

    pub fn with_tool_discovery(
        mut self,
        discovery: Arc<H::ToolDiscoverySession>,
    ) -> Self {
        self.tool_discovery = Some(discovery);
        self
    }

    /// Immutable visibility for the model request that produced these calls.
    pub fn with_tool_exposure_snapshot(
        mut self,
        ids: impl IntoIterator<Item = String>,
    ) -> Self {
        self.tool_exposure_snapshot = Some(Arc::new(ids.into_iter().collect()));
        self
    }

    /// Supply the session catalog; loops capture it only at model request boundaries.
    pub fn with_live_tool_catalog(mut self, catalog: Arc<H::ToolCatalog>) -> Self {
        self.live_tool_catalog = Some(catalog);
        self
    }

    pub fn live_tool_catalog(&self) -> Option<Arc<H::ToolCatalog>> {
        self.live_tool_catalog.clone()
    }

    pub fn catalog_registry(&self) -> Option<Vec<Box<dyn tool_types::Tool>>> {
        self.tool_catalog
            .as_ref()
            .map(|catalog| H::catalog_registry(catalog))
    }

    /// Pin the executable catalog captured for this model request.
    pub fn with_tool_catalog(mut self, catalog: Arc<H::ToolCatalogSnapshot>) -> Self {
        self.tool_catalog = Some(catalog);
        self
    }

    pub fn tool_catalog(&self) -> Option<&Arc<H::ToolCatalogSnapshot>> {
        self.tool_catalog.as_ref()
    }

    pub fn tool_is_exposed(&self, id: &str) -> bool {
        self.delegated_tool_is_allowed(id)
            && self.tool_catalog.as_ref().is_none_or(|catalog| H::catalog_authorizes(catalog, id))
            && self.tool_discovery.as_ref().is_none_or(|discovery| H::discovery_authorizes(discovery, id))
            && self.tool_exposure_snapshot.as_ref().is_none_or(|ids| ids.contains(id))
    }

    /// 返回当前会话是否启用了完全访问权限。
    pub fn full_access_enabled(&self) -> bool {
        self.full_access_enabled
    }

    /// 返回安全策略。
    pub fn security(&self) -> Option<&H::SecurityPolicy> {
        self.security.as_deref()
    }

    /// 返回审批管理器。
    pub fn approval_manager(&self) -> Option<&H::ApprovalManager> {
        self.approval.as_deref()
    }

    /// 返回消息视图快照。
    pub fn messages_view(&self) -> Option<&[ChatMessage]> {
        self.messages_view.as_deref().map(Vec::as_slice)
    }

    /// 返回当前用户或仓库协议明确指定的技能名称。
    pub fn explicitly_requested_skills(&self) -> &BTreeSet<String> {
        self.explicitly_requested_skills.as_ref()
    }

    /// 判断技能是否由当前用户或仓库协议明确指定。
    pub fn is_skill_explicitly_requested(&self, name: &str) -> bool {
        self.explicitly_requested_skills.contains(&name.trim().to_lowercase())
    }

    /// 返回仅由运行时注入的可信用户技能选择。
    ///
    /// 工具调用 JSON 和模型生成文本无法修改这个集合。
    pub fn trusted_user_selected_skills(&self) -> &BTreeSet<String> {
        self.trusted_user_selected_skills.as_ref()
    }

    /// 返回请求边界已确定的委托技能策略及配置默认项。
    pub fn delegate_skill_policy(
        &self,
    ) -> Option<(DelegateCapabilityPolicy, &BTreeSet<String>)> {
        self.delegate_skill_policy
            .map(|policy| (policy, self.delegate_default_allowed_skills.as_ref()))
    }

    /// 返回文件读取状态句柄。
    pub fn read_state_handle(&self) -> Arc<Mutex<FileReadStateCache>> {
        self.read_state.clone()
    }

    /// 返回读取状态的当前快照。
    pub fn read_state_snapshot(&self) -> FileReadStateCache {
        self.read_state.lock().unwrap_or_else(|error| error.into_inner()).clone()
    }

    /// 用另一个缓存快照合并读取状态。
    pub fn merge_read_state(&self, other: &FileReadStateCache) {
        self.read_state.lock().unwrap_or_else(|error| error.into_inner()).merge(other);
    }

    /// 返回观测器。
    pub fn observer(&self) -> Option<&Arc<H::Observer>> {
        self.observer.as_ref()
    }

    /// 返回进度通道。
    pub fn progress_tx(&self) -> Option<&Sender<String>> {
        self.progress_tx.as_ref()
    }

    /// 返回取消令牌。
    pub fn abort_token(&self) -> Option<&CancellationToken> {
        self.abort_token.as_ref()
    }

    /// 返回 Hook 运行器。
    pub fn hook_runner(&self) -> Option<&H::HookRunner> {
        self.hook_runner.as_deref()
    }

    /// 返回非 CLI 审批上下文。
    pub fn non_cli_approval_context(&self) -> Option<&crate::approval::NonCliApprovalContext> {
        self.non_cli_approval_context.as_ref()
    }

    /// 返回通道名称。
    pub fn channel(&self) -> Option<&str> {
        self.channel.as_deref()
    }

    /// 返回 provider 名称。
    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    /// 返回模型名称。
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// 返回当前模型是否由 Auto 为父请求选择。
    pub fn model_selected_by_auto(&self) -> bool {
        self.model_selected_by_auto
    }

    /// 返回 turn ID。
    pub fn turn_id(&self) -> Option<&str> {
        self.turn_id.as_deref()
    }

    /// 返回当前迭代序号。
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// 返回是否允许跳过本轮非 CLI 审批。
    pub fn bypass_non_cli_approval_for_turn(&self) -> bool {
        self.bypass_non_cli_approval_for_turn
    }

    /// 返回当前流程状态快照。
    pub fn workflow_state(&self) -> WorkflowState {
        let workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        WorkflowState {
            worktree: workflow.worktree.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            worktree_use: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_run_worktree_owner(mut self, owner: crate::run_worktree::RunWorktreeOwner) -> Self {
        self.run_worktree_owner = Some(owner);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_worktree_owner(&self) -> Option<&crate::run_worktree::RunWorktreeOwner> {
        self.run_worktree_owner.as_ref()
    }

    /// 返回当前绑定的 worktree 状态。
    pub fn worktree_binding_state(&self) -> WorktreeBindingState {
        self.workflow_state().worktree
    }

    /// 绑定当前会话使用的 worktree。
    pub fn bind_worktree(
        &self,
        directory: String,
        name: String,
        branch: String,
    ) -> anyhow::Result<WorktreeBindingState> {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        if workflow.worktree.directory.is_some() {
            anyhow::bail!("a worktree is already bound to this session");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let worktree_use =
            H::use_lease(Path::new(&directory))?.map(Arc::new);
        workflow.worktree = WorktreeBindingState {
            directory: Some(directory),
            name: Some(name),
            branch: Some(branch),
            entered_at_ms: Some(current_time_ms()),
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            workflow.worktree_use = worktree_use;
        }
        Ok(workflow.worktree.clone())
    }

    /// 清除当前会话的 worktree 绑定。
    pub fn clear_worktree_binding(&self) -> WorktreeBindingState {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        workflow.worktree = WorktreeBindingState::default();
        #[cfg(not(target_arch = "wasm32"))]
        drop(workflow.worktree_use.take());
        workflow.worktree.clone()
    }

    /// 覆盖工作区根路径。
    pub fn with_root(mut self, root: Option<String>) -> Self {
        self.root = root;
        self
    }

    /// 注入当前工具所属消息 ID。
    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// 注入当前工具调用 ID。
    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    /// 标记当前会话启用了完全访问权限。
    pub fn with_full_access_enabled(mut self, full_access_enabled: bool) -> Self {
        self.full_access_enabled = full_access_enabled;
        self
    }

    /// 注入安全策略。
    pub fn with_security(mut self, security: Arc<H::SecurityPolicy>) -> Self {
        self.security = Some(security);
        self
    }

    /// 注入审批管理器。
    pub fn with_approval(mut self, approval: Arc<H::ApprovalManager>) -> Self {
        self.approval = Some(approval);
        self
    }

    /// 注入消息视图快照。
    pub fn with_messages_view(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages_view = Some(Arc::new(messages));
        self
    }

    /// 注入由运行时从可信用户输入和仓库协议中解析出的技能名称。
    pub fn with_explicitly_requested_skills<I, S>(mut self, skills: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.explicitly_requested_skills = Arc::new(
            skills
                .into_iter()
                .map(|skill| skill.as_ref().trim().to_lowercase())
                .filter(|skill| !skill.is_empty())
                .collect(),
        );
        self
    }

    /// 注入可信用户明确选择的技能。
    ///
    /// 此方法仅供请求边界在解析原始用户输入后调用；子工具克隆上下文时会保留该集合。
    pub fn with_trusted_user_selected_skills<I, S>(mut self, skills: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.trusted_user_selected_skills = Arc::new(normalize_capability_selection(skills));
        self
    }

    /// 注入 Gateway/Session 传入的委托技能策略。模型工具参数无法修改这些值。
    pub fn with_delegate_skill_policy<I, S>(
        mut self,
        policy: DelegateCapabilityPolicy,
        default_skills: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.delegate_skill_policy = Some(policy);
        self.delegate_default_allowed_skills =
            Arc::new(normalize_capability_selection(default_skills));
        self
    }

    /// 复用现有的读取状态缓存句柄。
    pub fn with_read_state_handle(mut self, read_state: Arc<Mutex<FileReadStateCache>>) -> Self {
        self.read_state = read_state;
        self
    }

    /// 复用现有的流程状态句柄。
    pub fn with_workflow_handle(mut self, workflow: Arc<Mutex<WorkflowState>>) -> Self {
        self.workflow = workflow;
        self
    }

    /// 注入待重看图片队列句柄。
    pub fn with_pending_image_reloads_handle(
        mut self,
        pending_image_reloads: Arc<Mutex<PendingImageReloadQueue>>,
    ) -> Self {
        self.pending_image_reloads = pending_image_reloads;
        self
    }

    /// 绑定当前图片资产范围。
    pub fn set_image_asset_scope(&self, scope: &H::ImageAssetScope) {
        let mut queue =
            self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.set_scope_key(H::image_scope_key(scope));
    }

    /// 返回待重看队列句柄。
    pub fn pending_image_reloads_handle(&self) -> Arc<Mutex<PendingImageReloadQueue>> {
        Arc::clone(&self.pending_image_reloads)
    }

    /// 将 image_id 加入待重看队列。
    pub fn enqueue_pending_image_reload(&self, image_id: &H::ImageAssetId) -> Result<(), String> {
        let mut queue =
            self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.enqueue(H::image_id_str(image_id))
    }

    /// 查看待重看 ID 快照。
    pub fn peek_pending_image_reloads(&self) -> Vec<String> {
        let queue = self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.peek().to_vec()
    }

    /// 清除待重看队列（仅在 LLM step 成功后调用）。
    pub fn clear_pending_image_reloads(&self) {
        let mut queue =
            self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.clear();
    }

    /// 当前绑定的图片资产范围键。
    pub fn image_asset_scope_key(&self) -> Option<String> {
        let queue = self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.scope_key().map(ToOwned::to_owned)
    }

    /// 注入观测器。
    pub fn with_observer(mut self, observer: Arc<H::Observer>) -> Self {
        self.observer = Some(observer);
        self
    }

    /// 注入进度通道。
    pub fn with_progress_tx(mut self, progress_tx: Sender<String>) -> Self {
        self.progress_tx = Some(progress_tx);
        self
    }

    /// 注入取消令牌。
    pub fn with_abort_token(mut self, abort_token: CancellationToken) -> Self {
        self.abort_token = Some(abort_token);
        self
    }

    /// The data snapshot never owns store locks.
    pub fn plugin_snapshot(
        &self,
    ) -> Option<&Arc<H::PluginSessionSnapshot>> {
        self.plugin_snapshot.as_ref()
    }

    pub fn with_plugin_snapshot(
        mut self,
        snapshot: Arc<H::PluginSessionSnapshot>,
    ) -> Self {
        self.plugin_snapshot = Some(snapshot);
        self
    }

    /// 注入 Hook 运行器。
    pub fn with_hook_runner(mut self, hook_runner: Arc<H::HookRunner>) -> Self {
        self.hook_runner = Some(hook_runner);
        self
    }

    /// 注入非 CLI 审批上下文。
    pub fn with_non_cli_approval_context(
        mut self,
        non_cli_approval_context: crate::approval::NonCliApprovalContext,
    ) -> Self {
        self.non_cli_approval_context = Some(non_cli_approval_context);
        self
    }

    /// 注入通道名称。
    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// 注入 provider 名称。
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// 注入模型名称。
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 标记当前模型是否由 Auto 为父请求选择。
    pub fn with_auto_model_selection(mut self, selected_by_auto: bool) -> Self {
        self.model_selected_by_auto = selected_by_auto;
        self
    }

    /// 注入 turn ID。
    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.async_hooks = Arc::default();
        self.hook_contexts = Arc::default();
        self.turn_id = Some(turn_id.into());
        self
    }

    /// 注入迭代序号。
    pub fn with_iteration(mut self, iteration: usize) -> Self {
        self.iteration = iteration;
        self
    }

    /// 标记是否可跳过本轮非 CLI 审批。
    pub fn with_bypass_non_cli_approval_for_turn(mut self, bypass: bool) -> Self {
        self.bypass_non_cli_approval_for_turn = bypass;
        self
    }

    /// 返回上下文中的根目录或给定安全策略中的工作区目录。
    pub fn workspace_root(&self) -> Option<PathBuf> {
        self.worktree_binding_state()
            .directory
            .map(PathBuf::from)
            .or_else(|| self.root_path())
            .or_else(|| self.security().map(|security| H::security_workspace(security)))
    }
}

fn normalize_capability_selection<I, S>(selection: I) -> BTreeSet<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    selection
        .into_iter()
        .map(|item| item.as_ref().trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// 在 task-local 中作用域化当前工具上下文。
pub async fn scope_tool_use_context<H: ContextHost, F, T>(context: Arc<ToolUseContext<H>>, future: F) -> T
where
    F: Future<Output = T>,
{
    H::task_local().scope(context, future).await
}

/// Carry an existing context across a synchronous request boundary.
pub fn scope_tool_use_context_sync<H: ContextHost, T>(
    context: Arc<ToolUseContext<H>>,
    body: impl FnOnce() -> T,
) -> T {
    H::task_local().sync_scope(context, body)
}

/// 读取当前 task-local 中的工具上下文。
pub fn current_tool_use_context<H: ContextHost>() -> Option<Arc<ToolUseContext<H>>> {
    H::task_local().try_with(Clone::clone).ok()
}

/// 将 image_id 加入当前工具上下文的待重看队列。
pub fn enqueue_current_pending_image_reload<H: ContextHost>(image_id: &H::ImageAssetId) -> Result<(), String> {
    let context = current_tool_use_context::<H>()
        .ok_or_else(|| "image_asset_read requires an active tool use context".to_string())?;
    context.enqueue_pending_image_reload(image_id)
}

/// 返回当前 task-local 上下文中某个路径的最近读取状态。
pub fn current_read_state_for_path<H: ContextHost>(path: impl AsRef<Path>) -> Option<FileReadStateEntry> {
    let context = current_tool_use_context::<H>()?;
    let root = context.workspace_root();
    let read_state = context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());
    read_state.get(root.as_deref(), path)
}

/// 在当前 task-local 上下文中记录某个路径的最新读取状态。
pub fn note_current_read_state<H: ContextHost>(
    path: impl AsRef<Path>,
    bytes_read: usize,
    partial_view: bool,
    offset: Option<usize>,
    limit: Option<usize>,
    snapshot: Option<FileSnapshot>,
) -> Option<PathBuf> {
    let context = current_tool_use_context::<H>()?;
    let root = context.workspace_root();
    let read_state = context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());
    Some(read_state.note_read(
        root.as_deref(),
        path,
        bytes_read,
        partial_view,
        offset,
        limit,
        snapshot,
    ))
}

/// 在当前 task-local 上下文中使某个路径的读取状态失效。
pub fn invalidate_current_read_state<H: ContextHost>(path: impl AsRef<Path>) -> Option<FileReadStateEntry> {
    let context = current_tool_use_context::<H>()?;
    let root = context.workspace_root();
    let read_state = context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());
    read_state.invalidate(root.as_deref(), path)
}

/// 返回给定路径在当前上下文下的工作区归一化路径。
pub fn normalize_path_for_context<H: ContextHost>(context: &ToolUseContext<H>, path: impl AsRef<Path>) -> PathBuf {
    let root = context.workspace_root();
    FileReadStateCache::normalized_path(root.as_deref(), path)
}
