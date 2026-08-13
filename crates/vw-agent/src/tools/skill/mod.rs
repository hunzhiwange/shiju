//! 技能调用工具
//!
//! 加载并执行预定义的技能脚本。技能是可重用的工作流模板，
//! 用于封装常见的任务模式和最佳实践。

use super::context::{ToolUseContext, current_tool_use_context};
use super::traits::{Tool, ToolCallResult, ToolResult};
use crate::app::agent::agent;
use crate::app::agent::config;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::question;
use crate::app::agent::security::{SecurityPolicy, policy::ToolOperation};
use crate::app::agent::skills;
use crate::session::capability_policy::resolve_skill_capabilities;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

mod invocation;
mod prompt;

/// skill 工具的参数结构体
///
/// 用于接收技能名称参数，通过反序列化从 JSON 值构造
#[derive(Debug, Clone, Deserialize)]
struct Args {
    /// 要加载的技能名称
    #[serde(alias = "name", alias = "skill_name")]
    skill: String,
    /// 可选的技能调用参数。
    #[serde(default)]
    args: Option<String>,
}

/// 技能调用工具
///
/// 提供技能加载和执行功能。技能是预定义的工作流模板，
/// 用于封装常见的任务模式和最佳实践，可以通过此工具动态加载到对话上下文中。
///
/// # 功能特性
///
/// - 加载预定义的技能脚本和工作流
/// - 权限检查和用户确认机制
/// - 技能文件列表抽样展示
/// - 支持不同运行模式（普通模式和强制提示模式）
///
/// # 示例
///
/// ```rust
/// use std::sync::Arc;
/// use crate::app::agent::tools::skill::SkillTool;
/// use crate::app::agent::security::SecurityPolicy;
///
/// let security = Arc::new(SecurityPolicy::default());
/// let tool = SkillTool::new(security, "session-123".to_string());
/// ```
#[derive(Clone)]
pub struct SkillTool {
    /// 安全策略，用于检查工具操作权限
    security: Arc<SecurityPolicy>,
    /// 会话ID，用于权限确认交互
    session_id: String,
    /// 是否在 Allow 模式下强制提示用户确认
    force_prompt_on_allow: bool,
    /// 当前工具运行的工作区目录，用于解析本地技能配置。
    workspace_dir: PathBuf,
    /// 当前运行时配置，用于读取技能开关、目录与提示注入模式。
    root_config: Arc<config::Config>,
    /// 当前实例的工具描述文本。
    description_text: Arc<str>,
}

impl SkillTool {
    /// 创建新的 SkillTool 实例（普通模式）
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的 Arc 引用，用于权限检查
    /// - `session_id`: 会话标识符，用于权限确认时的交互
    ///
    /// # 返回值
    ///
    /// 返回配置为普通模式的 SkillTool 实例（force_prompt_on_allow = false）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let tool = SkillTool::new(security, "session-123".to_string());
    /// ```
    pub fn new(security: Arc<SecurityPolicy>, session_id: String) -> Self {
        Self::new_with_mode(security, session_id, false)
    }

    /// 创建新的 SkillTool 实例（可配置强制提示模式）
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的 Arc 引用，用于权限检查
    /// - `session_id`: 会话标识符，用于权限确认时的交互
    /// - `force_prompt_on_allow`: 是否在 Allow 模式下强制提示用户确认
    ///   - `true`: 即使权限规则允许，也会提示用户确认（用于 wildcard "*" 规则）
    ///   - `false`: 按权限规则正常处理
    ///
    /// # 返回值
    ///
    /// 返回配置好的 SkillTool 实例
    ///
    /// # 使用场景
    ///
    /// 当需要对通配符权限规则（如 "skill": "*"）进行额外的用户确认时使用
    pub fn new_with_mode(
        security: Arc<SecurityPolicy>,
        session_id: String,
        force_prompt_on_allow: bool,
    ) -> Self {
        let workspace_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        #[cfg(not(target_arch = "wasm32"))]
        let root_config = Arc::new(block_on(config::get()));
        #[cfg(target_arch = "wasm32")]
        let root_config = Arc::new(config::Config::default());

        Self::new_with_runtime_config(
            security,
            session_id,
            force_prompt_on_allow,
            workspace_dir,
            root_config,
        )
    }

