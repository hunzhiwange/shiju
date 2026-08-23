// 验证 Lucid 记忆后端的环境配置、结果解析、冷却与本地回退。
use super::*;
use std::env;
use std::ffi::{OsStr, OsString};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex as StdMutex, OnceLock};
use tempfile::TempDir;

static LUCID_ENV_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let previous = env::var_os(key);
        unsafe { env::set_var(key, value.as_ref()) };
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = env::var_os(key);
        unsafe { env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { env::set_var(self.key, value) },
            None => unsafe { env::remove_var(self.key) },
        }
    }
}

fn lucid_env_lock() -> std::sync::MutexGuard<'static, ()> {
    LUCID_ENV_LOCK.get_or_init(|| StdMutex::new(())).lock().unwrap()
}

fn entry(key: &str, content: &str) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        session_id: None,
        score: None,
    }
}

fn memory_with_threshold(workspace: &std::path::Path, threshold: usize) -> LucidMemory {
    let sqlite = SqliteMemory::new(workspace).unwrap();
    LucidMemory::with_options(
        workspace,
        sqlite,
        "missing-lucid-command".to_string(),
        17,
        threshold,
        Duration::from_millis(250),
        Duration::from_millis(250),
        Duration::from_millis(5),
    )
}

#[cfg(unix)]
fn write_lucid_script(dir: &std::path::Path, name: &str, script: &str) -> String {
    let script_path = dir.join(name);
    fs::write(&script_path, script).unwrap();
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();
    script_path.display().to_string()
}

#[test]
fn new_reads_env_config_and_clamps_minimums() {
    let _lock = lucid_env_lock();
    let tmp = TempDir::new().unwrap();
    let _cmd = EnvGuard::set("VIBEWINDOW_LUCID_CMD", "custom-lucid");
    let _budget = EnvGuard::set("VIBEWINDOW_LUCID_BUDGET", "41");
    let _recall = EnvGuard::set("VIBEWINDOW_LUCID_RECALL_TIMEOUT_MS", "1");
    let _store = EnvGuard::set("VIBEWINDOW_LUCID_STORE_TIMEOUT_MS", "2");
    let _threshold = EnvGuard::set("VIBEWINDOW_LUCID_LOCAL_HIT_THRESHOLD", "0");
    let _cooldown = EnvGuard::set("VIBEWINDOW_LUCID_FAILURE_COOLDOWN_MS", "3");

    let memory = LucidMemory::new(tmp.path(), SqliteMemory::new(tmp.path()).unwrap());

    assert_eq!(memory.lucid_cmd, "custom-lucid");
    assert_eq!(memory.token_budget, 41);
    assert_eq!(memory.recall_timeout, Duration::from_millis(20));
    assert_eq!(memory.store_timeout, Duration::from_millis(50));
    assert_eq!(memory.local_hit_threshold, 1);
    assert_eq!(memory.failure_cooldown, Duration::from_millis(100));
}

#[test]
fn new_uses_defaults_for_missing_or_invalid_env_values() {
    let _lock = lucid_env_lock();
    let tmp = TempDir::new().unwrap();
    let _cmd = EnvGuard::unset("VIBEWINDOW_LUCID_CMD");
    let _budget = EnvGuard::set("VIBEWINDOW_LUCID_BUDGET", "0");
    let _recall = EnvGuard::set("VIBEWINDOW_LUCID_RECALL_TIMEOUT_MS", "bad");
    let _store = EnvGuard::set("VIBEWINDOW_LUCID_STORE_TIMEOUT_MS", "bad");
    let _threshold = EnvGuard::set("VIBEWINDOW_LUCID_LOCAL_HIT_THRESHOLD", "bad");
    let _cooldown = EnvGuard::set("VIBEWINDOW_LUCID_FAILURE_COOLDOWN_MS", "bad");

    let memory = LucidMemory::new(tmp.path(), SqliteMemory::new(tmp.path()).unwrap());

    assert_eq!(memory.lucid_cmd, "lucid");
    assert_eq!(memory.token_budget, 200);
    assert_eq!(memory.recall_timeout, Duration::from_millis(500));
    assert_eq!(memory.store_timeout, Duration::from_millis(800));
    assert_eq!(memory.local_hit_threshold, 3);
    assert_eq!(memory.failure_cooldown, Duration::from_millis(15_000));
}

