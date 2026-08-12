use chrono_tz::{TZ_VARIANTS, Tz};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_TIMEZONE: &str = "Asia/Shanghai";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PreviewAutoSaveMode {
    Off,
    AfterDelay,
    #[default]
    OnFocusChange,
    OnWindowChange,
}

impl PreviewAutoSaveMode {
    pub const ALL: [Self; 4] =
        [Self::Off, Self::AfterDelay, Self::OnFocusChange, Self::OnWindowChange];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "关闭",
            Self::AfterDelay => "延迟保存",
            Self::OnFocusChange => "编辑器失焦时保存",
            Self::OnWindowChange => "窗口失焦时保存",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Off => "不自动保存，需手动保存当前文件。",
            Self::AfterDelay => "编辑内容变更后，等待短暂延迟再自动保存。",
            Self::OnFocusChange => "当编辑器失去焦点时，自动保存当前文件。",
            Self::OnWindowChange => "当应用窗口失去焦点时，自动保存当前文件。",
        }
    }
}

impl std::fmt::Display for PreviewAutoSaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct ModelRoute {
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickMessageGroup {
    #[default]
    General,
    Git,
}

impl QuickMessageGroup {
    pub const ALL: [Self; 2] = [Self::General, Self::Git];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "常用操作",
            Self::Git => "Git 操作",
        }
    }
}

impl std::fmt::Display for QuickMessageGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickMessageKind {
    #[default]
    Message,
    FillInput,
    CreateNewSession,
    SummarizeAndCreateNewSession,
    RefreshGitChanges,
    SelectAllGitChanges,
    InvertGitSelection,
    StageAllGitChanges,
    StageAllAndCommitGitChanges,
    CommitSelectedGitChanges,
}

impl QuickMessageKind {
    pub const ALL: [Self; 10] = [
        Self::FillInput,
        Self::Message,
        Self::CreateNewSession,
        Self::SummarizeAndCreateNewSession,
        Self::RefreshGitChanges,
        Self::SelectAllGitChanges,
        Self::InvertGitSelection,
        Self::StageAllGitChanges,
        Self::StageAllAndCommitGitChanges,
        Self::CommitSelectedGitChanges,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Message => "发送消息",
            Self::FillInput => "填入输入框",
            Self::CreateNewSession => "创建新会话",
            Self::SummarizeAndCreateNewSession => "压缩当前会话",
            Self::RefreshGitChanges => "刷新 Git 变更",
            Self::SelectAllGitChanges => "全选 Git 变更",
            Self::InvertGitSelection => "反选 Git 变更",
            Self::StageAllGitChanges => "暂存全部 Git 变更",
            Self::StageAllAndCommitGitChanges => "暂存并提交全部 Git 变更",
            Self::CommitSelectedGitChanges => "提交已选 Git 变更",
        }
    }

    pub fn action_badge(self) -> Option<&'static str> {
        match self {
            Self::Message => Some("发送"),
            Self::FillInput => None,
            Self::CreateNewSession => Some("新会话"),
            Self::SummarizeAndCreateNewSession => Some("压缩"),
            Self::RefreshGitChanges => Some("刷新"),
            Self::SelectAllGitChanges => Some("全选"),
            Self::InvertGitSelection => Some("反选"),
            Self::StageAllGitChanges => Some("暂存"),
            Self::StageAllAndCommitGitChanges => Some("暂存并提交"),
            Self::CommitSelectedGitChanges => Some("提交"),
        }
    }
}

impl std::fmt::Display for QuickMessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(default)]
pub struct QuickMessageConfig {
    pub text: String,
    pub group: QuickMessageGroup,
    pub kind: QuickMessageKind,
}

/// Desktop app UI configuration (`[app_ui]` section).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct AppUiConfig {
    /// System settings values managed by the desktop UI.
    #[serde(default)]
    pub system_settings: AppSystemSettingsConfig,
}

