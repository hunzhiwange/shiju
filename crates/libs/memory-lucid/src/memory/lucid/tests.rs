//! LucidMemory 单元测试模块
//!
//! 本模块提供 LucidMemory 的完整测试覆盖，验证以下核心行为：
//! - 本地 SQLite 与远程 Lucid 服务的协作
//! - 故障降级与重试机制
//! - 本地命中短路优化
//! - 冷启动延迟容忍
//!
//! # 测试策略
//!
//! 通过动态生成模拟 Lucid 脚本（fake/delayed/probe/failing），
//! 在隔离环境中验证各种边界条件，无需依赖真实 Lucid 服务。

use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

/// 创建模拟正常行为的 Lucid 脚本
///
/// 该脚本模拟 Lucid 服务的标准响应：
/// - `store` 命令返回成功并生成内存 ID
/// - `context` 命令返回包含决策和上下文的 XML 片段
///
/// # 参数
///
/// * `dir` - 脚本文件的输出目录
///
/// # 返回
///
/// 返回生成的脚本文件的绝对路径字符串
///
/// # 示例
///
/// ```ignore
/// let tmp = TempDir::new().unwrap();
/// let script_path = write_fake_lucid_script(tmp.path());
/// // script_path 可用于 LucidMemory 配置
/// ```
fn write_fake_lucid_script(dir: &Path) -> String {
    let script_path = dir.join("fake-lucid.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "store" ]]; then
  echo '{"success":true,"id":"mem_1"}'
  exit 0
fi

if [[ "${1:-}" == "context" ]]; then
  cat <<'EOF'
<lucid-context>
Auth context snapshot
- [decision] Use token refresh middleware
- [context] Working in src/auth.rs
</lucid-context>
EOF
  exit 0
fi

echo "unsupported command" >&2
exit 1
"#;

    // 写入脚本内容
    fs::write(&script_path, script).unwrap();
    // 设置可执行权限：所有者可读写执行，其他用户可读执行
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();
    script_path.display().to_string()
}

/// 创建带延迟的模拟 Lucid 脚本
///
/// 该脚本在 `context` 命令中引入 0.2 秒延迟，
/// 用于测试 LucidMemory 对冷启动或网络延迟的容忍能力。
///
/// # 参数
///
/// * `dir` - 脚本文件的输出目录
///
/// # 返回
///
/// 返回生成的脚本文件的绝对路径字符串
fn write_delayed_lucid_script(dir: &Path) -> String {
    let script_path = dir.join("delayed-lucid.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "store" ]]; then
  echo '{"success":true,"id":"mem_1"}'
  exit 0
fi

if [[ "${1:-}" == "context" ]]; then
  sleep 0.2
  cat <<'EOF'
<lucid-context>
- [decision] Delayed token refresh guidance
</lucid-context>
EOF
  exit 0
fi

echo "unsupported command" >&2
exit 1
"#;

    fs::write(&script_path, script).unwrap();
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();
    script_path.display().to_string()
}

/// 创建探测型模拟 Lucid 脚本
///
/// 该脚本在执行 `context` 命令时，会向指定的标记文件追加记录。
/// 用于验证 LucidMemory 是否在本地命中足够时跳过远程调用（短路优化）。
///
/// # 参数
///
/// * `dir` - 脚本文件的输出目录
/// * `marker_path` - 调用标记文件的路径，每次 context 调用都会追加一行
///
/// # 返回
///
/// 返回生成的脚本文件的绝对路径字符串
fn write_probe_lucid_script(dir: &Path, marker_path: &Path) -> String {
    let script_path = dir.join("probe-lucid.sh");
    let marker = marker_path.display().to_string();
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${{1:-}}" == "store" ]]; then
  echo '{{"success":true,"id":"mem_store"}}'
  exit 0
fi

if [[ "${{1:-}}" == "context" ]]; then
  printf 'context\n' >> "{marker}"
  cat <<'EOF'
<lucid-context>
- [decision] should not be used when local hits are enough
</lucid-context>
EOF
  exit 0
fi

echo "unsupported command" >&2
exit 1
"#
    );

    fs::write(&script_path, script).unwrap();
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();
    script_path.display().to_string()
}

/// 构造标准测试用的 LucidMemory 实例
///
/// 使用固定参数创建 LucidMemory，确保测试的一致性：
/// - 最大结果数：200
/// - 最小本地命中数：3（低于此值才会调用 Lucid）
/// - 超时配置：5秒
/// - 冷却期：2秒
///
/// # 参数
///
/// * `workspace` - 工作空间目录，用于 SQLite 数据库存储
/// * `cmd` - Lucid 可执行文件或脚本的路径
///
/// # 返回
///
/// 返回配置好的 LucidMemory 实例
fn test_memory(workspace: &Path, cmd: String) -> LucidMemory {
    let sqlite = SqliteMemory::new(workspace).unwrap();
    LucidMemory::with_options(
        workspace,
        sqlite,
        cmd,
        200,                    // max_results
        3,                      // min_local_hits
        Duration::from_secs(5), // lucid_timeout
        Duration::from_secs(5), // store_timeout
        Duration::from_secs(2), // failure_cooldown
    )
}

/// 验证 LucidMemory::name 返回固定标识符
///
/// 即使 Lucid 可执行文件不存在，name() 方法也应返回 "lucid"，
/// 这是 Memory trait 的契约要求。
#[tokio::test]
async fn lucid_name() {
    let tmp = TempDir::new().unwrap();
    let memory = test_memory(tmp.path(), "nonexistent-lucid-binary".to_string());
    assert_eq!(memory.name(), "lucid");
}

/// 验证 Lucid 不存在时 store 仍然成功
///
/// 当 Lucid 可执行文件缺失时，store 操作应降级为仅本地存储，
/// 不应返回错误。这确保了 LucidMemory 的健壮性。
#[tokio::test]
async fn store_succeeds_when_lucid_missing() {
    let tmp = TempDir::new().unwrap();
    let memory = test_memory(tmp.path(), "nonexistent-lucid-binary".to_string());

    // 执行存储操作，即使 Lucid 不存在也应成功
    memory.store("lang", "User prefers Rust", MemoryCategory::Core, None).await.unwrap();

    // 验证数据已存入本地 SQLite
    let entry = memory.get("lang").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().content, "User prefers Rust");
}

