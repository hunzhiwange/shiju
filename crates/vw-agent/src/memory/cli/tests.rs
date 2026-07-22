//! 内存 CLI 模块的单元测试
//!
//! 本模块包含对 `parse_category` 和 `truncate_content` 等辅助函数的测试用例。
//! 这些测试验证了内存分类解析和内容截断的正确性。

use super::*;
use crate::app::agent::config::{Config, StorageProviderConfig};
use crate::app::agent::memory::MemoryEntry;
use std::sync::Mutex;

#[derive(Debug, clap::Parser)]
#[command(name = "memory-test")]
struct MemoryTestCli {
    #[command(subcommand)]
    command: MemoryCommands,
}

/// 测试 `parse_category` 函数对已知分类变体的解析
///
/// # 验证点
/// - 能够正确解析标准分类：core、daily、conversation
/// - 解析不区分大小写（CORE 应解析为 Core）
/// - 解析能够忽略首尾空白字符（"  Daily  " 应解析为 Daily）
#[test]
fn parse_category_known_variants() {
    assert_eq!(parse_category("core"), MemoryCategory::Core);
    assert_eq!(parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(parse_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(parse_category("CORE"), MemoryCategory::Core);
    assert_eq!(parse_category("  Daily  "), MemoryCategory::Daily);
}

/// 测试 `parse_category` 函数对自定义分类的回退处理
///
/// # 验证点
/// - 无法识别的分类字符串应作为自定义分类（Custom）返回
/// - 自定义分类保留原始字符串值
#[test]
fn parse_category_custom_fallback() {
    assert_eq!(parse_category("project_notes"), MemoryCategory::Custom("project_notes".into()));
    assert_eq!(parse_category("  Mixed Case  "), MemoryCategory::Custom("mixed case".into()));
}

/// 测试 `truncate_content` 函数对短文本的处理
///
/// # 验证点
/// - 当文本长度小于最大长度时，应保持原样返回
#[test]
fn truncate_content_short_text_unchanged() {
    assert_eq!(truncate_content("hello", 10), "hello");
}

/// 测试 `truncate_content` 函数对长文本的截断处理
///
/// # 验证点
/// - 当文本长度超过最大长度时，应进行截断
/// - 截断后的文本应以 "..." 结尾
/// - 截断后的总字符数不应超过指定的最大长度
#[test]
fn truncate_content_long_text_truncated() {
    let result = truncate_content("this is a very long string", 10);
    assert!(result.ends_with("..."));
    assert!(result.chars().count() <= 10);
}

/// 测试 `truncate_content` 函数对多行文本的处理
///
/// # 验证点
/// - 多行文本应只使用第一行内容
/// - 第一行提取后再应用截断规则
#[test]
fn truncate_content_multiline_uses_first_line() {
    assert_eq!(truncate_content("first\nsecond", 20), "first");
}

/// 测试 `truncate_content` 函数对空字符串的处理
///
/// # 验证点
/// - 空字符串应原样返回，不发生任何变化
#[test]
fn truncate_content_empty_string() {
    assert_eq!(truncate_content("", 10), "");
}

#[test]
fn truncate_content_handles_tiny_limits() {
    assert_eq!(truncate_content("abcdef", 0), "...");
    assert_eq!(truncate_content("abcdef", 2), "...");
    assert_eq!(truncate_content("abcdef", 3), "...");
}

#[test]
fn memory_commands_round_trip_through_json() {
    let commands = [
        MemoryCommands::List {
            category: Some("core".into()),
            session: Some("session-1".into()),
            limit: 25,
            offset: 5,
        },
        MemoryCommands::Get { key: "alpha".into() },
        MemoryCommands::Stats,
        MemoryCommands::Clear { key: Some("alpha".into()), category: None, yes: true },
    ];

    for command in commands {
        let json = serde_json::to_string(&command).expect("command serializes");
        let decoded: MemoryCommands = serde_json::from_str(&json).expect("command deserializes");
        assert_eq!(decoded, command);
    }
}

#[test]
fn parse_category_blank_values_become_empty_custom_category() {
    assert_eq!(parse_category("   "), MemoryCategory::Custom(String::new()));
}

#[test]
fn truncate_content_exact_boundary_is_unchanged() {
    assert_eq!(truncate_content("abcdef", 6), "abcdef");
    assert_eq!(truncate_content("abcdef", 5), "ab...");
}

#[test]
fn truncate_content_truncates_unicode_on_character_boundaries() {
    assert_eq!(truncate_content("甲乙丙丁戊己庚辛", 6), "甲乙丙...");
}

#[test]
fn memory_commands_parse_list_defaults() {
    let cli = <MemoryTestCli as clap::Parser>::try_parse_from(["memory-test", "list"])
        .expect("list command parses");

    assert_eq!(
        cli.command,
        MemoryCommands::List { category: None, session: None, limit: 50, offset: 0 }
    );
}

#[test]
fn memory_commands_parse_list_filters_and_pagination() {
    let cli = <MemoryTestCli as clap::Parser>::try_parse_from([
        "memory-test",
        "list",
        "--category",
        "core",
        "--session",
        "session-1",
        "--limit",
        "25",
        "--offset",
        "5",
    ])
    .expect("filtered list command parses");

    assert_eq!(
        cli.command,
        MemoryCommands::List {
            category: Some("core".into()),
            session: Some("session-1".into()),
            limit: 25,
            offset: 5,
        }
    );
}

#[test]
fn memory_commands_parse_get_stats_and_clear_variants() {
    let get =
        <MemoryTestCli as clap::Parser>::try_parse_from(["memory-test", "get", "project-notes"])
            .expect("get command parses");
    let stats = <MemoryTestCli as clap::Parser>::try_parse_from(["memory-test", "stats"])
        .expect("stats command parses");
    let clear = <MemoryTestCli as clap::Parser>::try_parse_from([
        "memory-test",
        "clear",
        "--key",
        "project-notes",
        "--category",
        "daily",
        "--yes",
    ])
    .expect("clear command parses");

    assert_eq!(get.command, MemoryCommands::Get { key: "project-notes".into() });
    assert_eq!(stats.command, MemoryCommands::Stats);
    assert_eq!(
        clear.command,
        MemoryCommands::Clear {
            key: Some("project-notes".into()),
            category: Some("daily".into()),
            yes: true,
        }
    );
}

#[test]
fn memory_commands_reject_unknown_subcommand() {
    let error = <MemoryTestCli as clap::Parser>::try_parse_from(["memory-test", "unknown"])
        .expect_err("unknown command should be rejected");

    assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
}

#[test]
fn create_cli_memory_rejects_disabled_backend() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.memory.backend = "none".into();
    config.workspace_dir = tmp.path().to_path_buf();

    let error = match create_cli_memory(&config) {
        Ok(_) => panic!("disabled memory should be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("disabled"));
}

#[test]
fn create_cli_memory_prefers_storage_provider_override() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "none".into();
    config.storage.provider.config = StorageProviderConfig {
        provider: "postgres".into(),
        db_url: None,
        ..StorageProviderConfig::default()
    };

    let error = match create_cli_memory(&config) {
        Ok(_) => panic!("postgres override without db_url should fail"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("postgres"));
    assert!(!message.contains("disabled"));
}

#[test]
fn create_cli_memory_rejects_mariadb_without_connection_settings() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "mariadb".into();

    let error = match create_cli_memory(&config) {
        Ok(_) => panic!("mariadb without db_url should fail"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("mariadb") || message.contains("memory-mariadb"));
}

#[test]
fn create_cli_memory_normalizes_and_creates_markdown_backend() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "  MARKDOWN  ".into();

    let memory = create_cli_memory(&config).expect("markdown backend should be available");

    assert_eq!(memory.name(), "markdown");
}

#[test]
fn create_cli_memory_ignores_blank_storage_provider_override() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "none".into();
    config.storage.provider.config.provider = "   ".into();

    let error = match create_cli_memory(&config) {
        Ok(_) => panic!("blank provider should preserve the disabled memory backend"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("disabled"));
}

#[tokio::test]
async fn handle_command_routes_all_variants_to_disabled_backend_rejection() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "none".into();
    let commands = [
        MemoryCommands::List { category: None, session: None, limit: 50, offset: 0 },
        MemoryCommands::Get { key: "alpha".into() },
        MemoryCommands::Stats,
        MemoryCommands::Clear { key: None, category: None, yes: true },
    ];

    for command in commands {
        let error = handle_command(command, &config)
            .await
            .expect_err("disabled backend should reject every command");
        assert!(error.to_string().contains("disabled"));
    }
}

#[tokio::test]
async fn empty_markdown_backend_handles_all_commands_without_external_services() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().to_path_buf();
    config.memory.backend = "markdown".into();

    handle_command(
        MemoryCommands::List {
            category: Some("core".into()),
            session: Some("session-1".into()),
            limit: 1,
            offset: usize::MAX,
        },
        &config,
    )
    .await
    .expect("an empty backend should produce an empty list");
    handle_command(MemoryCommands::Get { key: "missing".into() }, &config)
        .await
        .expect("a missing key should be reported without failing");
    handle_command(MemoryCommands::Stats, &config)
        .await
        .expect("empty-backend statistics should remain available");
    handle_command(
        MemoryCommands::Clear { key: None, category: Some("daily".into()), yes: true },
        &config,
    )
    .await
    .expect("clearing an empty category should be a no-op");
}