/// Desktop system settings persisted in agent config (`[app_ui.system_settings]`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AppSystemSettingsConfig {
    /// Desktop gateway client connection settings.
    #[serde(default)]
    pub gateway_client: GatewayClientSystemSettingsConfig,
    /// Iced theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub app_theme: String,
    /// IANA timezone used for chat date and time context.
    #[serde(default = "default_timezone", deserialize_with = "deserialize_timezone")]
    pub timezone: String,
    /// Terminal shell key (`bash` | `zsh`).
    #[serde(default = "default_terminal_shell")]
    pub terminal_shell: String,
    /// Terminal theme key (`system` | `ui` | `solarized_dark` | `monokai`).
    #[serde(default = "default_terminal_theme")]
    pub terminal_theme: String,
    /// Terminal font family.
    #[serde(default = "default_terminal_font_family")]
    pub terminal_font_family: String,
    /// Terminal font size.
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: f32,
    /// Whether editor theme follows system/app theme.
    #[serde(default = "default_true")]
    pub editor_follow_system_theme: bool,
    /// Iced editor theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub editor_theme: String,
    /// Per-project worktree toggle map.
    #[serde(default)]
    pub project_worktree_enabled: HashMap<String, bool>,
    /// Preview/editor font size.
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: f32,
    /// Preview/editor line height.
    #[serde(default = "default_editor_line_height")]
    pub editor_line_height: f32,
    /// Whether line height follows font size.
    #[serde(default = "default_true")]
    pub editor_auto_line_height: bool,
    /// Preview editor auto save mode.
    #[serde(default)]
    pub preview_auto_save: PreviewAutoSaveMode,
    #[serde(default = "default_false")]
    pub dialogue_flow_show_reasoning_summary: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_shell_tool_section: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_edit_tool_section: bool,
    /// Configurable shortcuts shown above the desktop chat input.
    #[serde(default = "default_quick_messages")]
    pub quick_messages: Vec<QuickMessageConfig>,
    /// Desktop-managed model routing rules.
    #[serde(default)]
    pub model_routes: Vec<ModelRoute>,
    /// TUI logical action overrides (`context`, `action`, `keys`).
    #[serde(default)]
    pub tui_keybindings: Vec<TuiKeybindingOverrideConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(default)]
pub struct TuiKeybindingOverrideConfig {
    pub context: String,
    pub action: String,
    pub keys: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientServerConfig {
    #[serde(default = "default_gateway_client_server_id")]
    pub id: String,
    #[serde(default = "default_gateway_client_server_name")]
    pub name: String,
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GatewayClientSystemSettingsConfig {
    #[serde(default = "default_gateway_client_host")]
    pub host: String,
    #[serde(default = "default_gateway_client_port")]
    pub port: u16,
    #[serde(default = "default_platform_id", deserialize_with = "deserialize_tenant_id")]
    pub platform_id: String,
    #[serde(default = "default_company_id", deserialize_with = "deserialize_tenant_id")]
    pub company_id: String,
    /// 旧版 `bearer_token` 字段，仅用于读取旧配置并迁移到 skey。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bearer_token: String,
    /// 旧版用户名字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    /// 旧版密码字段，仅用于读取旧配置。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skey: String,
    #[serde(default)]
    pub active_server_id: String,
    #[serde(default)]
    pub servers: Vec<GatewayClientServerConfig>,
}

fn default_app_theme_name() -> String {
    "Light".to_string()
}

pub fn default_timezone() -> String {
    DEFAULT_TIMEZONE.to_string()
}

pub fn timezone_options() -> Vec<&'static str> {
    TZ_VARIANTS.iter().map(|timezone| timezone.name()).collect()
}

fn deserialize_timezone<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value
        .parse::<Tz>()
        .map_err(|_| serde::de::Error::custom(format!("invalid IANA timezone: {value}")))?;
    Ok(value)
}

fn default_gateway_client_host() -> String {
    "127.0.0.1".to_string()
}

fn default_gateway_client_port() -> u16 {
    crate::gateway::DEFAULT_GATEWAY_PORT
}

fn default_platform_id() -> String {
    "10000".to_string()
}

fn default_company_id() -> String {
    "100000000".to_string()
}

fn deserialize_tenant_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("tenant id must not be empty"));
    }
    Ok(value)
}

fn default_gateway_client_server_id() -> String {
    "local".to_string()
}

fn default_gateway_client_server_name() -> String {
    "本地网关".to_string()
}

fn default_terminal_shell() -> String {
    "zsh".to_string()
}

fn default_terminal_theme() -> String {
    "system".to_string()
}

fn default_terminal_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_terminal_font_size() -> f32 {
    13.0
}

fn default_editor_font_size() -> f32 {
    14.0
}