    /// 使用指定工作区和运行时配置创建 SkillTool。
    pub(crate) fn new_with_runtime_config(
        security: Arc<SecurityPolicy>,
        session_id: String,
        force_prompt_on_allow: bool,
        workspace_dir: PathBuf,
        root_config: Arc<config::Config>,
    ) -> Self {
        let description_text =
            Arc::<str>::from(build_description_text(&workspace_dir, root_config.as_ref()));
        Self {
            security,
            session_id,
            force_prompt_on_allow,
            workspace_dir,
            root_config,
            description_text,
        }
    }

    /// 用请求边界已确定的策略刷新模型可见的技能目录。
    pub(crate) fn with_skill_directory_context(mut self, context: &ToolUseContext) -> Self {
        let skills = filter_delegate_skills_for_context(
            skills::load_skills_with_config(&self.workspace_dir, self.root_config.as_ref()),
            context,
        );
        #[cfg(not(target_arch = "wasm32"))]
        let skills = accessible_skills(skills, Some(context));
        self.description_text = Arc::<str>::from(prompt::build_description_text_for_skills(
            &self.workspace_dir,
            skills,
        ));
        self
    }

    /// 生成工具参数的 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回 JSON Schema 格式的参数定义，包含：
    /// - type: "object"
    /// - properties.name: 字符串类型的技能名称
    /// - required: ["name"] - name 为必填字段
    ///
    /// # 示例
    ///
    /// 返回值示例：
    /// ```json
    /// {
    ///     "type": "object",
    ///     "properties": {
    ///         "name": { "type": "string" }
    ///     },
    ///     "required": ["name"]
    /// }
    /// ```
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string" },
                "args": { "type": "string" }
            },
            "required": ["skill"]
        })
    }

    /// 请求技能使用权限
    ///
    /// 根据安全策略和权限规则，检查是否允许使用指定的技能。
    /// 如果需要用户确认，会通过问答系统提示用户进行授权决策。
    ///
    /// # 参数
    ///
    /// - `name`: 要加载的技能名称
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 权限检查通过，允许使用该技能
    /// - `Err(String)`: 权限被拒绝，返回错误信息说明原因
    ///
    /// # 权限处理流程
    ///
    /// 1. 获取当前代理的权限规则集
    /// 2. 评估 "skill" 类型的权限规则
    /// 3. 如果启用了 force_prompt_on_allow 且规则为通配符允许，则强制转为 Ask 模式
    /// 4. 根据评估结果执行：
    ///    - Allow: 直接返回成功
    ///    - Deny: 返回权限不足错误
    ///    - Ask: 提示用户选择 once/always/reject
    ///      - once: 本次允许
    ///      - always: 将权限规则写入配置文件
    ///      - reject: 拒绝授权
    ///
    /// # 错误情况
    ///
    /// - 权限规则为 Deny
    /// - 用户选择拒绝
    /// - 配置更新失败
    async fn ask_skill_permission(&self, name: &str) -> Result<(), String> {
        let agent_name = agent::default_agent().await.unwrap_or_else(|_| "main".to_string());
        let ruleset = match agent::permission_rules(&agent_name).await {
            Some(ruleset) => ruleset,
            None if agent_name != "main" => {
                agent::permission_rules("main").await.unwrap_or_default()
            }
            None => Default::default(),
        };

        // 评估权限规则
        let mut rule = permission_next::evaluate("skill", name, &[ruleset]);

        // 如果启用了强制提示模式，且规则为通配符允许，则转为 Ask 模式
        // 这确保了即使有宽松的权限规则，也会让用户明确确认
        if self.force_prompt_on_allow
            && rule.action == permission_next::Action::Allow
            && rule.pattern == "*"
        {
            rule.action = permission_next::Action::Ask;
        }

        match rule.action {
            permission_next::Action::Allow => Ok(()),
            permission_next::Action::Deny => {
                Err(format!("Denied: 权限不足：不允许使用权限：skill（目标：{}）", name))
            }
            permission_next::Action::Ask => {
                // 构造用户询问信息
                let question_text = format!("需要使用权限：skill\n\n目标:\n{}\n\n是否允许？", name);
                let questions = vec![question::Info {
                    question: question_text,
                    header: "skill".to_string(),
                    options: vec![
                        question::OptionInfo {
                            label: "once".to_string(),
                            description: "允许一次".to_string(),
                            preview: None,
                        },
                        question::OptionInfo {
                            label: "always".to_string(),
                            description: "始终允许".to_string(),
                            preview: None,
                        },
                        question::OptionInfo {
                            label: "reject".to_string(),
                            description: "拒绝".to_string(),
                            preview: None,
                        },
                    ],
                    multiple: Some(false),
                    custom: Some(false),
                }];

                // 发起权限确认询问
                let answers = question::ask(question::AskInput {
                    session_id: self.session_id.clone(),
                    questions,
                    tool: None,
                })
                .await
                .map_err(|e| format!("Denied: {e}"))?;

                // 处理用户选择
                let choice =
                    answers.first().and_then(|a| a.first()).map(|s| s.as_str()).unwrap_or("reject");
                match choice {
                    "once" => Ok(()),
                    "always" => {
                        // 将权限规则持久化到配置文件
                        let mut map = serde_json::Map::new();
                        map.insert(name.to_string(), Value::String("allow".to_string()));
                        let patch =
                            serde_json::json!({ "permission": { "skill": Value::Object(map) } });
                        config::update(patch).await.map_err(|e| e.to_string())
                    }
                    _ => Err(format!("Denied: 用户拒绝授权：skill（目标：{}）", name)),
                }
            }
        }
    }

    async fn invoke(&self, input: Value) -> ToolCallResult {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = input;
            return failure_result("skill 在 Web 版本不可用".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let requested_name = input
                .get("skill")
                .or_else(|| input.get("name"))
                .or_else(|| input.get("skill_name"))
                .and_then(Value::as_str);
            let explicitly_requested = requested_name.is_some_and(skill_request_is_explicit);
            if !explicitly_requested
                && let Err(error) =
                    self.security.enforce_tool_operation(ToolOperation::Act, "skill")
            {
                return failure_result(error);
            }
            let args: Args = match serde_json::from_value(input) {
                Ok(args) => args,
                Err(error) => {
                    return failure_result(format!("Missing or invalid parameters: {error}"));
                }
            };
            let skills = skills::load_skills_full_with_config(
                &self.workspace_dir,
                self.root_config.as_ref(),
            );
            let skills = filter_delegate_skills(skills);
            let Some(skill) =
                skills.iter().find(|skill| skill_matches_name(skill, &args.skill)).cloned()
            else {
                return failure_result(format!(
                    "未找到技能 \"{}\"。可用技能：{}",
                    args.skill,
                    available_skill_names(&skills)
                ));
            };
            if explicitly_requested && !skill.user_invocable {
                return failure_result(format!("技能 \"{}\" 禁止用户显式调用", skill.name));
            }
            if skill.disable_model_invocation && !explicitly_requested {
                return failure_result(format!("技能 \"{}\" 禁止模型调用", skill.name));
            }
            if !explicitly_requested
                && let Err(error) = self.ask_skill_permission(&skill.name).await
            {
                return failure_result(error);
            }
            match invocation::build_invocation_result(
                &skill,
                &self.workspace_dir,
                args.args.as_deref(),
                explicitly_requested,
            ) {
                Ok(result) => result,
                Err(error) => failure_result(error),
            }
        }
    }
}

