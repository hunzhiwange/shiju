use super::{
    ApprovalLogEntry, ApprovalManager, ApprovalRequest, ApprovalResponse, prompt_cli_interactive,
    summarize_args,
};
use config_security::{AutonomyConfig, NonCliNaturalLanguageApprovalMode};
use config_security::AutonomyLevel;
use chrono::Utc;
use std::collections::{HashMap, HashSet};

impl ApprovalManager {
    /// 规范化非 CLI 审批者列表。
    ///
    /// 处理输入字符串列表，去除空白字符并过滤空条目。
    fn normalize_non_cli_approvers(entries: &[String]) -> HashSet<String> {
        entries
            .iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect()
    }

    /// 规范化按通道的自然语言审批模式映射。
    ///
    /// 将通道名称转换为小写并去除空白，过滤无效条目。
    fn normalize_non_cli_natural_language_mode_by_channel(
        entries: &HashMap<String, NonCliNaturalLanguageApprovalMode>,
    ) -> HashMap<String, NonCliNaturalLanguageApprovalMode> {
        entries
            .iter()
            .filter_map(|(channel, mode)| {
                let normalized = channel.trim().to_ascii_lowercase();
                if normalized.is_empty() { None } else { Some((normalized, *mode)) }
            })
            .collect()
    }