fn default_editor_line_height() -> f32 {
    20.0
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

pub fn builtin_git_quick_messages() -> Vec<QuickMessageConfig> {
    vec![
        QuickMessageConfig {
            text: "刷新 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::RefreshGitChanges,
        },
        QuickMessageConfig {
            text: "全选 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::SelectAllGitChanges,
        },
        QuickMessageConfig {
            text: "反选 Git 变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::InvertGitSelection,
        },
        QuickMessageConfig {
            text: "暂存全部变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::StageAllGitChanges,
        },
        QuickMessageConfig {
            text: "暂存并提交全部变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::StageAllAndCommitGitChanges,
        },
        QuickMessageConfig {
            text: "提交已选变更".to_string(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::CommitSelectedGitChanges,
        },
    ]
}

pub fn builtin_general_quick_messages() -> Vec<QuickMessageConfig> {
    let send = |text: &str| QuickMessageConfig {
        text: text.to_string(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::Message,
    };
    let fill = |text: &str| QuickMessageConfig {
        text: text.to_string(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::FillInput,
    };
    let mut messages = vec![
        send("继续推进"),
        send("执行下一步"),
        fill("摘要并生成下一个任务的提示词"),
        QuickMessageConfig {
            text: "压缩当前会话".to_string(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::SummarizeAndCreateNewSession,
        },
        fill("为剩余任务制定开发计划"),
    ];
    messages.extend(
        [
            "不要兼容旧的代码",
            "修复单元测试",
            "给代码增加单元测试",
            "尽你最大的能力突破阻塞",
            "不对啊，完全不对啊，你仔细检查一下",
            "你实现得太简单了，做得细节一点",
            "多次修改都不对，你这一次需要仔细深度思考解决问题",
            "不要修改代码，这次只做分析和计划",
            "从其他语言移植代码，需要尽量一一对齐，不要尝试简化实现",
            "从第一个任务开始，直到所有任务全部完成才结束，不要轻易结束。",
            "这是一个开放性的需求，你可以自由发挥，做得细节一点，希望给我惊喜",
            "请选择合适的技能，给我制定计划，但是不要急着修改代码",
            "先复述你理解的目标、约束和验收标准，确认没有遗漏后再开始",
            "先定位根因和影响范围，不要用临时绕过掩盖底层问题",
            "请沿完整调用链检查数据流、状态流和错误流，再决定修改位置",
            "请主动覆盖边界条件、并发时序、空状态和失败恢复",
            "完成后逐条对照需求做自我验收，有偏差就继续修正",
            "请检查是否破坏现有行为，并补充能防止回归的测试",
            "请优先复用现有架构和组件，避免无必要的新抽象与依赖",
            "请站在最终用户视角打磨交互、文案、反馈和异常体验",
            "对不确定的假设先用代码和测试验证，不要凭记忆直接实现",
            "交付前检查修改范围、格式、静态检查和相关测试，明确报告结果",
        ]
        .into_iter()
        .map(fill),
    );
    messages
}

pub fn append_missing_builtin_general_quick_messages(messages: &mut Vec<QuickMessageConfig>) {
    if messages.is_empty() {
        return;
    }

    let mut insert_at = messages
        .iter()
        .position(|item| item.group == QuickMessageGroup::Git)
        .unwrap_or(messages.len());
    for builtin in builtin_general_quick_messages() {
        if !messages.iter().any(|item| {
            item.group == builtin.group
                && item.kind == builtin.kind
                && item.text.trim() == builtin.text
        }) {
            messages.insert(insert_at, builtin);
            insert_at += 1;
        }
    }
}

pub fn append_missing_builtin_git_quick_messages(messages: &mut Vec<QuickMessageConfig>) {
    if messages.is_empty() {
        return;
    }

    for builtin in builtin_git_quick_messages() {
        if !messages.iter().any(|item| item.kind == builtin.kind) {
            messages.push(builtin);
        }
    }
}

pub fn upgrade_legacy_quick_message_actions(messages: &mut [QuickMessageConfig]) {
    let fill_input_texts = builtin_general_quick_messages()
        .into_iter()
        .filter(|item| item.kind == QuickMessageKind::FillInput)
        .map(|item| item.text)
        .collect::<Vec<_>>();

    for item in messages {
        if matches!(item.text.trim(), "摘要并创建新会话" | "压缩当前会话")
            && matches!(
                item.kind,
                QuickMessageKind::CreateNewSession | QuickMessageKind::SummarizeAndCreateNewSession
            )
        {
            item.kind = QuickMessageKind::SummarizeAndCreateNewSession;
            item.text = "压缩当前会话".to_string();
        } else if item.group == QuickMessageGroup::General
            && item.kind == QuickMessageKind::Message
            && fill_input_texts.iter().any(|text| item.text.trim() == text)
        {
            item.kind = QuickMessageKind::FillInput;
        }
    }
}

fn default_quick_messages() -> Vec<QuickMessageConfig> {
    let mut messages = builtin_general_quick_messages();
    messages.extend(builtin_git_quick_messages());
    messages
}

impl Default for AppSystemSettingsConfig {
    fn default() -> Self {
        Self {
            gateway_client: GatewayClientSystemSettingsConfig::default(),
            app_theme: default_app_theme_name(),
            timezone: default_timezone(),
            terminal_shell: default_terminal_shell(),
            terminal_theme: default_terminal_theme(),
            terminal_font_family: default_terminal_font_family(),
            terminal_font_size: default_terminal_font_size(),
            editor_follow_system_theme: default_true(),
            editor_theme: default_app_theme_name(),
            project_worktree_enabled: HashMap::new(),
            editor_font_size: default_editor_font_size(),
            editor_line_height: default_editor_line_height(),
            editor_auto_line_height: default_true(),
            preview_auto_save: PreviewAutoSaveMode::default(),
            dialogue_flow_show_reasoning_summary: default_false(),
            dialogue_flow_expand_shell_tool_section: default_false(),
            dialogue_flow_expand_edit_tool_section: default_false(),
            quick_messages: default_quick_messages(),
            model_routes: Vec::new(),
            tui_keybindings: Vec::new(),
        }
    }
}

impl Default for GatewayClientSystemSettingsConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
            active_server_id: default_gateway_client_server_id(),
            servers: vec![GatewayClientServerConfig::default()],
        }
    }
}

impl Default for GatewayClientServerConfig {
    fn default() -> Self {
        Self {
            id: default_gateway_client_server_id(),
            name: default_gateway_client_server_name(),
            host: default_gateway_client_host(),
            port: default_gateway_client_port(),
            platform_id: default_platform_id(),
            company_id: default_company_id(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: String::new(),
        }
    }
}

impl GatewayClientSystemSettingsConfig {
    pub fn normalized_servers(&self) -> Vec<GatewayClientServerConfig> {
        if self.servers.is_empty() {
            return vec![GatewayClientServerConfig {
                id: default_gateway_client_server_id(),
                name: default_gateway_client_server_name(),
                host: self.host.clone(),
                port: self.port,
                platform_id: self.platform_id.clone(),
                company_id: self.company_id.clone(),
                bearer_token: self.bearer_token.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
                skey: self.skey.clone(),
            }];
        }

        self.servers
            .iter()
            .enumerate()
            .map(|(index, server)| {
                let mut server = server.clone();
                if server.id.trim().is_empty() {
                    server.id = if index == 0 {
                        default_gateway_client_server_id()
                    } else {
                        format!("gateway-{}", index + 1)
                    };
                }
                if server.name.trim().is_empty() {
                    server.name = if index == 0 {
                        default_gateway_client_server_name()
                    } else {
                        format!("网关 {}", index + 1)
                    };
                }
                server
            })
            .collect()
    }

    pub fn active_server(&self) -> GatewayClientServerConfig {
        let servers = self.normalized_servers();
        servers
            .iter()
            .find(|server| server.id == self.active_server_id)
            .cloned()
            .or_else(|| servers.first().cloned())
            .unwrap_or_default()
    }

    pub fn set_servers(
        &mut self,
        servers: Vec<GatewayClientServerConfig>,
        active_server_id: String,
    ) {
        let servers =
            if servers.is_empty() { vec![GatewayClientServerConfig::default()] } else { servers };
        let active = servers
            .iter()
            .find(|server| server.id == active_server_id)
            .or_else(|| servers.first())
            .cloned()
            .unwrap_or_default();

        self.active_server_id = active.id.clone();
        self.host = active.host.clone();
        self.port = active.port;
        self.platform_id = active.platform_id.clone();
        self.company_id = active.company_id.clone();
        self.bearer_token = active.bearer_token.clone();
        self.username = active.username.clone();
        self.password = active.password.clone();
        self.skey = active.skey.clone();
        self.servers = servers;
    }
}
#[cfg(test)]
#[path = "ui_tests.rs"]
mod ui_tests;