#[derive(Default)]
struct StubMemory {
    entries: Mutex<Vec<MemoryEntry>>,
    forgotten: Mutex<Vec<String>>,
    get_error: Option<&'static str>,
    list_error: Option<&'static str>,
    forget_error: Option<&'static str>,
    forget_result: Option<bool>,
}

impl StubMemory {
    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self { entries: Mutex::new(entries), forgotten: Mutex::new(Vec::new()), ..Self::default() }
    }

    fn forgotten_keys(&self) -> Vec<String> {
        self.forgotten.lock().unwrap().clone()
    }
}

fn entry(key: &str, category: MemoryCategory, session_id: Option<&str>) -> MemoryEntry {
    MemoryEntry {
        id: format!("id-{key}"),
        key: key.to_string(),
        content: format!("content for {key}"),
        category,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        session_id: session_id.map(str::to_string),
        score: None,
    }
}

#[async_trait::async_trait]
impl Memory for StubMemory {
    fn name(&self) -> &str {
        "stub"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        if let Some(message) = self.get_error {
            anyhow::bail!(message);
        }
        Ok(self.entries.lock().unwrap().iter().find(|entry| entry.key == key).cloned())
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        if let Some(message) = self.list_error {
            anyhow::bail!(message);
        }
        let entries = self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| match category {
                Some(category) => &entry.category == category,
                None => true,
            })
            .filter(|entry| match session_id {
                Some(session_id) => entry.session_id.as_deref() == Some(session_id),
                None => true,
            })
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.forgotten.lock().unwrap().push(key.to_string());
        if let Some(message) = self.forget_error {
            anyhow::bail!(message);
        }
        Ok(self
            .forget_result
            .unwrap_or_else(|| self.entries.lock().unwrap().iter().any(|entry| entry.key == key)))
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.lock().unwrap().len())
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn clear_key_deletes_exact_match_without_prompt_when_confirmed() {
    let memory = StubMemory::with_entries(vec![
        entry("alpha", MemoryCategory::Core, None),
        entry("alphabet", MemoryCategory::Daily, None),
    ]);

    handle_clear_key(&memory, "alpha", true).await.unwrap();

    assert_eq!(memory.forgotten_keys(), vec!["alpha"]);
}

