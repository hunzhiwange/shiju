//! 解析功能单元测试模块
//!
//! 本模块提供对迁移解析相关函数的单元测试，验证 Markdown 内存文件的解析行为。
//!
//! # 测试覆盖
//!
//! - 结构化 Markdown 行解析（`parse_structured_memory_line`）
//! - 非结构化 Markdown 文件解析（`parse_markdown_file`）
//! - 内存分类解析（`parse_category`）
//! - 键标准化处理（`normalize_key`）
//! - Markdown 条目读取与排序（`read_openclaw_markdown_entries`）
//!
//! # 确定性保证
//!
//! 测试确保解析结果在不同运行间保持确定性，包括键的生成和条目的排序顺序。

use super::super::*;
use agent_memory::MemoryCategory;
use std::path::Path;

/// 测试结构化 Markdown 行的正确解析
///
/// 验证 `parse_structured_memory_line` 能够正确解析格式为 `**key**: value` 的行，
/// 并准确提取键和值部分。
#[test]
fn parse_structured_markdown_line() {
    let line = "**user_pref**: likes Rust";
    let parsed = parse_structured_memory_line(line).unwrap();
    assert_eq!(parsed.0, "user_pref");
    assert_eq!(parsed.1, "likes Rust");
}

/// 测试非结构化 Markdown 内容的键自动生成
///
/// 当 Markdown 内容不符合结构化格式时，`parse_markdown_file` 应自动生成
/// 带有 `openclaw_<category>_` 前缀的唯一键。
#[test]
fn parse_unstructured_markdown_generates_key() {
    let entries = parse_markdown_file(
        Path::new("/tmp/MEMORY.md"),
        "- plain note",
        MemoryCategory::Core,
        "core",
    );
    assert_eq!(entries.len(), 1);
    assert!(entries[0].key.starts_with("openclaw_core_"));
    assert_eq!(entries[0].content, "plain note");
}

/// 测试 `parse_category` 对所有分类变体的处理
///
/// 验证函数能够正确解析内置分类（core、daily、conversation）、
/// 空字符串（回退到 Core）以及自定义分类字符串。
#[test]
fn parse_category_handles_all_variants() {
    assert_eq!(parse_category("core"), MemoryCategory::Core);
    assert_eq!(parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(parse_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(parse_category(""), MemoryCategory::Core);
    assert_eq!(parse_category("custom_type"), MemoryCategory::Custom("custom_type".to_string()));
}

/// 测试 `parse_category` 的大小写不敏感行为
///
/// 验证分类名称解析不区分大小写，`CORE`、`Daily`、`CONVERSATION`
/// 等变体应正确映射到对应的枚举值。
#[test]
fn parse_category_case_insensitive() {
    assert_eq!(parse_category("CORE"), MemoryCategory::Core);
    assert_eq!(parse_category("Daily"), MemoryCategory::Daily);
    assert_eq!(parse_category("CONVERSATION"), MemoryCategory::Conversation);
}

/// 测试 `normalize_key` 对空字符串的处理
///
/// 当键为空字符串时，函数应生成 `openclaw_<counter>` 格式的默认键。
#[test]
fn normalize_key_handles_empty_string() {
    let key = normalize_key("", 42);
    assert_eq!(key, "openclaw_42");
}

/// 测试 `normalize_key` 的空白字符修剪功能
///
/// 验证函数能够去除键首尾的空白字符，确保生成的键不包含多余空格。
#[test]
fn normalize_key_trims_whitespace() {
    let key = normalize_key("  my_key  ", 0);
    assert_eq!(key, "my_key");
}

/// 测试 `parse_structured_memory_line` 拒绝空键
///
/// 格式 `****:value`（键为空）应被拒绝，返回 `None`。
#[test]
fn parse_structured_markdown_rejects_empty_key() {
    assert!(parse_structured_memory_line("****:value").is_none());
}

/// 测试 `parse_structured_memory_line` 拒绝空值
///
/// 格式 `**key**:`（值为空）应被拒绝，返回 `None`。
#[test]
fn parse_structured_markdown_rejects_empty_value() {
    assert!(parse_structured_memory_line("**key**:").is_none());
}

/// 测试 `parse_structured_memory_line` 拒绝缺少星号的格式
///
/// 纯文本格式 `key: value`（无 `**` 标记）应被拒绝，返回 `None`，
/// 因为它不符合结构化 Markdown 的约定格式。
#[test]
fn parse_structured_markdown_rejects_no_stars() {
    assert!(parse_structured_memory_line("key: value").is_none());
}

/// 测试 Markdown 条目的确定性排序顺序
///
/// 验证 `read_openclaw_markdown_entries` 读取多个文件时，
/// 返回的条目按文件名字母顺序排列，确保跨运行的结果一致。
///
/// # 测试步骤
///
/// 1. 创建临时目录及 `memory/` 子目录
/// 2. 写入两个文件 `b.md` 和 `a.md`（故意乱序）
/// 3. 读取并验证结果顺序为 `a.md` 在前、`b.md` 在后
#[test]
fn markdown_entries_are_sorted_for_deterministic_order() {
    let source = tempfile::TempDir::new().unwrap();
    let memory_dir = source.path().join("memory");
    std::fs::create_dir_all(&memory_dir).unwrap();
    std::fs::write(memory_dir.join("b.md"), "- b note").unwrap();
    std::fs::write(memory_dir.join("a.md"), "- a note").unwrap();

    let entries = read_openclaw_markdown_entries(source.path()).unwrap();
    let keys: Vec<String> = entries.into_iter().map(|entry| entry.key).collect();

    assert_eq!(keys, vec!["openclaw_a_1", "openclaw_b_1"]);
}