fn filter_delegate_skills(skills: Vec<skills::Skill>) -> Vec<skills::Skill> {
    let Some(context) = current_tool_use_context() else {
        return skills;
    };
    filter_delegate_skills_for_context(skills, context.as_ref())
}

fn filter_delegate_skills_for_context(
    skills: Vec<skills::Skill>,
    context: &ToolUseContext,
) -> Vec<skills::Skill> {
    let Some((policy, defaults)) = context.delegate_skill_policy() else {
        return skills;
    };
    let trusted = context.trusted_user_selected_skills();
    let visible = resolve_skill_capabilities(
        skills.iter().map(|skill| skill.name.as_str()),
        defaults.iter().map(String::as_str),
        trusted.iter().map(String::as_str),
        policy,
        std::iter::empty::<&str>(),
    )
    .visible_ids;
    skills
        .into_iter()
        .filter(|skill| visible.contains(&skill.name.trim().to_lowercase()))
        .collect()
}

/// 从可信用户输入和已加载仓库协议中找出明确引用的技能名称及文件 ID。
pub(crate) fn explicitly_referenced_skill_names<'a>(
    skills: &[skills::Skill],
    sources: impl IntoIterator<Item = &'a str>,
) -> BTreeSet<String> {
    let sources = sources.into_iter().collect::<Vec<_>>();
    let mut requested = BTreeSet::new();
    for skill in skills {
        let mut names = vec![skill.name.as_str()];
        let file_id = skill_file_id(skill);
        if let Some(file_id) = file_id.as_deref()
            && !names.iter().any(|name| name.eq_ignore_ascii_case(file_id))
        {
            names.push(file_id);
        }
        if !names
            .iter()
            .any(|name| sources.iter().any(|source| text_contains_skill_reference(source, name)))
        {
            continue;
        }
        requested.extend(names.into_iter().map(|name| name.trim().to_lowercase()));
    }
    requested
}