/// 验证 recall 会合并 Lucid 和本地的检索结果
///
/// 当本地结果不足时，应调用 Lucid 并将其返回的上下文
/// 与本地 SQLite 结果合并返回给调用者。
#[tokio::test]
async fn recall_merges_lucid_and_local_results() {
    let tmp = TempDir::new().unwrap();
    let fake_cmd = write_fake_lucid_script(tmp.path());
    let memory = test_memory(tmp.path(), fake_cmd);

    // 先存入一条本地记忆
    memory
        .store("local_note", "Local sqlite auth fallback note", MemoryCategory::Core, None)
        .await
        .unwrap();

    // 执行检索，应合并本地和 Lucid 结果
    let entries = memory.recall("auth", 5, None).await.unwrap();

    // 验证本地结果存在
    assert!(entries.iter().any(|e| e.content.contains("Local sqlite auth fallback note")));
    // 验证 Lucid 结果存在（来自模拟脚本的 token refresh 决策）
    assert!(entries.iter().any(|e| e.content.contains("token refresh")));
}

/// 验证 recall 能容忍 Lucid 冷启动延迟
///
/// 当 Lucid 服务响应较慢（如冷启动）时，只要在超时时间内完成，
/// 结果应被正确合并。此测试使用 0.2 秒延迟的模拟脚本。
#[tokio::test]
async fn recall_handles_lucid_cold_start_delay_within_timeout() {
    let tmp = TempDir::new().unwrap();
    let delayed_cmd = write_delayed_lucid_script(tmp.path());
    let memory = test_memory(tmp.path(), delayed_cmd);

    // 存入本地记忆
    memory
        .store("local_note", "Local sqlite auth fallback note", MemoryCategory::Core, None)
        .await
        .unwrap();

    // 执行检索，延迟脚本应在 5 秒超时内完成
    let entries = memory.recall("auth", 5, None).await.unwrap();

    // 验证本地和延迟的 Lucid 结果都被包含
    assert!(entries.iter().any(|e| e.content.contains("Local sqlite auth fallback note")));
    assert!(entries.iter().any(|e| e.content.contains("Delayed token refresh guidance")));
}

