//! 技能调用工具
//!
//! 加载并执行预定义的技能脚本。技能是可重用的工作流模板，
//! 用于封装常见的任务模式和最佳实践。

use super::traits::{Tool, ToolResult};
use crate::app::agent::agent;
use crate::app::agent::config;
use crate::app::agent::file::ripgrep;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::question;
use crate::app::agent::security::{SecurityPolicy, policy::ToolOperation};
use crate::app::agent::skills;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const SKILL_EXECUTION_POLICY: &str = r#"<skill_execution_policy>
严格执行要求：
1. 除非用户明确要求跳过、修改或重排某个具体步骤，否则必须按本技能规定的顺序完整执行全部工作流。
2. 不得静默跳过、合并、替代或虚构任何步骤；不得因为已有部分信息而省略技能要求的验证步骤。
3. 每完成一步，必须核对该步骤的要求和实际结果，再进入下一步。
4. 如果任一步骤无法执行，必须立即停止并明确报告阻塞原因；不得继续后续步骤，也不得声称任务已经完成。
5. 输出最终结果前，必须逐项确认所有必需步骤均已完成，并具有实际执行依据。
</skill_execution_policy>"#;

/// skill 工具的参数结构体
///
/// 用于接收技能名称参数，通过反序列化从 JSON 值构造
#[derive(Debug, Clone, Deserialize)]
struct Args {
    /// 要加载的技能名称
    name: String,
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
                "name": { "type": "string" }
            },
            "required": ["name"]
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
}

/// 生成工具描述文本（非 WASM 平台）
#[cfg(not(target_arch = "wasm32"))]
fn build_description_text(workspace_dir: &Path, root_config: &config::Config) -> String {
    let skills = skills::load_skills_with_config(workspace_dir, root_config);
    let accessible = accessible_skills(skills);

    if accessible.is_empty() {
        return "加载一个专用技能，以获得特定领域的指导与工作流。目前没有可用技能。".to_string();
    }

    let mut lines = vec![
        "加载一个专用技能，以获得特定领域的指导与工作流。".to_string(),
        String::new(),
        "当任务匹配下面任一技能时，调用该工具来加载对应技能。".to_string(),
        "技能会把详细指令、工作流，以及随附资源的访问方式注入到对话上下文中。".to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in accessible {
        lines.push("  <skill>".to_string());
        lines.push(format!("    <name>{}</name>", xml_escape(&skill.name)));
        if let Some(id) = skill_file_id(&skill)
            && id != skill.name
        {
            lines.push(format!("    <id>{}</id>", xml_escape(&id)));
        }
        lines.push(format!("    <description>{}</description>", xml_escape(&skill.description)));
        let location = skill_location(&skill, workspace_dir);
        lines.push(format!("    <location>{}</location>", file_url(&location)));
        lines.push("  </skill>".to_string());
    }
    lines.push("</available_skills>".to_string());
    lines.join("\n")
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
    "Load a specialized skill with domain-specific instructions and workflow context.".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn accessible_skills(skills: Vec<skills::Skill>) -> Vec<skills::Skill> {
    let agent_name = block_on(agent::default_agent()).unwrap_or_else(|_| "main".to_string());
    let ruleset = block_on(agent::permission_rules(&agent_name))
        .or_else(|| block_on(agent::permission_rules("main")))
        .unwrap_or_default();

    skills
        .into_iter()
        .filter(|skill| {
            !matches!(
                permission_next::evaluate("skill", &skill.name, std::slice::from_ref(&ruleset))
                    .action,
                permission_next::Action::Deny
            )
        })
        .collect()
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
    let content = skill
        .prompts
        .iter()
        .map(|prompt| prompt.trim())
        .filter(|prompt| !prompt.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let content = if content.is_empty() { "该技能没有内联指令。" } else { &content };
    format!("{SKILL_EXECUTION_POLICY}\n\n{content}")
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
        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("skill 在 Web 版本不可用".to_string()),
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 1. 安全策略检查
            if let Err(error) = self.security.enforce_tool_operation(ToolOperation::Act, "skill") {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error),
                });
            }

            // 2. 解析参数
            let args: Args = match serde_json::from_value(args) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Missing or invalid parameters: {e}")),
                    });
                }
            };

            // 3. 查找技能
            let skills = skills::load_skills_full_with_config(
                &self.workspace_dir,
                self.root_config.as_ref(),
            );
            let skill_info =
                match skills.iter().find(|skill| skill_matches_name(skill, &args.name)).cloned() {
                    Some(info) => info,
                    None => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some(format!(
                                "未找到技能 \"{}\"。可用技能：{}",
                                args.name,
                                available_skill_names(&skills)
                            )),
                        });
                    }
                };

            // 4. 请求权限确认
            if let Err(error) = self.ask_skill_permission(&skill_info.name).await {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error),
                });
            }

            // 5. 获取技能目录和基础 URL
            let loc = skill_location(&skill_info, &self.workspace_dir);
            let dir = match loc.parent().map(|p| p.to_path_buf()) {
                Some(v) => v,
                None => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("技能路径无效".to_string()),
                    });
                }
            };
            let base = base_dir_url(dir.as_path());

            // 6. 获取技能目录下的文件列表（抽样）
            // 过滤掉 SKILL.md 文件，最多显示 10 个文件
            let files = ripgrep::files(ripgrep::FilesInput {
                cwd: dir.clone(),
                glob: None,
                hidden: Some(true),
                follow: Some(false),
                max_depth: None,
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|f| !f.contains("SKILL.md"))
            .take(10)
            .map(|rel| format!("<file>{}</file>", dir.join(rel).to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n");

            // 7. 构造并返回技能内容
            Ok(ToolResult {
                success: true,
                output: [
                    format!("<skill_content name=\"{}\">", xml_escape(&skill_info.name)),
                    format!("# 技能：{}", skill_info.name),
                    String::new(),
                    skill_prompt_content(&skill_info),
                    String::new(),
                    format!("该技能的基础目录：{}", base),
                    "该技能中的相对路径（例如 scripts/、reference/）都相对于该基础目录。"
                        .to_string(),
                    "注意：文件列表为抽样结果。".to_string(),
                    String::new(),
                    "<skill_files>".to_string(),
                    files,
                    "</skill_files>".to_string(),
                    "</skill_content>".to_string(),
                ]
                .join("\n"),
                error: None,
            })
        }
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