/// 判断真实用户输入是否明确要求本轮使用技能，但没有指定技能名称。
pub(crate) fn user_explicitly_requests_skill_use(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    if text.is_empty() {
        return false;
    }
    let denied = [
        "不要使用技能",
        "不要加载技能",
        "不要调用技能",
        "别使用技能",
        "别加载技能",
        "禁止使用技能",
        "无需使用技能",
        "不必使用技能",
        "不使用技能",
        "不加载技能",
        "do not use skill",
        "do not load skill",
        "don't use skill",
        "don't load skill",
        "never use skill",
        "without using skill",
    ];
    if denied.iter().any(|phrase| text.contains(phrase)) {
        return false;
    }

    let has_skill_noun = text.contains("技能")
        || ["skill", "skills"]
            .iter()
            .any(|noun| text_contains_skill_reference(&text, noun));
    if !has_skill_noun {
        return false;
    }
    let has_action = ["使用", "加载", "调用", "启用", "运行", "执行"]
        .iter()
        .any(|action| text.contains(action))
        || ["use", "load", "invoke", "enable", "run", "execute"]
            .iter()
            .any(|action| text_contains_skill_reference(&text, action));
    if !has_action {
        return false;
    }

    let directive_markers = ["请", "先", "必须", "务必", "直接", "现在", "帮我", "please", "must", "first"];
    directive_markers.iter().any(|marker| text.contains(marker))
        || ["使用", "加载", "调用", "启用", "运行", "执行", "use", "load", "invoke", "enable", "run", "execute"]
            .iter()
            .any(|action| text.starts_with(action))
}

/// 返回本轮可由用户泛化授权显式调用的技能名称及文件 ID。
pub(crate) fn user_invocable_skill_names(skills: &[skills::Skill]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for skill in skills.iter().filter(|skill| skill.user_invocable) {
        names.insert(skill.name.trim().to_lowercase());
        if let Some(file_id) = skill_file_id(skill) {
            names.insert(file_id.trim().to_lowercase());
        }
    }
    names.retain(|name| !name.is_empty());
    names
}

fn skill_request_is_explicit(name: &str) -> bool {
    let Some(context) = current_tool_use_context() else {
        return false;
    };
    if context.is_skill_explicitly_requested(name) {
        return true;
    }
    context
        .messages_view()
        .and_then(|messages| {
            messages.iter().rev().find(|message| {
                message.role == "user" && !message.content.trim().starts_with("<system-reminder>")
            })
        })
        .is_some_and(|message| text_contains_skill_reference(&message.content, name))
}

fn text_contains_skill_reference(text: &str, name: &str) -> bool {
    let text = text.to_lowercase();
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return false;
    }
    text.match_indices(&name).any(|(start, _)| {
        let end = start + name.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        before.is_none_or(|ch| !is_skill_name_char(ch))
            && after.is_none_or(|ch| !is_skill_name_char(ch))
    })
}

fn is_skill_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '-' | '_' | ':')
}

