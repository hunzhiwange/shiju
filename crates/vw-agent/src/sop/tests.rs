//! SOP（标准操作流程）模块测试
//!
//! 本模块包含针对 SOP 解析、加载、验证和触发器处理等核心功能的单元测试。
//! 测试覆盖以下主要功能：
//! - Markdown 格式的步骤解析
//! - 从文件系统加载 SOP 配置
//! - SOP 配置验证和警告检测
//! - 目录路径解析
//! - 标题提取工具
//! - 多种触发器类型的 TOML 解析

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use std::fs;

    /// 测试基本的步骤解析功能
    ///
    /// 验证 `parse_steps` 函数能够正确解析包含完整元数据的 Markdown 格式步骤：
    /// - 步骤编号和标题的提取
    /// - 步骤内容的解析
    /// - 建议工具列表的提取
    /// - 确认要求标志的识别
    #[test]
    fn parse_steps_basic() {
        let md = r#"# Test SOP

    ## Conditions
    Some conditions here.

    ## Steps

    1. **Check readings** — Read sensor data and confirm.
       - tools: gpio_read, memory_store

    2. **Close valve** — Set GPIO pin 5 LOW.
       - tools: gpio_write, gpio_read
       - requires_confirmation: true

    3. **Notify operator** — Send alert.
       - tools: pushover
    "#;

        let steps = parse_steps(md);
        assert_eq!(steps.len(), 3);

        assert_eq!(steps[0].number, 1);
        assert_eq!(steps[0].title, "Check readings");
        assert!(steps[0].body.contains("Read sensor data"));
        assert_eq!(steps[0].suggested_tools, vec!["gpio_read", "memory_store"]);
        assert!(!steps[0].requires_confirmation);

        assert_eq!(steps[1].number, 2);
        assert_eq!(steps[1].title, "Close valve");
        assert!(steps[1].requires_confirmation);
        assert_eq!(steps[1].suggested_tools, vec!["gpio_write", "gpio_read"]);

        assert_eq!(steps[2].number, 3);
        assert_eq!(steps[2].title, "Notify operator");
    }

    /// 测试解析不包含步骤段的 Markdown 文档
    ///
    /// 验证当 Markdown 文档中没有 ## Steps 部分时，
    /// `parse_steps` 函数返回空列表而不是报错。
    #[test]
    fn parse_steps_empty_md() {
        let steps = parse_steps("# Nothing here\n\nNo steps section.");
        assert!(steps.is_empty());
    }

    /// 测试解析没有加粗标题的步骤
    ///
    /// 验证当步骤没有使用 **加粗** 格式时，
    /// `parse_steps` 函数能够正确处理，并将整行作为标题。
    #[test]
    fn parse_steps_no_bold_title() {
        let md = "## Steps\n\n1. Just a plain step without bold.\n";
        let steps = parse_steps(md);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].title, "Just a plain step without bold.");
    }

    /// 测试解析包含多行内容的步骤
    ///
    /// 验证 `parse_steps` 函数能够正确解析跨越多行的步骤内容，
    /// 确保所有行都被包含在步骤的 body 字段中。
    #[test]
    fn parse_steps_multiline_body() {
        let md = r#"## Steps

    1. **Do thing** — First line of body.
       Second line of body.
       Third line of body.
       - tools: shell
    "#;
        let steps = parse_steps(md);
        assert_eq!(steps.len(), 1);
        assert!(steps[0].body.contains("First line"));
        assert!(steps[0].body.contains("Second line"));
        assert!(steps[0].body.contains("Third line"));
    }

    /// 测试从目录加载完整的 SOP 配置
    ///
    /// 验证 `load_sops_from_directory` 函数能够：
    /// - 正确读取 SOP.toml 配置文件
    /// - 正确读取 SOP.md 步骤文件
    /// - 解析所有配置字段（名称、描述、优先级、执行模式等）
    /// - 解析触发器列表
    /// - 解析步骤列表及其属性
    /// - 记录 SOP 文件位置
    ///
    /// 测试场景：
    /// - 创建临时目录结构
    /// - 写入包含元数据和触发器的 TOML 配置
    /// - 写入包含多个步骤的 Markdown 文件
    /// - 验证加载后的 SOP 对象完整性
    #[test]
    fn load_sop_from_directory() {
        // 创建临时目录用于测试
        let dir = tempfile::tempdir().unwrap();
        let sop_dir = dir.path().join("test-sop");
        fs::create_dir_all(&sop_dir).unwrap();

        // 写入 TOML 配置文件，定义 SOP 元数据和触发器
        fs::write(
            sop_dir.join("SOP.toml"),
            r#"
    [sop]
    name = "test-sop"
    description = "A test SOP"
    version = "1.0.0"
    priority = "high"
    execution_mode = "auto"
    cooldown_secs = 60

    [[triggers]]
    type = "manual"

    [[triggers]]
    type = "webhook"
    path = "/sop/test"
    "#,
        )
        .unwrap();

        // 写入 Markdown 文件，定义 SOP 执行步骤
        fs::write(
            sop_dir.join("SOP.md"),
            r#"# Test SOP

    ## Steps

    1. **Step one** — Do something.
       - tools: shell

    2. **Step two** — Do something else.
       - requires_confirmation: true
    "#,
        )
        .unwrap();

        // 加载 SOP 并验证解析结果
        let sops = load_sops_from_directory(dir.path(), SopExecutionMode::Supervised);
        assert_eq!(sops.len(), 1);

        // 验证 SOP 元数据字段
        let sop = &sops[0];
        assert_eq!(sop.name, "test-sop");
        assert_eq!(sop.priority, SopPriority::High);
        assert_eq!(sop.execution_mode, SopExecutionMode::Auto);
        assert_eq!(sop.cooldown_secs, 60);

        // 验证触发器和步骤
        assert_eq!(sop.triggers.len(), 2);
        assert_eq!(sop.steps.len(), 2);
        assert!(sop.steps[1].requires_confirmation);

        // 验证文件位置已记录
        assert!(sop.location.is_some());
    }

    /// 测试从空目录加载 SOP
    ///
    /// 验证当目录中没有任何 SOP 配置文件时，
    /// `load_sops_from_directory` 函数返回空列表而不报错。
    #[test]
    fn load_sops_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sops = load_sops_from_directory(dir.path(), SopExecutionMode::Supervised);
        assert!(sops.is_empty());
    }

    /// 测试从不存在的目录加载 SOP
    ///
    /// 验证当目录路径不存在时，
    /// `load_sops_from_directory` 函数安全地返回空列表而不 panic。
    #[test]
    fn load_sops_nonexistent_dir() {
        let sops =
            load_sops_from_directory(Path::new("/nonexistent/path"), SopExecutionMode::Supervised);
        assert!(sops.is_empty());
    }

    /// 测试仅包含 TOML 配置文件而没有 Markdown 步骤文件的 SOP
    ///
    /// 验证当只有 SOP.toml 文件而没有 SOP.md 文件时，
    /// SOP 仍然能够成功加载，但步骤列表为空。
    /// 这允许定义没有具体执行步骤的声明性 SOP。
    #[test]
    fn load_sop_toml_only_no_md() {
        let dir = tempfile::tempdir().unwrap();
        let sop_dir = dir.path().join("no-steps");
        fs::create_dir_all(&sop_dir).unwrap();

        fs::write(
            sop_dir.join("SOP.toml"),
            r#"
    [sop]
    name = "no-steps"
    description = "SOP without steps"

    [[triggers]]
    type = "manual"
    "#,
        )
        .unwrap();

        let sops = load_sops_from_directory(dir.path(), SopExecutionMode::Supervised);
        assert_eq!(sops.len(), 1);
        assert!(sops[0].steps.is_empty());
    }

    /// 测试当 TOML 配置中省略执行模式时的默认行为
    ///
    /// 验证当 SOP.toml 文件中没有显式指定 execution_mode 字段时，
    /// `load_sops_from_directory` 函数使用配置参数中提供的默认执行模式。
    ///
    /// 这确保了全局配置可以作为未指定 SOP 的回退默认值。
    #[test]
    fn load_sop_uses_config_default_execution_mode_when_omitted() {
        let dir = tempfile::tempdir().unwrap();
        let sop_dir = dir.path().join("default-mode");
        fs::create_dir_all(&sop_dir).unwrap();

        fs::write(
            sop_dir.join("SOP.toml"),
            r#"
    [sop]
    name = "default-mode"
    description = "SOP without explicit execution mode"

    [[triggers]]
    type = "manual"
    "#,
        )
        .unwrap();

        let sops = load_sops_from_directory(dir.path(), SopExecutionMode::Auto);
        assert_eq!(sops.len(), 1);
        assert_eq!(sops[0].execution_mode, SopExecutionMode::Auto);
    }

    /// 测试 SOP 验证函数能够正确检测并报告警告
    ///
    /// 验证 `validate_sop` 函数能够识别以下问题：
    /// - 空名称字段
    /// - 空描述字段
    /// - 缺少触发器
    /// - 缺少执行步骤
    ///
    /// 测试创建一个故意不完整的 SOP 对象，
    /// 确保所有预期的警告都被生成。
    #[test]
    fn validate_sop_warnings() {
        // 创建一个故意不完整的 SOP 对象，用于触发所有验证警告
        let sop = Sop {
            name: String::new(),        // 空名称
            description: String::new(), // 空描述
            version: "1.0.0".into(),
            priority: SopPriority::Normal,
            execution_mode: SopExecutionMode::Supervised,
            triggers: Vec::new(), // 空触发器列表
            steps: Vec::new(),    // 空步骤列表
            cooldown_secs: 0,
            max_concurrent: 1,
            location: None,
        };

        // 验证所有预期的警告都已生成
        let warnings = validate_sop(&sop);
        assert!(warnings.iter().any(|w| w.contains("name is empty")));
        assert!(warnings.iter().any(|w| w.contains("description is empty")));
        assert!(warnings.iter().any(|w| w.contains("no triggers")));
        assert!(warnings.iter().any(|w| w.contains("no steps")));
    }

    /// 测试 SOP 验证函数对有效配置的处理
    ///
    /// 验证当 SOP 对象包含所有必需字段和有效数据时，
    /// `validate_sop` 函数不生成任何警告。
    ///
    /// 测试创建一个完整的 SOP 对象，包含：
    /// - 有效名称和描述
    /// - 至少一个触发器
    /// - 至少一个执行步骤
    #[test]
    fn validate_sop_clean() {
        // 创建一个完整的有效 SOP 对象
        let sop = Sop {
            name: "valid-sop".into(),
            description: "A valid SOP".into(),
            version: "1.0.0".into(),
            priority: SopPriority::High,
            execution_mode: SopExecutionMode::Auto,
            triggers: vec![SopTrigger::Manual], // 至少一个触发器
            steps: vec![SopStep {
                // 至少一个步骤
                number: 1,
                title: "Do thing".into(),
                body: "Do the thing".into(),
                suggested_tools: vec!["shell".into()],
                requires_confirmation: false,
            }],
            cooldown_secs: 0,
            max_concurrent: 1,
            location: None,
        };

        // 验证没有生成任何警告
        let warnings = validate_sop(&sop);
        assert!(warnings.is_empty());
    }

    /// 测试默认 SOP 目录路径解析
    ///
    /// 验证当未提供自定义路径时，
    /// `resolve_sops_dir` 函数返回工作区路径下的 "sops" 子目录。
    ///
    /// 预期行为：`/workspace/path` -> `/workspace/path/sops`
    #[test]
    fn resolve_sops_dir_default() {
        let ws = Path::new("/home/user/.vibewindow/workspace");
        let dir = resolve_sops_dir(ws, None);
        assert_eq!(dir, ws.join("sops"));
    }

    /// 测试自定义 SOP 目录路径覆盖
    ///
    /// 验证当提供自定义路径时，
    /// `resolve_sops_dir` 函数返回提供的自定义路径，
    /// 而不是默认的工作区子目录。
    ///
    /// 预期行为：自定义路径参数优先于默认路径
    #[test]
    fn resolve_sops_dir_override() {
        let ws = Path::new("/home/user/.vibewindow/workspace");
        let dir = resolve_sops_dir(ws, Some("/custom/sops"));
        assert_eq!(dir, PathBuf::from("/custom/sops"));
    }

    /// 测试从带破折号分隔符的文本中提取加粗标题
    ///
    /// 验证 `extract_bold_title` 函数能够正确解析格式：
    /// `**标题** — 内容` 或 `**标题**- 内容`
    ///
    /// 预期行为：返回元组 (标题, 内容)，正确分离加粗部分和其余内容
    #[test]
    fn extract_bold_title_with_dash() {
        let (title, body) = extract_bold_title("**Close valve** — Set GPIO pin LOW.").unwrap();
        assert_eq!(title, "Close valve");
        assert_eq!(body, "Set GPIO pin LOW.");
    }

    /// 测试从没有分隔符的文本中提取加粗标题
    ///
    /// 验证当文本格式为 `**标题** 内容`（没有破折号或其他分隔符）时，
    /// `extract_bold_title` 函数仍然能够正确提取标题和内容。
    #[test]
    fn extract_bold_title_no_separator() {
        let (title, body) = extract_bold_title("**Close valve** Set pin LOW.").unwrap();
        assert_eq!(title, "Close valve");
        assert_eq!(body, "Set pin LOW.");
    }

    /// 测试从没有加粗格式的文本中提取标题（应返回 None）
    ///
    /// 验证当文本不包含 **加粗** 格式时，
    /// `extract_bold_title` 函数返回 None，表示无法提取标题。
    #[test]
    fn extract_bold_title_none() {
        assert!(extract_bold_title("No bold here").is_none());
    }

    /// 测试解析 TOML 中所有类型的触发器配置
    ///
    /// 验证 `SopManifest` 能够正确反序列化所有支持的触发器类型：
    /// - MQTT 触发器：包含 topic 和 condition 字段
    /// - Webhook 触发器：包含 path 字段
    /// - Cron 触发器：包含 expression 字段（cron 表达式）
    /// - Manual 触发器：手动触发，无需额外字段
    ///
    /// 测试验证每个触发器都被解析为正确的枚举变体。
    #[test]
    fn parse_all_trigger_types() {
        // 定义包含所有触发器类型的 TOML 配置
        let toml_str = r#"
    [sop]
    name = "multi-trigger"
    description = "SOP with all trigger types"

    [[triggers]]
    type = "mqtt"
    topic = "sensors/temp"
    condition = "$.value > 90"

    [[triggers]]
    type = "webhook"
    path = "/sop/test"

    [[triggers]]
    type = "cron"
    expression = "0 */5 * * *"

    [[triggers]]
    type = "manual"
    "#;

        // 解析 TOML 并验证触发器数量和类型
        let manifest: SopManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.triggers.len(), 4);

        // 使用模式匹配验证每个触发器被解析为正确的类型
        assert!(matches!(manifest.triggers[0], SopTrigger::Mqtt { .. }));
        assert!(matches!(manifest.triggers[1], SopTrigger::Webhook { .. }));
        assert!(matches!(manifest.triggers[2], SopTrigger::Cron { .. }));
        assert!(matches!(manifest.triggers[3], SopTrigger::Manual));
    }

    #[test]
    fn resolve_sops_dir_empty_override_uses_workspace_default() {
        let workspace = Path::new("/workspace");

        assert_eq!(resolve_sops_dir(workspace, Some("")), workspace.join("sops"));
    }

    #[test]
    fn parse_numbered_item_accepts_digits_and_rejects_invalid_prefixes() {
        assert_eq!(parse_numbered_item("12. Run checks"), Some("Run checks"));
        assert_eq!(parse_numbered_item("1. "), Some(""));
        assert_eq!(parse_numbered_item(". Missing number"), None);
        assert_eq!(parse_numbered_item("one. Not numeric"), None);
        assert_eq!(parse_numbered_item("1.No separating space"), None);
    }

    #[test]
    fn parse_steps_stops_at_next_second_level_heading() {
        let steps = parse_steps(
            "## Steps\n1. **First** — Included body.\n## Notes\n2. **Ignored** — Outside steps.\n",
        );

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].title, "First");
        assert_eq!(steps[0].body, "Included body.");
    }

    #[test]
    fn parse_steps_normalizes_numbers_and_handles_sub_bullet_edges() {
        let steps = parse_steps(
            "## STEPS\n7. **First** – Initial body.\n- tools: shell, , memory_store\n- requires_confirmation: TRUE\n- note: preserve this bullet\n42. Plain second step\n- tools:\n- requires_confirmation: false\n",
        );

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].number, 1);
        assert_eq!(steps[0].title, "First");
        assert_eq!(steps[0].suggested_tools, vec!["shell", "memory_store"]);
        assert!(steps[0].requires_confirmation);
        assert_eq!(steps[0].body, "Initial body.\n- note: preserve this bullet");

        assert_eq!(steps[1].number, 2);
        assert_eq!(steps[1].title, "Plain second step");
        assert!(steps[1].suggested_tools.is_empty());
        assert!(!steps[1].requires_confirmation);
    }

    #[test]
    fn extract_bold_title_rejects_unclosed_marker() {
        assert!(extract_bold_title("**Unclosed title").is_none());
    }

    #[test]
    fn load_sops_read_dir_error_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not-a-directory");
        fs::write(&file_path, "content").unwrap();

        let sops = load_sops_from_directory(&file_path, SopExecutionMode::Supervised);

        assert!(sops.is_empty());
    }

    #[test]
    fn load_sops_skips_invalid_entries_and_sorts_by_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("SOP.toml"), "ignored root file").unwrap();
        fs::create_dir(dir.path().join("missing-manifest")).unwrap();

        let invalid_dir = dir.path().join("invalid");
        fs::create_dir(&invalid_dir).unwrap();
        fs::write(invalid_dir.join("SOP.toml"), "not valid TOML = [").unwrap();

        for (directory, name) in [("first-on-disk", "zeta"), ("second-on-disk", "alpha")] {
            let sop_dir = dir.path().join(directory);
            fs::create_dir(&sop_dir).unwrap();
            fs::write(
                sop_dir.join("SOP.toml"),
                format!(
                    "[sop]\nname = \"{name}\"\ndescription = \"{name} description\"\n\n[[triggers]]\ntype = \"manual\"\n"
                ),
            )
            .unwrap();
        }

        let sops = load_sops_from_directory(dir.path(), SopExecutionMode::Supervised);
        let names: Vec<&str> = sops.iter().map(|sop| sop.name.as_str()).collect();

        assert_eq!(names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn load_sops_uses_explicit_directory() {
        let workspace = tempfile::tempdir().unwrap();
        let configured_dir = tempfile::tempdir().unwrap();
        let sop_dir = configured_dir.path().join("configured");
        fs::create_dir(&sop_dir).unwrap();
        fs::write(
            sop_dir.join("SOP.toml"),
            "[sop]\nname = \"configured\"\ndescription = \"Configured SOP\"\n\n[[triggers]]\ntype = \"manual\"\n",
        )
        .unwrap();

        let sops =
            load_sops(workspace.path(), configured_dir.path().to_str(), SopExecutionMode::Auto);

        assert_eq!(sops.len(), 1);
        assert_eq!(sops[0].name, "configured");
        assert_eq!(sops[0].execution_mode, SopExecutionMode::Auto);
    }

    #[test]
    fn load_sop_invalid_manifest_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("SOP.toml"), "invalid = [").unwrap();

        let result = load_sop(dir.path(), SopExecutionMode::Supervised);

        assert!(result.is_err());
    }

    #[test]
    fn validate_sop_reports_numbering_gaps_and_empty_titles() {
        let sop = Sop {
            name: "numbering".into(),
            description: "Validate step numbering".into(),
            version: "1.0.0".into(),
            priority: SopPriority::Normal,
            execution_mode: SopExecutionMode::Supervised,
            triggers: vec![SopTrigger::Manual],
            steps: vec![
                SopStep {
                    number: 2,
                    title: String::new(),
                    body: String::new(),
                    suggested_tools: Vec::new(),
                    requires_confirmation: false,
                },
                SopStep {
                    number: 4,
                    title: "Later step".into(),
                    body: String::new(),
                    suggested_tools: Vec::new(),
                    requires_confirmation: false,
                },
            ],
            cooldown_secs: 0,
            max_concurrent: 1,
            location: None,
        };

        let warnings = validate_sop(&sop);

        assert_eq!(
            warnings,
            vec![
                "Step numbering gap: expected 1, got 2".to_string(),
                "Step 2 has an empty title".to_string(),
                "Step numbering gap: expected 2, got 4".to_string(),
            ]
        );
    }

    #[test]
    fn sop_commands_round_trip_through_json() {
        let commands = [
            SopCommands::List,
            SopCommands::Validate { name: None },
            SopCommands::Validate { name: Some("inspect".into()) },
            SopCommands::Show { name: "inspect".into() },
        ];

        for command in commands {
            let json = serde_json::to_string(&command).unwrap();
            let decoded: SopCommands = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, command);
        }
    }

    #[test]
    fn handle_validate_missing_named_sop_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::app::agent::config::Config::default();
        config.workspace_dir = dir.path().to_path_buf();
        config.sop.sops_dir = Some(dir.path().join("missing").to_string_lossy().into_owned());

        let error = handle_command(SopCommands::Validate { name: Some("absent".into()) }, &config)
            .unwrap_err();

        assert_eq!(error.to_string(), "SOP not found: absent");
    }

    #[test]
    fn handle_show_missing_named_sop_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::app::agent::config::Config::default();
        config.workspace_dir = dir.path().to_path_buf();
        config.sop.sops_dir = Some(dir.path().join("missing").to_string_lossy().into_owned());

        let error =
            handle_command(SopCommands::Show { name: "absent".into() }, &config).unwrap_err();

        assert_eq!(error.to_string(), "SOP not found: absent");
    }

    #[test]
    fn handle_list_and_validate_empty_directory_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::app::agent::config::Config::default();
        config.workspace_dir = dir.path().to_path_buf();
        config.sop.sops_dir = Some(dir.path().join("missing").to_string_lossy().into_owned());

        assert!(handle_command(SopCommands::List, &config).is_ok());
        assert!(handle_command(SopCommands::Validate { name: None }, &config).is_ok());
    }

    #[test]
    fn handle_commands_accept_valid_named_sop() {
        let dir = tempfile::tempdir().unwrap();
        let sop_dir = dir.path().join("inspect");
        fs::create_dir(&sop_dir).unwrap();
        fs::write(
            sop_dir.join("SOP.toml"),
            r#"[sop]
name = "inspect"
description = "Inspect a valid SOP"
version = "1.2.3"
priority = "high"
execution_mode = "auto"
cooldown_secs = 30
max_concurrent = 2

[[triggers]]
type = "manual"
"#,
        )
        .unwrap();
        fs::write(
            sop_dir.join("SOP.md"),
            "## Steps\n1. **Inspect** — Check the system.\n- tools: shell\n- requires_confirmation: true\n",
        )
        .unwrap();

        let mut config = crate::app::agent::config::Config::default();
        config.workspace_dir = dir.path().to_path_buf();
        config.sop.sops_dir = Some(dir.path().to_string_lossy().into_owned());

        assert!(handle_command(SopCommands::List, &config).is_ok());
        assert!(
            handle_command(SopCommands::Validate { name: Some("inspect".into()) }, &config).is_ok()
        );
        assert!(handle_command(SopCommands::Show { name: "inspect".into() }, &config).is_ok());
    }

    #[test]
    fn handle_validate_reports_loaded_sop_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let sop_dir = dir.path().join("incomplete");
        fs::create_dir(&sop_dir).unwrap();
        fs::write(
            sop_dir.join("SOP.toml"),
            "[sop]\nname = \"incomplete\"\ndescription = \"Missing steps\"\n\n[[triggers]]\ntype = \"manual\"\n",
        )
        .unwrap();

        let mut config = crate::app::agent::config::Config::default();
        config.workspace_dir = dir.path().to_path_buf();
        config.sop.sops_dir = Some(dir.path().to_string_lossy().into_owned());

        let error =
            handle_command(SopCommands::Validate { name: Some("incomplete".into()) }, &config)
                .unwrap_err();

        assert_eq!(error.to_string(), "Validation completed with warnings");
    }
}