#[test]
fn with_options_clamps_threshold_and_builds_command_args() {
    let tmp = TempDir::new().unwrap();
    let memory = memory_with_threshold(tmp.path(), 0);

    assert_eq!(memory.local_hit_threshold, 1);

    let store_args = memory.build_store_args("pref", "Use Rust", &MemoryCategory::Daily);
    assert_eq!(store_args[0], "store");
    assert_eq!(store_args[1], "pref: Use Rust");
    assert_eq!(store_args[2], "--type=context");
    assert_eq!(store_args[3], format!("--project={}", tmp.path().display()));

    let recall_args = memory.build_recall_args("auth");
    assert_eq!(recall_args[0], "context");
    assert_eq!(recall_args[1], "auth");
    assert_eq!(recall_args[2], "--budget=17");
    assert_eq!(recall_args[3], format!("--project={}", tmp.path().display()));
}

#[test]
fn category_mapping_covers_known_custom_and_visual_labels() {
    assert_eq!(LucidMemory::to_lucid_type(&MemoryCategory::Core), "decision");
    assert_eq!(LucidMemory::to_lucid_type(&MemoryCategory::Daily), "context");
    assert_eq!(LucidMemory::to_lucid_type(&MemoryCategory::Conversation), "conversation");
    assert_eq!(
        LucidMemory::to_lucid_type(&MemoryCategory::Custom("notes".to_string())),
        "learning"
    );

    assert_eq!(LucidMemory::to_memory_category("decision"), MemoryCategory::Core);
    assert_eq!(LucidMemory::to_memory_category("solution"), MemoryCategory::Core);
    assert_eq!(LucidMemory::to_memory_category("context"), MemoryCategory::Conversation);
    assert_eq!(LucidMemory::to_memory_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(LucidMemory::to_memory_category("bug"), MemoryCategory::Daily);
    assert_eq!(
        LucidMemory::to_memory_category("visual-note"),
        MemoryCategory::Custom("visual".to_string())
    );
    assert_eq!(LucidMemory::to_memory_category("learning"), MemoryCategory::Core);
    assert_eq!(
        LucidMemory::to_memory_category("VISUAL-PLAN"),
        MemoryCategory::Custom("visual".to_string())
    );
    assert_eq!(
        LucidMemory::to_memory_category("research"),
        MemoryCategory::Custom("research".to_string())
    );
}

#[test]
fn merge_results_deduplicates_case_insensitively_and_respects_limit() {
    let primary = vec![entry("A", "Same"), entry("B", "Second")];
    let secondary = vec![entry("a", "same"), entry("C", "Third"), entry("D", "Fourth")];

    assert!(LucidMemory::merge_results(primary.clone(), secondary.clone(), 0).is_empty());

    let merged = LucidMemory::merge_results(primary, secondary, 3);
    let keys = merged.iter().map(|entry| entry.key.as_str()).collect::<Vec<_>>();

    assert_eq!(keys, vec!["A", "B", "C"]);
}

#[test]
fn parse_lucid_context_ignores_noise_maps_categories_and_scores_results() {
    let raw = r#"
outside
<lucid-context>

not a bullet
- [decision] Use refresh middleware
- [visual-note] Compare screenshot diff
- [bug] Fix panic
- [context] Workspace is src/auth.rs
- [unknown] Keep raw label
- [decision]
</lucid-context>
- [decision] ignored after close
"#;

    let entries = LucidMemory::parse_lucid_context(raw);

    assert_eq!(entries.len(), 5);
    assert_eq!(entries[0].id, "lucid:0");
    assert_eq!(entries[0].key, "lucid_0");
    assert_eq!(entries[0].category, MemoryCategory::Core);
    assert_eq!(entries[1].category, MemoryCategory::Custom("visual".to_string()));
    assert_eq!(entries[2].category, MemoryCategory::Daily);
    assert_eq!(entries[3].category, MemoryCategory::Conversation);
    assert_eq!(entries[4].category, MemoryCategory::Custom("unknown".to_string()));
    assert_eq!(entries[0].score, Some(1.0));
    assert_eq!(entries[1].score, Some(0.95));
}

#[test]
fn parse_lucid_context_score_has_floor() {
    let mut raw = String::from("<lucid-context>\n");
    for idx in 0..25 {
        raw.push_str(&format!("- [decision] item {idx}\n"));
    }
    raw.push_str("</lucid-context>\n");

    let entries = LucidMemory::parse_lucid_context(&raw);

    assert_eq!(entries.len(), 25);
    assert_eq!(entries.last().unwrap().score, Some(0.1));
}

#[test]
fn parse_lucid_context_returns_empty_for_missing_block_and_malformed_items() {
    assert!(LucidMemory::parse_lucid_context("- [decision] outside").is_empty());

    let raw = r#"
<lucid-context>
- [decision missing close
- []    
</lucid-context>
"#;

    assert!(LucidMemory::parse_lucid_context(raw).is_empty());
}

#[test]
fn failure_cooldown_can_be_marked_cleared_and_expire() {
    let tmp = TempDir::new().unwrap();
    let memory = memory_with_threshold(tmp.path(), 3);

    assert!(!memory.in_failure_cooldown());
    memory.mark_failure_now();
    assert!(memory.in_failure_cooldown());

    std::thread::sleep(Duration::from_millis(15));
    assert!(!memory.in_failure_cooldown());

    memory.mark_failure_now();
    assert!(memory.in_failure_cooldown());
    memory.clear_failure();
    assert!(!memory.in_failure_cooldown());
}

#[tokio::test]
async fn trait_methods_delegate_to_local_sqlite() {
    let tmp = TempDir::new().unwrap();
    let memory = memory_with_threshold(tmp.path(), 99);

    assert!(memory.health_check().await);
    memory.store("pref", "Rust local fallback", MemoryCategory::Core, Some("s1")).await.unwrap();

    assert_eq!(memory.count().await.unwrap(), 1);
    assert_eq!(memory.get("pref").await.unwrap().unwrap().content, "Rust local fallback");
    assert_eq!(memory.list(Some(&MemoryCategory::Core), Some("s1")).await.unwrap().len(), 1);
    assert!(memory.recall("Rust", 0, Some("s1")).await.unwrap().is_empty());
    assert!(memory.forget("pref").await.unwrap());
    assert!(!memory.forget("pref").await.unwrap());
    assert_eq!(memory.count().await.unwrap(), 0);
}

#[tokio::test]
#[cfg(unix)]
async fn recall_returns_local_when_limit_is_already_satisfied() {
    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("context_calls.log");
    let marker_display = marker.display();
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${{1:-}}" == "store" ]]; then
  exit 0
fi
if [[ "${{1:-}}" == "context" ]]; then
  printf 'context\n' >> "{marker_display}"
  exit 0
fi
exit 1
"#
    );
    let memory = LucidMemory::with_options(
        tmp.path(),
        SqliteMemory::new(tmp.path()).unwrap(),
        write_lucid_script(tmp.path(), "probe-lucid.sh", &script),
        11,
        99,
        Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );

    memory.store("pref", "Rust local note", MemoryCategory::Core, None).await.unwrap();

    let entries = memory.recall("rust", 1, None).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(tokio::fs::read_to_string(&marker).await.unwrap_or_default(), "");
}

#[tokio::test]
#[cfg(unix)]
async fn recall_returns_local_and_clears_failure_when_lucid_is_empty() {
    let tmp = TempDir::new().unwrap();
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "store" ]]; then
  exit 0