fn failure_result(error: String) -> ToolCallResult {
    ToolCallResult {
        data: json!({ "success": false, "error": error }),
        model_result: Value::String(error),
        telemetry: Some(super::traits::ToolCallTelemetry {
            success: false,
            ..super::traits::ToolCallTelemetry::default()
        }),
        ..ToolCallResult::default()
    }
}

/// 生成工具描述文本（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
fn build_description_text(workspace_dir: &Path, root_config: &config::Config) -> String {
    prompt::build_description_text(workspace_dir, root_config)
}

/// 生成工具描述文本（WASM 平台）
///
/// 在 WebAssembly 环境下返回简化的英文描述。
/// 由于 WASM 版本不支持技能加载，仅返回基本说明。
///
/// # 返回值
///
/// 返回简化的英文工具描述
#[cfg(target_arch = "wasm32")]
fn build_description_text(_workspace_dir: &Path, _root_config: &config::Config) -> String {
    prompt::build_description_text(_workspace_dir, _root_config)
}

#[cfg(not(target_arch = "wasm32"))]
fn accessible_skills(
    skills: Vec<skills::Skill>,
    context: Option<&ToolUseContext>,
) -> Vec<skills::Skill> {
    let agent_name = block_on(agent::default_agent()).unwrap_or_else(|_| "main".to_string());
    let ruleset = block_on(agent::permission_rules(&agent_name))
        .or_else(|| block_on(agent::permission_rules("main")))
        .unwrap_or_default();

    skills
        .into_iter()
        .filter(|skill| {
            context.is_some_and(|context| skill_is_explicitly_requested(context, skill))
                || !matches!(
                    permission_next::evaluate(
                        "skill",
                        &skill.name,
                        std::slice::from_ref(&ruleset),
                    )
                    .action,
                    permission_next::Action::Deny
                )
        })
        .collect()
}

fn skill_is_explicitly_requested(context: &ToolUseContext, skill: &skills::Skill) -> bool {
    context.is_skill_explicitly_requested(&skill.name)
        || skill_file_id(skill)
            .as_deref()
            .is_some_and(|file_id| context.is_skill_explicitly_requested(file_id))
}

fn skill_file_id(skill: &skills::Skill) -> Option<String> {
    let location = skill.location.as_ref()?;
    let file_name = location.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if file_name.eq_ignore_ascii_case("SKILL.md") || file_name.eq_ignore_ascii_case("SKILL.toml") {
        return location
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
    }

    location.file_stem().and_then(|name| name.to_str()).map(ToOwned::to_owned)
}

fn skill_matches_name(skill: &skills::Skill, requested: &str) -> bool {
    skill.name == requested || skill_file_id(skill).as_deref() == Some(requested)
}

fn skill_location(skill: &skills::Skill, workspace_dir: &Path) -> PathBuf {
    skill
        .location
        .clone()
        .unwrap_or_else(|| workspace_dir.join("skills").join(&skill.name).join("SKILL.md"))
}

fn available_skill_names(skills: &[skills::Skill]) -> String {
    let mut names = BTreeSet::new();
    for skill in skills {
        names.insert(skill.name.clone());
        if let Some(id) = skill_file_id(skill) {
            names.insert(id);
        }
    }

    if names.is_empty() {
        "无".to_string()
    } else {
        names.into_iter().collect::<Vec<_>>().join(", ")
    }
}

fn skill_prompt_content(skill: &skills::Skill) -> String {
    prompt::skill_prompt_content(skill, None)
}

fn xml_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// 在同步上下文中执行异步操作（非 WASM 平台）
///
/// 提供在同步环境中执行异步代码的能力，用于在 Lazy 初始化等场景中调用异步函数。
///
/// # 参数
///
/// - `fut`: 要执行的 Future
///
/// # 返回值
///
/// 返回 Future 的执行结果
///
/// # 实现策略
///
/// 1. 如果当前已经在 Tokio 运行时中，使用独立作用域线程承载临时运行时
/// 2. 如果不在运行时中，创建新的单线程运行时执行 Future
///
/// # Panics
///
/// 当无法创建新的 Tokio 运行时可能 panic
#[cfg(not(target_arch = "wasm32"))]
fn block_on<F>(fut: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(1)
                        .enable_all()
                        .build()
                        .expect("failed to build tokio runtime")
                        .block_on(fut)
                })
                .join()
                .expect("tokio bridge thread panicked")
        })
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(fut)
    }
}

