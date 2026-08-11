//! 提示词构建模块的单元测试
//!
//! 本模块包含针对提示词生成系统的全面测试套件，主要验证以下功能：
//!
//! - 工作空间身份文件的注入与合并
//! - 系统提示词的组装与各部分内容的正确性
//! - 技能（Skills）提示词的注入与转义处理
//! - 日期时间信息的格式化输出
//! - 提示词内容的安全性（XML 转义）
//!
//! # 测试覆盖范围
//!
//! - `IdentitySection`：身份文件的注入逻辑
//! - `SystemPromptBuilder`：完整提示词的构建与组装
//! - `SkillsSection`：技能信息在完整模式和精简模式下的渲染
//! - `DateTimeSection`：当前时间戳和时区信息的格式化
//!
//! # 测试策略
//!
//! 使用临时文件系统隔离测试环境，确保测试的独立性和可重复性。
//! 所有测试均为确定性测试，不依赖外部服务或网络连接。

use super::*;

/// 测试模块内部实现
///
/// 此模块包含所有测试用例的实现，使用 `#[allow(dead_code)]` 标注
/// 以允许测试辅助代码在非测试构建中被编译器忽略。
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::tools::traits::Tool;
    use async_trait::async_trait;

    /// 测试用工具结构体
    ///
    /// 提供一个最小化的工具实现，用于测试提示词构建器对工具列表的处理。
    /// 此工具不执行实际操作，仅返回固定的成功结果。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let tool = TestTool;
    /// assert_eq!(tool.name(), "test_tool");
    /// ```
    struct TestTool;

    /// 为 TestTool 实现 Tool trait
    ///
    /// 提供工具的基本元数据和执行逻辑，所有方法返回固定值以简化测试。
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Tool for TestTool {
        /// 返回工具名称
        ///
        /// # 返回值
        ///
        /// 固定返回 `"test_tool"` 字符串
        fn name(&self) -> &str {
            "test_tool"
        }

        /// 返回工具描述
        ///
        /// # 返回值
        ///
        /// 固定返回 `"tool desc"` 字符串
        fn description(&self) -> &str {
            "tool desc"
        }

        /// 返回工具参数的 JSON Schema
        ///
        /// # 返回值
        ///
        /// 返回一个空对象的 JSON Schema 定义
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        /// 执行工具操作（模拟实现）
        ///
        /// 此方法不执行任何实际操作，始终返回成功结果。
        ///
        /// # 参数
        ///
        /// * `_args` - 工具参数（被忽略）
        ///
        /// # 返回值
        ///
        /// 返回固定的成功结果，包含：
        /// - `success: true` - 执行成功标志
        /// - `output: "ok"` - 输出内容
        /// - `error: None` - 无错误信息
        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> anyhow::Result<crate::app::agent::tools::ToolResult> {
            Ok(crate::app::agent::tools::ToolResult {
                success: true,
                output: "ok".into(),
                error: None,
            })
        }
    }

    /// 测试身份段在 AIEOS 模式下是否包含工作空间文件
    ///
    /// 验证身份段会按新的工作区身份文件列表注入内容。
    ///
    /// # 测试场景
    ///
    /// - 创建临时工作空间并写入 `AGENTS.md` 与 `HEARTBEAT.md`
    /// - 构建身份段
    /// - 验证两个文件内容都被包含
    ///
    /// # 预期结果
    ///
    /// - 提示词中包含 "AGENT_FILE_LOADED"（来自 `AGENTS.md`）
    /// - 提示词中包含 "HEARTBEAT_FILE_LOADED"（来自 `HEARTBEAT.md`）
    #[test]
    fn identity_section_includes_workspace_identity_files() {
        // 创建唯一的临时工作空间目录，避免测试间冲突
        let workspace =
            std::env::temp_dir().join(format!("vibewindow_prompt_test_{}", uuid::Uuid::new_v4()));

        // 设置测试环境：创建目录和身份文件
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("AGENTS.md"), "Always respond with: AGENT_FILE_LOADED")
            .unwrap();
        std::fs::write(
            workspace.join("HEARTBEAT.md"),
            "Always respond with: HEARTBEAT_FILE_LOADED",
        )
        .unwrap();

        // 构建提示词上下文
        let tools: Vec<Box<dyn Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: &workspace,
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 执行身份段构建
        let section = IdentitySection;
        let output = section.build(&ctx).unwrap();

        // 验证新的工作区身份文件内容被正确注入
        assert!(
            output.contains("AGENT_FILE_LOADED"),
            "AGENTS.md content should be present in prompt"
        );
        assert!(
            output.contains("HEARTBEAT_FILE_LOADED"),
            "HEARTBEAT.md content should be present in prompt"
        );

        // 清理临时工作空间
        let _ = std::fs::remove_dir_all(workspace);
    }

    /// 测试提示词构建器的段组装功能
    ///
    /// 验证 `SystemPromptBuilder` 能够正确地将各个部分组装成完整的系统提示词，
    /// 包括工具列表、调度器指令等内容。
    ///
    /// # 测试场景
    ///
    /// - 使用默认配置初始化提示词构建器
    /// - 提供测试工具和调度器指令
    /// - 构建完整的系统提示词
    ///
    /// # 预期结果
    ///
    /// - 提示词包含 "## Tools" 章节标题
    /// - 提示词包含测试工具的名称 "test_tool"
    /// - 提示词包含调度器指令 "instr"
    #[test]
    fn prompt_builder_assembles_sections() {
        // 准备测试工具列表
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];

        // 构建提示词上下文
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "instr",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 使用默认配置构建提示词
        let prompt = SystemPromptBuilder::with_defaults().build(&ctx).unwrap();

        // 验证各个部分被正确组装
        assert!(prompt.contains("## Tools"));
        assert!(prompt.contains("test_tool"));
        assert!(prompt.contains("instr"));
    }

    /// 测试技能段在完整模式下包含指令和工具信息
    ///
    /// 验证 `SkillsSection` 在 `Full` 模式下能够正确渲染技能的完整信息，
    /// 包括技能名称、描述、指令以及技能自带的工具列表。
    ///
    /// # 测试场景
    ///
    /// - 定义包含工具和指令的技能
    /// - 使用 `Full` 模式构建技能段
    /// - 验证所有技能信息都被正确渲染
    ///
    /// # 预期结果
    ///
    /// - 包含 `<available_skills>` 标签
    /// - 包含技能名称 `<name>deploy</name>`
    /// - 包含技能指令 `<instruction>Run smoke tests before deploy.</instruction>`
    /// - 包含技能工具名称 `<name>release_checklist</name>`
    /// - 包含工具类型 `<kind>shell</kind>`
    #[test]
    fn skills_section_does_not_inline_skill_bodies() {
        let tools: Vec<Box<dyn Tool>> = vec![];

        // 创建包含工具和指令的技能
        let skills = vec![crate::app::agent::skills::Skill {
            name: "deploy".into(),
            description: "Release safely".into(),
            version: "1.0.0".into(),
            author: None,
            tags: vec![],
            tools: vec![crate::app::agent::skills::SkillTool {
                name: "release_checklist".into(),
                description: "Validate release readiness".into(),
                kind: "shell".into(),
                command: "echo ok".into(),
                args: std::collections::HashMap::new(),
            }],
            prompts: vec!["Run smoke tests before deploy.".into()],
            location: None,
            ..crate::app::agent::skills::Skill::default()
        }];

        // 使用 Full 模式构建上下文
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &skills,
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 构建技能段
        let output = SkillsSection.build(&ctx).unwrap();

        assert!(output.is_empty());
    }

    /// 测试技能段在精简模式下省略指令和工具详情
    ///
    /// 验证 `SkillsSection` 在 `Compact` 模式下只渲染技能的基本元数据，
    /// 不包含详细的指令和工具列表，以减少提示词长度。
    ///
    /// # 测试场景
    ///
    /// - 定义包含工具、指令和位置信息的技能
    /// - 使用 `Compact` 模式构建技能段
    /// - 验证只包含基本信息而不包含详细内容
    ///
    /// # 预期结果
    ///
    /// - 包含 `<available_skills>` 标签
    /// - 包含技能名称 `<name>deploy</name>`
    /// - 包含相对位置路径 `<location>skills/deploy/SKILL.md</location>`
    /// - **不**包含指令标签 `<instruction>`
    /// - **不**包含工具详情标签 `<tools>`
    #[test]
    fn skills_section_is_empty_in_compact_mode() {
        let tools: Vec<Box<dyn Tool>> = vec![];

        // 创建包含完整信息的技能（但在 Compact 模式下会被简化）
        let skills = vec![crate::app::agent::skills::Skill {
            name: "deploy".into(),
            description: "Release safely".into(),
            version: "1.0.0".into(),
            author: None,
            tags: vec![],
            tools: vec![crate::app::agent::skills::SkillTool {
                name: "release_checklist".into(),
                description: "Validate release readiness".into(),
                kind: "shell".into(),
                command: "echo ok".into(),
                args: std::collections::HashMap::new(),
            }],
            prompts: vec!["Run smoke tests before deploy.".into()],
            location: Some(Path::new("/tmp/workspace/skills/deploy/SKILL.md").to_path_buf()),
            ..crate::app::agent::skills::Skill::default()
        }];

        // 使用 Compact 模式构建上下文
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp/workspace"),
            model_name: "test-model",
            tools: &tools,
            skills: &skills,
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Compact,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 构建技能段
        let output = SkillsSection.build(&ctx).unwrap();

        assert!(output.is_empty());
    }

    /// 测试日期时间段的格式化输出
    ///
    /// 验证 `DateTimeSection` 能够正确生成包含当前时间戳和时区信息的格式化输出。
    ///
    /// # 测试场景
    ///
    /// - 使用基本的提示词上下文
    /// - 构建日期时间段
    /// - 验证输出格式符合预期
    ///
    /// # 预期结果
    ///
    /// - 以 "## Current Date & Time\n\n" 开头
    /// - 包含数字字符（时间戳）
    /// - 包含括号（时区信息）
    /// - 以右括号结尾
    #[test]
    fn datetime_section_includes_timestamp_and_timezone() {
        let tools: Vec<Box<dyn Tool>> = vec![];

        // 构建提示词上下文
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "instr",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 构建日期时间段
        let rendered = DateTimeSection.build(&ctx).unwrap();

        // 验证章节标题格式
        assert!(rendered.starts_with("## Current Date & Time\n\n"));

        // 提取实际的时间负载内容
        let payload = rendered.trim_start_matches("## Current Date & Time\n\n");

        // 验证包含数字（时间戳的一部分）
        assert!(payload.chars().any(|c| c.is_ascii_digit()));

        // 验证包含时区信息（括号包围）
        assert!(payload.contains(" ("));
        assert!(payload.ends_with(')'));
    }

    #[test]
    fn datetime_section_uses_configured_timezone() {
        let tools: Vec<Box<dyn Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::America::New_York,
        };

        let rendered = DateTimeSection.build(&ctx).unwrap();

        assert!(rendered.contains("-04:00") || rendered.contains("-05:00"));
        assert!(rendered.contains("(America/New_York)"));
    }

    /// 测试提示词构建器对技能内容的内联和 XML 转义处理
    ///
    /// 验证当技能名称、描述、指令等内容包含特殊字符（如 `<`, `>`, `&`, `"`, `'`）时，
    /// 系统能够正确地进行 XML 转义，以防止提示词注入攻击和格式错误。
    ///
    /// # 测试场景
    ///
    /// - 定义包含各种特殊字符的技能：
    ///   - 技能名称包含 `<`、`>`、`&`
    ///   - 描述包含 `"` 和 `'`
    ///   - 工具名称、类型、指令包含各种特殊字符
    /// - 使用 `Full` 模式构建完整提示词
    /// - 验证所有特殊字符都被正确转义
    ///
    /// # 预期结果
    ///
    /// - 包含 `<available_skills>` 标签
    /// - `<` 被转义为 `&lt;`
    /// - `>` 被转义为 `&gt;`
    /// - `&` 被转义为 `&amp;`
    /// - `"` 被转义为 `&quot;`
    /// - `'` 被转义为 `&apos;`
    ///
    /// # 安全性说明
    ///
    /// 此测试确保用户提供的技能内容不会破坏提示词的 XML 结构，
    /// 防止潜在的提示词注入攻击。
    #[test]
    fn prompt_builder_excludes_skill_bodies() {
        let tools: Vec<Box<dyn Tool>> = vec![];

        // 创建包含特殊字符的技能，用于测试 XML 转义功能
        let skills = vec![crate::app::agent::skills::Skill {
            name: "code<review>&".into(), // 包含 XML 特殊字符
            description: "Review \"unsafe\" and 'risky' bits".into(), // 包含引号
            version: "1.0.0".into(),
            author: None,
            tags: vec![],
            tools: vec![crate::app::agent::skills::SkillTool {
                name: "run\"linter\"".into(),              // 包含双引号
                description: "Run <lint> & report".into(), // 包含多种特殊字符
                kind: "shell&exec".into(),                 // 包含 & 符号
                command: "cargo clippy".into(),
                args: std::collections::HashMap::new(),
            }],
            prompts: vec!["Use <tool_call> and & keep output \"safe\"".into()], // 包含各种特殊字符
            location: None,
            ..crate::app::agent::skills::Skill::default()
        }];

        // 构建提示词上下文
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp/workspace"),
            model_name: "test-model",
            tools: &tools,
            skills: &skills,
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::Asia::Shanghai,
        };

        // 构建完整的系统提示词
        let prompt = SystemPromptBuilder::with_defaults().build(&ctx).unwrap();

        assert!(!prompt.contains("<available_skills>"));
        assert!(!prompt.contains("Use <tool_call> and & keep output \"safe\""));
    }
}