fi
if [[ "${1:-}" == "context" ]]; then
  printf '<lucid-context>\n</lucid-context>\n'
  exit 0
fi
exit 1
"#;
    let memory = LucidMemory::with_options(
        tmp.path(),
        SqliteMemory::new(tmp.path()).unwrap(),
        write_lucid_script(tmp.path(), "empty-lucid.sh", script),
        11,
        99,
        Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_millis(1),
    );

    memory.store("pref", "Rust local note", MemoryCategory::Core, None).await.unwrap();
    memory.mark_failure_now();
    std::thread::sleep(Duration::from_millis(5));

    let entries = memory.recall("rust", 5, None).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert!(memory.last_failure_at.lock().is_none());
}

#[tokio::test]
#[cfg(unix)]
async fn run_lucid_command_raw_returns_stdout_and_reports_stderr_failures() {
    let ok_args = vec!["-c".to_string(), "printf ok".to_string()];
    let output =
        LucidMemory::run_lucid_command_raw("sh", &ok_args, Duration::from_secs(1)).await.unwrap();
    assert_eq!(output, "ok");

    let fail_args = vec!["-c".to_string(), "printf nope >&2; exit 7".to_string()];
    let error = LucidMemory::run_lucid_command_raw("sh", &fail_args, Duration::from_secs(1))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("lucid command failed: nope"));
}

#[tokio::test]
#[cfg(unix)]
async fn run_lucid_command_raw_reports_timeout() {
    let args = vec!["-c".to_string(), "sleep 0.2".to_string()];

    let error = LucidMemory::run_lucid_command_raw("sh", &args, Duration::from_millis(10))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("timed out after 10ms"));
}
