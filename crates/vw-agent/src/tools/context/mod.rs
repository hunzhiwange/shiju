//! 工具运行时上下文。
//!
//! `ToolUseContext` 是 Claude Tools V2 的厚上下文容器，用来承载工具执行阶段
//! 需要共享的运行时能力与状态，而不是继续把权限、审批、hook、读取缓存等状态
//! 零散地塞在调用栈上。
//!
//! 由于当前仓库里仍有大量旧工具沿用 `Tool::call(input)` / `Tool::execute(args)`
//! 签名，本模块同时提供一个 task-local 作用域，让具体工具在不改 trait 签名的
//! 前提下，也能按需读取当前上下文。

use super::read_state::{FileReadStateCache, FileReadStateEntry, FileSnapshot};
use crate::agent::loop_::approval::NonCliApprovalContext;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::multimodal::assets::{ImageAssetId, ImageAssetScope};
use crate::app::agent::observability::Observer;
use crate::app::agent::providers::ChatMessage;
use crate::app::agent::security::SecurityPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use vw_config_types::agent::DelegateCapabilityPolicy;

/// 单次工具循环允许排队重看的图片数量上限。
pub const MAX_PENDING_IMAGE_RELOADS: usize = 4;

tokio::task_local! {
    static ACTIVE_TOOL_USE_CONTEXT: Arc<ToolUseContext>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanModeState {
    pub active: bool,
    pub goal: Option<String>,
    pub note: Option<String>,
    pub entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeBindingState {
    pub directory: Option<String>,
    pub name: Option<String>,
    pub branch: Option<String>,
    pub entered_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowState {
    pub plan_mode: PlanModeState,
    pub worktree: WorktreeBindingState,
}

/// 待重看图片队列：去重、有数量上限，并绑定当前资产范围。
#[derive(Debug, Clone, Default)]
pub struct PendingImageReloadQueue {
    scope_key: Option<String>,
    ids: Vec<String>,
}

impl PendingImageReloadQueue {
    /// 绑定当前图片资产范围键。
    pub fn set_scope_key(&mut self, scope_key: impl Into<String>) {
        self.scope_key = Some(scope_key.into());
    }

    /// 当前范围键。
    pub fn scope_key(&self) -> Option<&str> {
        self.scope_key.as_deref()
    }

    /// 入队一个 image_id；重复忽略，超量返回错误。
    pub fn enqueue(&mut self, image_id: &str) -> Result<(), String> {
        let image_id = image_id.trim();
        if image_id.is_empty() {
            return Err("image_id must be non-empty".to_string());
        }
        if self.ids.iter().any(|existing| existing == image_id) {
            return Ok(());
        }
        if self.ids.len() >= MAX_PENDING_IMAGE_RELOADS {
            return Err(format!(
                "pending image reload queue is full (max {MAX_PENDING_IMAGE_RELOADS})"
            ));
        }
        self.ids.push(image_id.to_string());
        Ok(())
    }

    /// 查看当前排队 ID。
    pub fn peek(&self) -> &[String] {
        &self.ids
    }

    /// 取出全部排队 ID（不清空 scope）。
    pub fn take_all(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ids)
    }

    /// 清空排队 ID。
    pub fn clear(&mut self) {
        self.ids.clear();
    }
}

/// Claude Tools V2 共享运行时上下文。
#[derive(Clone)]
pub struct ToolUseContext {
    session: String,
    root: Option<String>,
    message_id: Option<String>,
    tool_call_id: Option<String>,
    full_access_enabled: bool,
    security: Option<Arc<SecurityPolicy>>,
    approval: Option<Arc<ApprovalManager>>,
    messages_view: Option<Arc<Vec<ChatMessage>>>,
    explicitly_requested_skills: Arc<BTreeSet<String>>,
    trusted_user_selected_tools: Arc<BTreeSet<String>>,
    trusted_user_selected_skills: Arc<BTreeSet<String>>,
    delegate_skill_policy: Option<DelegateCapabilityPolicy>,
    delegate_default_allowed_skills: Arc<BTreeSet<String>>,
    read_state: Arc<Mutex<FileReadStateCache>>,
    observer: Option<Arc<dyn Observer>>,
    progress_tx: Option<Sender<String>>,
    abort_token: Option<CancellationToken>,
    hook_runner: Option<Arc<HookRunner>>,
    non_cli_approval_context: Option<NonCliApprovalContext>,
    channel: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    model_selected_by_auto: bool,
    turn_id: Option<String>,
    iteration: usize,
    bypass_non_cli_approval_for_turn: bool,
    workflow: Arc<Mutex<WorkflowState>>,
    pending_image_reloads: Arc<Mutex<PendingImageReloadQueue>>,
}

impl Default for ToolUseContext {
    fn default() -> Self {
        Self::new(
            "default",
            std::env::current_dir().ok().map(|path| path.to_string_lossy().to_string()),
        )
    }
}

impl fmt::Debug for ToolUseContext {
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
            .field("has_non_cli_approval_context", &self.non_cli_approval_context.is_some())
            .finish()
    }
}

impl ToolUseContext {
    /// 创建新的工具运行时上下文。
    pub fn new(session: impl Into<String>, root: Option<String>) -> Self {
        Self {
            session: session.into(),
            root,
            message_id: None,
            tool_call_id: None,
            full_access_enabled: false,
            security: None,
            approval: None,
            messages_view: None,
            explicitly_requested_skills: Arc::new(BTreeSet::new()),
            trusted_user_selected_tools: Arc::new(BTreeSet::new()),
            trusted_user_selected_skills: Arc::new(BTreeSet::new()),
            delegate_skill_policy: None,
            delegate_default_allowed_skills: Arc::new(BTreeSet::new()),
            read_state: Arc::new(Mutex::new(FileReadStateCache::default())),
            observer: None,
            progress_tx: None,
            abort_token: None,
            hook_runner: None,
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
        }
    }

    /// 返回当前会话 ID。
    pub fn session(&self) -> &str {
        &self.session
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

    /// 返回当前会话是否启用了完全访问权限。
    pub fn full_access_enabled(&self) -> bool {
        self.full_access_enabled
    }

    /// 返回安全策略。
    pub fn security(&self) -> Option<&SecurityPolicy> {
        self.security.as_deref()
    }

    /// 返回审批管理器。
    pub fn approval_manager(&self) -> Option<&ApprovalManager> {
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

    /// 返回仅由运行时注入的可信用户工具选择。
    ///
    /// 工具调用 JSON 和模型生成文本无法修改这个集合。
    pub fn trusted_user_selected_tools(&self) -> &BTreeSet<String> {
        self.trusted_user_selected_tools.as_ref()
    }

    /// 返回仅由运行时注入的可信用户技能选择。
    ///
    /// 工具调用 JSON 和模型生成文本无法修改这个集合。
    pub fn trusted_user_selected_skills(&self) -> &BTreeSet<String> {
        self.trusted_user_selected_skills.as_ref()
    }

    /// 返回请求边界已确定的委托技能策略及配置默认项。
    pub(crate) fn delegate_skill_policy(
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
    pub fn observer(&self) -> Option<&Arc<dyn Observer>> {
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
    pub fn hook_runner(&self) -> Option<&HookRunner> {
        self.hook_runner.as_deref()
    }

    /// 返回非 CLI 审批上下文。
    pub(crate) fn non_cli_approval_context(&self) -> Option<&NonCliApprovalContext> {
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
        self.workflow.lock().unwrap_or_else(|error| error.into_inner()).clone()
    }

    /// 返回当前 plan mode 状态。
    pub fn plan_mode_state(&self) -> PlanModeState {
        self.workflow_state().plan_mode
    }

    /// 返回当前绑定的 worktree 状态。
    pub fn worktree_binding_state(&self) -> WorktreeBindingState {
        self.workflow_state().worktree
    }

    /// 标记进入 plan mode。
    pub fn enter_plan_mode(&self, goal: Option<String>, note: Option<String>) -> PlanModeState {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        workflow.plan_mode.active = true;
        workflow.plan_mode.goal = goal;
        workflow.plan_mode.note = note;
        workflow.plan_mode.entered_at_ms = Some(current_time_ms());
        workflow.plan_mode.clone()
    }

    /// 标记退出 plan mode。
    pub fn exit_plan_mode(&self) -> PlanModeState {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        workflow.plan_mode.active = false;
        workflow.plan_mode.clone()
    }

    /// 绑定当前会话使用的 worktree。
    pub fn bind_worktree(
        &self,
        directory: String,
        name: String,
        branch: String,
    ) -> WorktreeBindingState {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        workflow.worktree = WorktreeBindingState {
            directory: Some(directory),
            name: Some(name),
            branch: Some(branch),
            entered_at_ms: Some(current_time_ms()),
        };
        workflow.worktree.clone()
    }

    /// 清除当前会话的 worktree 绑定。
    pub fn clear_worktree_binding(&self) -> WorktreeBindingState {
        let mut workflow = self.workflow.lock().unwrap_or_else(|error| error.into_inner());
        workflow.worktree = WorktreeBindingState::default();
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
    pub fn with_security(mut self, security: Arc<SecurityPolicy>) -> Self {
        self.security = Some(security);
        self
    }

    /// 注入审批管理器。
    pub fn with_approval(mut self, approval: Arc<ApprovalManager>) -> Self {
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

    /// 注入可信用户明确选择的工具。
    ///
    /// 此方法仅供请求边界在解析原始用户输入后调用；子工具克隆上下文时会保留该集合。
    pub fn with_trusted_user_selected_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.trusted_user_selected_tools = Arc::new(normalize_capability_selection(tools));
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
    pub(crate) fn with_delegate_skill_policy<I, S>(
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
    pub fn set_image_asset_scope(&self, scope: &ImageAssetScope) {
        let mut queue =
            self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.set_scope_key(scope.key());
    }

    /// 返回待重看队列句柄。
    pub fn pending_image_reloads_handle(&self) -> Arc<Mutex<PendingImageReloadQueue>> {
        Arc::clone(&self.pending_image_reloads)
    }

    /// 将 image_id 加入待重看队列。
    pub fn enqueue_pending_image_reload(&self, image_id: &ImageAssetId) -> Result<(), String> {
        let mut queue =
            self.pending_image_reloads.lock().unwrap_or_else(|error| error.into_inner());
        queue.enqueue(image_id.as_str())
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
    pub fn with_observer(mut self, observer: Arc<dyn Observer>) -> Self {
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

    /// 注入 Hook 运行器。
    pub fn with_hook_runner(mut self, hook_runner: Arc<HookRunner>) -> Self {
        self.hook_runner = Some(hook_runner);
        self
    }

    /// 注入非 CLI 审批上下文。
    pub(crate) fn with_non_cli_approval_context(
        mut self,
        non_cli_approval_context: NonCliApprovalContext,
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
            .or_else(|| self.security().map(|security| security.workspace_dir.clone()))
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
pub(crate) async fn scope_tool_use_context<F, T>(context: Arc<ToolUseContext>, future: F) -> T
where
    F: Future<Output = T>,
{
    ACTIVE_TOOL_USE_CONTEXT.scope(context, future).await
}

/// 读取当前 task-local 中的工具上下文。
pub fn current_tool_use_context() -> Option<Arc<ToolUseContext>> {
    ACTIVE_TOOL_USE_CONTEXT.try_with(Clone::clone).ok()
}

/// 将 image_id 加入当前工具上下文的待重看队列。
pub fn enqueue_current_pending_image_reload(image_id: &ImageAssetId) -> Result<(), String> {
    let context = current_tool_use_context()
        .ok_or_else(|| "image_asset_read requires an active tool use context".to_string())?;
    context.enqueue_pending_image_reload(image_id)
}

/// 返回当前 task-local 上下文中某个路径的最近读取状态。
pub fn current_read_state_for_path(path: impl AsRef<Path>) -> Option<FileReadStateEntry> {
    let context = current_tool_use_context()?;
    let root = context.workspace_root();
    let read_state = context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());
    read_state.get(root.as_deref(), path)
}

/// 在当前 task-local 上下文中记录某个路径的最新读取状态。
pub fn note_current_read_state(
    path: impl AsRef<Path>,
    bytes_read: usize,
    partial_view: bool,
    offset: Option<usize>,
    limit: Option<usize>,
    snapshot: Option<FileSnapshot>,
) -> Option<PathBuf> {
    let context = current_tool_use_context()?;
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
pub fn invalidate_current_read_state(path: impl AsRef<Path>) -> Option<FileReadStateEntry> {
    let context = current_tool_use_context()?;
    let root = context.workspace_root();
    let read_state = context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());
    read_state.invalidate(root.as_deref(), path)
}

/// 返回给定路径在当前上下文下的工作区归一化路径。
pub fn normalize_path_for_context(context: &ToolUseContext, path: impl AsRef<Path>) -> PathBuf {
    let root = context.workspace_root();
    FileReadStateCache::normalized_path(root.as_deref(), path)
}
#[cfg(test)]
mod tests;