    /// 从自主配置创建审批管理器实例。
    pub fn from_config(config: &AutonomyConfig) -> Self {
        Self {
            auto_approve: parking_lot::RwLock::new(config.auto_approve.iter().cloned().collect()),
            always_ask: parking_lot::RwLock::new(config.always_ask.iter().cloned().collect()),
            autonomy_level: config.level,
            session_allowlist: parking_lot::Mutex::new(HashSet::new()),
            non_cli_allowlist: parking_lot::Mutex::new(HashSet::new()),
            non_cli_allow_all_once_remaining: parking_lot::Mutex::new(0),
            non_cli_approval_approvers: parking_lot::RwLock::new(
                Self::normalize_non_cli_approvers(&config.non_cli_approval_approvers),
            ),
            non_cli_natural_language_approval_mode: parking_lot::RwLock::new(
                config.non_cli_natural_language_approval_mode,
            ),
            non_cli_natural_language_approval_mode_by_channel: parking_lot::RwLock::new(
                Self::normalize_non_cli_natural_language_mode_by_channel(
                    &config.non_cli_natural_language_approval_mode_by_channel,
                ),
            ),
            pending_non_cli_requests: parking_lot::Mutex::new(HashMap::new()),
            resolved_non_cli_requests: parking_lot::Mutex::new(HashMap::new()),
            audit_log: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// 检查工具调用是否需要交互式审批。
    pub fn needs_approval(&self, tool_name: &str) -> bool {
        if self.autonomy_level == AutonomyLevel::Full {
            return false;
        }

        if self.autonomy_level == AutonomyLevel::ReadOnly {
            return false;
        }

        if self.always_ask.read().contains(tool_name) {
            return true;
        }

        if self.auto_approve.read().contains(tool_name) {
            return false;
        }

        let allowlist = self.session_allowlist.lock();
        if allowlist.contains(tool_name) {
            return false;
        }

        true
    }

    /// 记录审批决策并更新会话状态。
    pub fn record_decision(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        decision: ApprovalResponse,
        channel: &str,
    ) {
        if decision == ApprovalResponse::Always {
            let mut allowlist = self.session_allowlist.lock();
            allowlist.insert(tool_name.to_string());
        }

        let summary = summarize_args(args);
        let entry = ApprovalLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            tool_name: tool_name.to_string(),
            arguments_summary: summary,
            decision,
            channel: channel.to_string(),
        };
        let mut log = self.audit_log.lock();
        log.push(entry);
    }

    /// 获取审计日志的快照。
    pub fn audit_log(&self) -> Vec<ApprovalLogEntry> {
        self.audit_log.lock().clone()
    }

    /// 获取当前会话白名单的副本。
    pub fn session_allowlist(&self) -> HashSet<String> {
        self.session_allowlist.lock().clone()
    }

    /// 授予特定工具的会话级非 CLI 审批权限。
    pub fn grant_non_cli_session(&self, tool_name: &str) {
        let mut allowlist = self.non_cli_allowlist.lock();
        allowlist.insert(tool_name.to_string());
    }

    /// 撤销特定工具的会话级非 CLI 审批权限。
    pub fn revoke_non_cli_session(&self, tool_name: &str) -> bool {
        let mut allowlist = self.non_cli_allowlist.lock();
        allowlist.remove(tool_name)
    }

    /// 检查特定工具是否已获得非 CLI 会话审批。
    pub fn is_non_cli_session_granted(&self, tool_name: &str) -> bool {
        let allowlist = self.non_cli_allowlist.lock();
        allowlist.contains(tool_name)
    }

    /// 获取当前非 CLI 会话白名单的副本。
    pub fn non_cli_session_allowlist(&self) -> HashSet<String> {
        self.non_cli_allowlist.lock().clone()
    }

    /// 授予一个非 CLI"一次性允许所有工具"令牌。
    pub fn grant_non_cli_allow_all_once(&self) -> u32 {
        let mut remaining = self.non_cli_allow_all_once_remaining.lock();
        *remaining = remaining.saturating_add(1);
        *remaining
    }

    /// 消费一个非 CLI"一次性允许所有工具"令牌。
    pub fn consume_non_cli_allow_all_once(&self) -> bool {
        let mut remaining = self.non_cli_allow_all_once_remaining.lock();
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        true
    }

    /// 获取剩余的非 CLI"一次性允许所有工具"令牌数。
    pub fn non_cli_allow_all_once_remaining(&self) -> u32 {
        *self.non_cli_allow_all_once_remaining.lock()
    }

    /// 获取配置的非 CLI 审批管理者列表的快照。
    pub fn non_cli_approval_approvers(&self) -> HashSet<String> {
        self.non_cli_approval_approvers.read().clone()
    }

    /// 获取非 CLI 审批管理命令的默认自然语言处理模式。
    pub fn non_cli_natural_language_approval_mode(&self) -> NonCliNaturalLanguageApprovalMode {
        *self.non_cli_natural_language_approval_mode.read()
    }

    /// 获取按通道的自然语言审批模式覆盖配置的快照。
    pub fn non_cli_natural_language_approval_mode_by_channel(
        &self,
    ) -> HashMap<String, NonCliNaturalLanguageApprovalMode> {
        self.non_cli_natural_language_approval_mode_by_channel.read().clone()
    }

    /// 获取特定通道的有效自然语言审批模式。
    pub fn non_cli_natural_language_approval_mode_for_channel(
        &self,
        channel: &str,
    ) -> NonCliNaturalLanguageApprovalMode {
        let normalized = channel.trim().to_ascii_lowercase();
        self.non_cli_natural_language_approval_mode_by_channel
            .read()
            .get(&normalized)
            .copied()
            .unwrap_or_else(|| self.non_cli_natural_language_approval_mode())
    }

    /// 检查指定通道上的发送者是否有权管理非 CLI 审批。
    pub fn is_non_cli_approval_actor_allowed(&self, channel: &str, sender: &str) -> bool {
        let approvers = self.non_cli_approval_approvers.read();

        if approvers.is_empty() {
            return true;
        }

        if approvers.contains("*") || approvers.contains(sender) {
            return true;
        }

        let exact = format!("{channel}:{sender}");
        if approvers.contains(&exact) {
            return true;
        }

        let any_on_channel = format!("{channel}:*");
        if approvers.contains(&any_on_channel) {
            return true;
        }

        let sender_any_channel = format!("*:{sender}");
        approvers.contains(&sender_any_channel)
    }

    /// 应用运行时 + 持久化的审批授予语义。
    pub fn apply_persistent_runtime_grant(&self, tool_name: &str) {
        {
            let mut auto = self.auto_approve.write();
            auto.insert(tool_name.to_string());
        }

        let mut always = self.always_ask.write();
        always.remove(tool_name);
    }

    /// 应用运行时 + 持久化的审批撤销语义。
    pub fn apply_persistent_runtime_revoke(&self, tool_name: &str) -> bool {
        let mut auto = self.auto_approve.write();
        auto.remove(tool_name)
    }

    /// 从配置热重载替换运行时持久化的非 CLI 策略。
    pub fn replace_runtime_non_cli_policy(
        &self,
        auto_approve: &[String],
        always_ask: &[String],
        non_cli_approval_approvers: &[String],
        non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode,
        non_cli_natural_language_approval_mode_by_channel: &HashMap<
            String,
            NonCliNaturalLanguageApprovalMode,
        >,
    ) {
        {
            let mut auto = self.auto_approve.write();
            *auto = auto_approve.iter().cloned().collect();
        }
        {
            let mut always = self.always_ask.write();
            *always = always_ask.iter().cloned().collect();
        }
        {
            let mut approvers = self.non_cli_approval_approvers.write();
            *approvers = Self::normalize_non_cli_approvers(non_cli_approval_approvers);
        }
        {
            let mut mode = self.non_cli_natural_language_approval_mode.write();
            *mode = non_cli_natural_language_approval_mode;
        }
        {
            let mut mode_by_channel =
                self.non_cli_natural_language_approval_mode_by_channel.write();
            *mode_by_channel = Self::normalize_non_cli_natural_language_mode_by_channel(
                non_cli_natural_language_approval_mode_by_channel,
            );
        }
    }

    /// 获取运行时自动批准工具列表的快照。
    pub fn auto_approve_tools(&self) -> HashSet<String> {
        self.auto_approve.read().clone()
    }

    /// 获取运行时始终询问工具列表的快照。
    pub fn always_ask_tools(&self) -> HashSet<String> {
        self.always_ask.read().clone()
    }

    /// 在 CLI 上提示用户并返回其决策。
    pub fn prompt_cli(&self, request: &ApprovalRequest) -> ApprovalResponse {
        prompt_cli_interactive(request)
    }
}