/// 验证本地命中足够时跳过 Lucid 调用（短路优化）
///
/// 当本地 SQLite 返回的结果数达到 min_local_hits 阈值时，
/// 应跳过远程 Lucid 调用以减少延迟和资源消耗。
///
/// 本测试使用探测脚本，如果 Lucid 被调用，会在标记文件中留下记录。
#[tokio::test]
async fn recall_skips_lucid_when_local_hits_are_enough() {
    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("context_calls.log");
    let probe_cmd = write_probe_lucid_script(tmp.path(), &marker);

    // 创建 min_local_hits=1 的配置，测试短路行为
    let sqlite = SqliteMemory::new(tmp.path()).unwrap();
    let memory = LucidMemory::with_options(
        tmp.path(),
        sqlite,
        probe_cmd,
        200, // max_results
        1,   // min_local_hits - 只要本地有 1 条命中就跳过 Lucid
        Duration::from_secs(5),
        Duration::from_secs(5),
        Duration::from_secs(2),
    );

    // 存入本地记忆
    memory.store("pref", "Rust should stay local-first", MemoryCategory::Core, None).await.unwrap();

    // 执行检索，由于本地有匹配结果且达到阈值，应跳过 Lucid
    let entries = memory.recall("rust", 5, None).await.unwrap();
    assert!(entries.iter().any(|e| e.content.contains("Rust should stay local-first")));

    // 验证 Lucid 未被调用：标记文件应为空
    let context_calls = tokio::fs::read_to_string(&marker).await.unwrap_or_default();
    assert!(
        context_calls.trim().is_empty(),
        "Expected local-hit short-circuit; got calls: {context_calls}"
    );
}

/// 创建会失败的模拟 Lucid 脚本
///
/// 该脚本在 `context` 命令时返回错误退出码，
/// 用于测试 LucidMemory 的故障处理和冷却机制。
///
/// # 参数
///
/// * `dir` - 脚本文件的输出目录
/// * `marker_path` - 调用标记文件的路径
///
/// # 返回
///
/// 返回生成的脚本文件的绝对路径字符串
fn write_failing_lucid_script(dir: &Path, marker_path: &Path) -> String {
    let script_path = dir.join("failing-lucid.sh");
    let marker = marker_path.display().to_string();
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${{1:-}}" == "store" ]]; then
  echo '{{"success":true,"id":"mem_store"}}'
  exit 0
fi

if [[ "${{1:-}}" == "context" ]]; then
  printf 'context\n' >> "{marker}"
  echo "simulated lucid failure" >&2
  exit 1
fi

echo "unsupported command" >&2
exit 1
"#
    );

    fs::write(&script_path, script).unwrap();
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();
    script_path.display().to_string()
}

/// 验证失败冷却机制避免重复 Lucid 调用
///
/// 当 Lucid 调用失败后，在冷却期内应跳过后续调用，
/// 防止对故障服务的持续轰炸。此测试配置了 5 秒冷却期。
///
/// 测试流程：
/// 1. 第一次 recall 触发 Lucid 调用并失败
/// 2. 第二次 recall 应因冷却期跳过 Lucid
/// 3. 验证标记文件只有一条调用记录
#[tokio::test]
async fn failure_cooldown_avoids_repeated_lucid_calls() {
    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("failing_context_calls.log");
    let failing_cmd = write_failing_lucid_script(tmp.path(), &marker);

    // 配置较大的 min_local_hits 以确保会尝试调用 Lucid
    let sqlite = SqliteMemory::new(tmp.path()).unwrap();
    let memory = LucidMemory::with_options(
        tmp.path(),
        sqlite,
        failing_cmd,
        200,
        99, // min_local_hits - 设置高阈值，强制调用 Lucid
        Duration::from_secs(5),
        Duration::from_secs(5),
        Duration::from_secs(5), // failure_cooldown - 5秒冷却期
    );

    // 第一次调用：触发 Lucid 并失败
    let first = memory.recall("auth", 5, None).await.unwrap();
    // 第二次调用：因冷却期跳过 Lucid
    let second = memory.recall("auth", 5, None).await.unwrap();

    // 两次都应返回空结果（本地无数据，Lucid 失败）
    assert!(first.is_empty());
    assert!(second.is_empty());

    // 验证只调用了一次 Lucid（第二次被冷却期阻止）
    let calls = tokio::fs::read_to_string(&marker).await.unwrap_or_default();
    assert_eq!(calls.lines().count(), 1);
}