#[tokio::test]
async fn clear_key_deletes_unique_prefix_match() {
    let memory = StubMemory::with_entries(vec![
        entry("alpha", MemoryCategory::Core, None),
        entry("beta", MemoryCategory::Daily, None),
    ]);

    handle_clear_key(&memory, "alp", true).await.unwrap();

    assert_eq!(memory.forgotten_keys(), vec!["alpha"]);
}

#[tokio::test]
async fn clear_key_leaves_missing_prefix_untouched() {
    let memory = StubMemory::with_entries(vec![entry("alpha", MemoryCategory::Core, None)]);

    handle_clear_key(&memory, "missing", false).await.unwrap();

    assert!(memory.forgotten_keys().is_empty());
}

#[tokio::test]
async fn clear_key_leaves_ambiguous_prefix_untouched() {
    let memory = StubMemory::with_entries(vec![
        entry("alpha", MemoryCategory::Core, None),
        entry("alphabet", MemoryCategory::Daily, None),
    ]);

    handle_clear_key(&memory, "alph", false).await.unwrap();

    assert!(memory.forgotten_keys().is_empty());
}

#[tokio::test]
async fn clear_key_propagates_exact_lookup_error_without_forgetting() {
    let memory =
        StubMemory { get_error: Some("get failed"), ..StubMemory::with_entries(Vec::new()) };

    let error =
        handle_clear_key(&memory, "alpha", true).await.expect_err("get failure should be returned");

    assert_eq!(error.to_string(), "get failed");
    assert!(memory.forgotten_keys().is_empty());
}

#[tokio::test]
async fn clear_key_propagates_prefix_list_error_without_forgetting() {
    let memory =
        StubMemory { list_error: Some("list failed"), ..StubMemory::with_entries(Vec::new()) };

    let error = handle_clear_key(&memory, "alpha", true)
        .await
        .expect_err("list failure should be returned");

    assert_eq!(error.to_string(), "list failed");
    assert!(memory.forgotten_keys().is_empty());
}

#[tokio::test]
async fn clear_key_propagates_forget_error_for_resolved_target() {
    let memory = StubMemory {
        forget_error: Some("forget failed"),
        ..StubMemory::with_entries(vec![entry("alpha", MemoryCategory::Core, None)])
    };

    let error = handle_clear_key(&memory, "alpha", true)
        .await
        .expect_err("forget failure should be returned");

    assert_eq!(error.to_string(), "forget failed");
    assert_eq!(memory.forgotten_keys(), vec!["alpha"]);
}

#[tokio::test]
async fn clear_key_calls_forget_once_when_backend_reports_not_deleted() {
    let memory = StubMemory {
        forget_result: Some(false),
        ..StubMemory::with_entries(vec![entry("alpha", MemoryCategory::Core, None)])
    };

    handle_clear_key(&memory, "alpha", true).await.unwrap();

    assert_eq!(memory.forgotten_keys(), vec!["alpha"]);
}
