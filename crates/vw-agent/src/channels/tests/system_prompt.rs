//! 系统提示构建功能测试模块
//!
//! 本模块包含针对 `build_system_prompt` 函数的全面测试套件，验证系统提示的各个组成部分：
//! - 工具列表注入
//! - 安全策略注入
//! - 工作区文件注入（SOUL.md、IDENTITY.md、USER.md、AGENTS.md、TOOLS.md、MEMORY.md）
//! - 运行时元数据注入
//! - 文件截断逻辑
//! - 空文件跳过逻辑
//! - 频道能力说明注入
//!
//! 这些测试确保系统提示始终包含必要的信息，为 AI 代理提供正确的上下文和约束。

use super::*;

fn build_system_prompt(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[crate::app::agent::skills::Skill],
    identity_config: Option<&crate::app::agent::config::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
) -> String {
    crate::app::agent::channels::build_system_prompt(
        workspace_dir,
        model_name,
        tools,
        skills,
        identity_config,
        bootstrap_max_chars,
        chrono_tz::Asia::Shanghai,
    )
}

/// 测试系统提示是否包含所有必需的章节
///
/// 验证系统提示中包含以下核心章节：
/// - Tools（工具章节）
/// - Safety（安全章节）
/// - Workspace（工作区章节）
/// - Project Context（项目上下文章节）
/// - Current Date & Time（当前日期时间章节）
/// - Runtime（运行时章节）
#[test]
fn prompt_contains_all_sections() {
    let ws = make_workspace();
    let tools = vec![("shell", "Run commands"), ("file_read", "Read files")];
    let prompt = build_system_prompt(ws.path(), "test-model", &tools, &[], None, None);

    // 验证所有必需章节都存在
    assert!(prompt.contains("## Tools"), "missing Tools section");
    assert!(prompt.contains("## Safety"), "missing Safety section");
    assert!(prompt.contains("## Workspace"), "missing Workspace section");
    assert!(prompt.contains("## Project Context"), "missing Project Context");
    assert!(prompt.contains("## Current Date & Time"), "missing Date/Time");
    assert!(prompt.contains("## Runtime"), "missing Runtime section");
}

#[test]
fn channel_prompt_uses_configured_timezone() {
    let ws = make_workspace();
    let prompt = crate::app::agent::channels::build_system_prompt(
        ws.path(),
        "test-model",
        &[],
        &[],
        None,
        None,
        chrono_tz::America::New_York,
    );

    assert!(prompt.contains("-04:00") || prompt.contains("-05:00"));
    assert!(prompt.contains("(America/New_York)"));
}

/// 测试工具列表是否正确注入到系统提示中
///
/// 验证传入的工具列表会以格式化的方式出现在系统提示中，
/// 包括工具名称（加粗显示）和工具描述。
#[test]
fn prompt_injects_tools() {
    let ws = make_workspace();
    let tools = vec![("shell", "Run commands"), ("memory_recall", "Search memory")];
    let prompt = build_system_prompt(ws.path(), "gpt-4o", &tools, &[], None, None);

    // 验证工具名称和描述都存在于提示中
    assert!(prompt.contains("**shell**"));
    assert!(prompt.contains("Run commands"));
    assert!(prompt.contains("**memory_recall**"));
}

/// 测试安全策略是否正确注入到系统提示中
///
/// 验证系统提示包含关键的安全约束，包括：
/// - 不泄露私有数据
/// - 不运行破坏性命令
/// - 优先使用 `trash` 而非 `rm`（更安全的删除方式）
#[test]
fn prompt_injects_safety() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证安全策略的存在
    assert!(prompt.contains("Do not exfiltrate private data"));
    assert!(prompt.contains("Do not run destructive commands"));
    assert!(prompt.contains("Prefer `trash` over `rm`"));
}

/// 测试工作区文件是否正确注入到系统提示中
///
/// 验证以下文件内容会被注入到系统提示：
/// - SOUL.md（核心指导原则）
/// - IDENTITY.md（身份信息）
/// - USER.md（用户配置）
/// - AGENTS.md（代理配置）
/// - TOOLS.md（工具配置）
/// - MEMORY.md（长期记忆）
///
/// 同时验证 HEARTBEAT.md 不会被注入（因为它是心跳文件，不应包含在频道提示中）
#[test]
fn prompt_injects_workspace_files() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证各种配置文件的存在
    assert!(prompt.contains("### SOUL.md"), "missing SOUL.md header");
    assert!(prompt.contains("Be helpful"), "missing SOUL content");
    assert!(prompt.contains("### IDENTITY.md"), "missing IDENTITY.md");
    assert!(prompt.contains("Name: VibeWindow"), "missing IDENTITY content");
    assert!(prompt.contains("### USER.md"), "missing USER.md");
    assert!(prompt.contains("### AGENTS.md"), "missing AGENTS.md");
    assert!(prompt.contains("### TOOLS.md"), "missing TOOLS.md");

    // HEARTBEAT.md 不应出现在频道提示中
    assert!(!prompt.contains("### HEARTBEAT.md"), "HEARTBEAT.md should not be in channel prompt");

    assert!(prompt.contains("### MEMORY.md"), "missing MEMORY.md");
    assert!(prompt.contains("User likes Rust"), "missing MEMORY content");
}