/// 将文件路径转换为 file:// URL 格式（非 WASM 平台）
///
/// 将相对或绝对文件路径转换为标准的 file:// URL 格式，
/// 用于在技能描述中提供可点击的文件链接。
///
/// # 参数
///
/// - `path`: 文件路径字符串
///
/// # 返回值
///
/// 返回 file:// URL 格式的字符串
///
/// # 示例
///
/// ```rust
/// let url = file_url("/path/to/file.md");
/// // 返回: "file:///path/to/file.md"
/// ```
#[cfg(not(target_arch = "wasm32"))]
fn file_url(path: &Path) -> String {
    let p = path.to_path_buf();
    // 转换为绝对路径
    let abs = if p.is_absolute() {
        p
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p)
    };
    // 构造 file:// URL，统一使用正斜杠
    format!("file:///{}", abs.to_string_lossy().replace('\\', "/"))
}

/// 将目录路径转换为 file:// URL 格式（带尾部斜杠）（非 WASM 平台）
///
/// 将目录路径转换为 file:// URL，并确保以斜杠结尾，
/// 用于表示技能的基础目录 URL。
///
/// # 参数
///
/// - `dir`: 目录路径的 PathBuf 引用
///
/// # 返回值
///
/// 返回以斜杠结尾的 file:// URL 字符串
///
/// # 示例
///
/// ```rust
/// let url = base_dir_url(&PathBuf::from("/path/to/skill"));
/// // 返回: "file:///path/to/skill/"
/// ```
#[cfg(not(target_arch = "wasm32"))]
fn base_dir_url(dir: &Path) -> String {
    let mut s = format!("file:///{}", dir.to_string_lossy().replace('\\', "/"));
    // 确保以斜杠结尾
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

/// 为 SkillTool 实现 Tool trait
///
/// 提供技能工具的标准接口实现，包括工具名称、描述、参数 schema 和执行逻辑。
/// 该实现区分 WASM 和非 WASM 平台，在 WebAssembly 环境下禁用技能加载功能。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for SkillTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 固定返回 "skill"
    fn name(&self) -> &str {
        "skill"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    ///
    /// 返回动态生成的工具描述文本，包含可用技能列表和使用指南
    fn description(&self) -> &str {
        self.description_text.as_ref()
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回定义了 "name" 参数的 JSON Schema
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    /// 执行技能加载操作
    ///
    /// 根据提供的参数加载指定的技能，并将技能内容注入到对话上下文中。
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数，必须包含 "name" 字段
    ///
    /// # 返回值
    ///
    /// 返回 ToolResult，包含：
    /// - success: true 表示成功，false 表示失败
    /// - output: 成功时返回技能内容（XML 格式）
    /// - error: 失败时返回错误信息
    ///
    /// # 执行流程（非 WASM 平台）
    ///
    /// 1. 检查安全策略，验证是否有执行权限
    /// 2. 解析参数，获取技能名称
    /// 3. 查找技能，如果不存在则返回可用技能列表
    /// 4. 请求权限确认（如需要）
    /// 5. 加载技能内容和相关文件列表
    /// 6. 返回格式化的技能内容
    ///
    /// # 错误情况
    ///
    /// - WASM 平台：返回 "skill 在 Web 版本不可用"
    /// - 安全策略拒绝：返回权限错误
    /// - 参数无效：返回参数错误信息
    /// - 技能不存在：返回可用技能列表
    /// - 权限被拒绝：返回权限拒绝信息
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let result = self.invoke(args).await;
        if result.is_success() {
            Ok(ToolResult { success: true, output: result.model_text(), error: None })
        } else {
            Ok(ToolResult { success: false, output: String::new(), error: result.error_text() })
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        Ok(self.invoke(input).await)
    }
}

/// 单元测试模块
///
/// 测试文件位于 tests/skill.rs
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "invocation_tests.rs"]
mod invocation_tests;