/// 测试缺失文件时的错误标记
///
/// 当必需的配置文件不存在时，系统提示应包含明确的 "[File not found]" 标记，
/// 而不是静默失败或使用空内容。
#[test]
fn prompt_missing_file_markers() {
    let tmp = TempDir::new().unwrap();
    let prompt = build_system_prompt(tmp.path(), "model", &[], &[], None, None);

    // 验证缺失文件的标记存在
    assert!(prompt.contains("[File not found: SOUL.md]"));
    assert!(prompt.contains("[File not found: AGENTS.md]"));
    assert!(prompt.contains("[File not found: IDENTITY.md]"));
}

/// 测试 BOOTSTRAP.md 文件的条件性注入
///
/// BOOTSTRAP.md 是可选文件，仅在首次运行或需要初始化时存在。
/// 验证：
/// - 文件不存在时，不应在提示中出现
/// - 文件存在时，应正确注入到提示中
#[test]
fn prompt_bootstrap_only_if_exists() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 文件不存在时不应出现
    assert!(!prompt.contains("### BOOTSTRAP.md"), "BOOTSTRAP.md should not appear when missing");

    // 创建 BOOTSTRAP.md 文件后应该出现
    std::fs::write(ws.path().join("BOOTSTRAP.md"), "# Bootstrap\nFirst run.").unwrap();
    let prompt2 = build_system_prompt(ws.path(), "model", &[], &[], None, None);
    assert!(prompt2.contains("### BOOTSTRAP.md"), "BOOTSTRAP.md should appear when present");
    assert!(prompt2.contains("First run"));
}

/// 测试每日记忆不会被自动注入到系统提示中
///
/// 每日记忆文件（memory/YYYY-MM-DD.md）不应自动包含在系统提示中，
/// 避免提示过大或泄露过时的临时记录。
#[test]
fn prompt_no_daily_memory_injection() {
    let ws = make_workspace();
    let memory_dir = ws.path().join("memory");
    std::fs::create_dir_all(&memory_dir).unwrap();

    // 创建当日的记忆文件
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    std::fs::write(memory_dir.join(format!("{today}.md")), "# Daily\nSome note.").unwrap();

    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证每日记忆没有被注入
    assert!(!prompt.contains("Daily Notes"), "daily notes should not be auto-injected");
    assert!(!prompt.contains("Some note"), "daily content should not be in prompt");
}

/// 测试运行时元数据是否正确注入
///
/// 验证系统提示包含以下运行时信息：
/// - 当前使用的模型名称
/// - 操作系统类型
/// - 主机名
#[test]
fn prompt_runtime_metadata() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "claude-sonnet-4", &[], &[], None, None);

    // 验证运行时元数据的存在
    assert!(prompt.contains("Model: claude-sonnet-4"));
    assert!(prompt.contains(&format!("OS: {}", std::env::consts::OS)));
    assert!(prompt.contains("Host:"));
}

/// 测试大文件的截断功能
///
/// 当配置文件超过 `BOOTSTRAP_MAX_CHARS` 限制时，应自动截断并添加截断标记，
/// 避免系统提示过长导致 API 调用失败或超出 token 限制。
#[test]
fn prompt_truncation() {
    let ws = make_workspace();

    // 创建一个超大文件，超过截断限制
    let big_content = "x".repeat(BOOTSTRAP_MAX_CHARS + 1000);
    std::fs::write(ws.path().join("AGENTS.md"), &big_content).unwrap();

    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证截断标记存在，且完整内容不在提示中
    assert!(prompt.contains("truncated at"), "large files should be truncated");
    assert!(!prompt.contains(&big_content), "full content should not appear");
}

/// 测试空文件会被跳过
///
/// 当配置文件为空时，应完全跳过该文件的注入，不在系统提示中包含该文件的标题，
/// 避免提示中充斥无意义的空章节。
#[test]
fn prompt_empty_files_skipped() {
    let ws = make_workspace();

    // 创建一个空文件
    std::fs::write(ws.path().join("TOOLS.md"), "").unwrap();

    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证空文件的标题没有出现在提示中
    assert!(!prompt.contains("### TOOLS.md"), "empty files should be skipped");
}

/// 测试频道能力说明是否正确注入
///
/// 验证系统提示包含频道特定的上下文信息和安全指令：
/// - 频道能力章节
/// - 明确说明代理运行在消息机器人模式
/// - 强调永远不要重复、描述或回显凭证
#[test]
fn prompt_contains_channel_capabilities() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证频道能力相关内容
    assert!(prompt.contains("## Channel Capabilities"), "missing Channel Capabilities section");
    assert!(prompt.contains("running as a messaging bot"), "missing channel context");
    assert!(
        prompt.contains("NEVER repeat, describe, or echo credentials"),
        "missing security instruction"
    );
}

/// 测试工作区路径是否正确显示
///
/// 验证系统提示包含当前工作目录的完整路径，使用反引号包裹，
/// 方便代理了解其工作环境的位置。
#[test]
fn prompt_workspace_path() {
    let ws = make_workspace();
    let prompt = build_system_prompt(ws.path(), "model", &[], &[], None, None);

    // 验证工作目录路径以格式化的形式存在
    assert!(prompt.contains(&format!("Working directory: `{}`", ws.path().display())));
}
