// 内存清理模块的单元测试
//
// 本模块包含对 `hygiene` 模块的完整测试套件，验证内存归档、清理和修剪功能。
//
// # 测试覆盖范围
//
// - **日常记忆文件归档**：验证过期的日常 Markdown 文件被正确归档
// - **会话文件归档**：验证过期的会话日志文件被正确归档
// - **执行节流**：验证基于时间间隔的执行控制机制
// - **归档文件清理**：验证超过保留期限的归档文件被正确清理
// - **数据库修剪**：验证 SQLite 后端中过期对话记录的修剪逻辑
//
// # 测试策略
//
// 所有测试使用临时目录作为工作空间，确保测试隔离且不产生副作用。
// 通过模拟不同时间点的文件创建，验证时间相关逻辑的正确性。

use super::*;
use crate::app::agent::memory::{Memory, MemoryCategory, SqliteMemory};
use tempfile::TempDir;

/// 创建默认的内存配置
///
/// 返回一个使用默认值初始化的 `MemoryConfig` 实例，用于测试场景。
///
/// # 返回值
///
/// 返回配置了默认归档和清理参数的 `MemoryConfig`
fn default_cfg() -> MemoryConfig {
    MemoryConfig::default()
}

#[test]
fn hygiene_state_path_uses_user_worktree_dir() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();

    let path = state_path(workspace).unwrap();
    let home = directories::UserDirs::new().unwrap().home_dir().to_path_buf();

    assert!(
        path.starts_with(vw_config_types::paths::home_config_dir(home).join("worktree")),
        "state path should stay in the user VibeWindow worktree state directory"
    );
    assert!(path.components().any(|component| component.as_os_str() == "workspaces"));
    assert!(
        !path.starts_with(workspace),
        "state path should not be written inside the project workspace"
    );
}

/// 测试归档过期的日常记忆文件
///
/// 验证超过归档期限的日常 Markdown 文件被移动到 archive 目录，
/// 而当天的文件保持在原位置不被移动。
///
/// # 测试步骤
///
/// 1. 创建临时工作空间和 memory 目录
/// 2. 创建一个 10 天前的旧文件和当天的文件
/// 3. 运行清理任务
/// 4. 验证旧文件被移至 archive 目录，当天文件保持原位
#[test]
fn archives_old_daily_memory_files() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    fs::create_dir_all(storage.join("memory")).unwrap();

    let old = (Local::now().date_naive() - Duration::days(10)).format("%Y-%m-%d").to_string();
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();

    let old_file = storage.join("memory").join(format!("{old}.md"));
    let today_file = storage.join("memory").join(format!("{today}.md"));
    fs::write(&old_file, "old note").unwrap();
    fs::write(&today_file, "fresh note").unwrap();

    run_if_due(&default_cfg(), workspace).unwrap();

    assert!(!old_file.exists(), "old daily file should be archived");
    assert!(
        !workspace.join("state").exists(),
        "hygiene state should not be created inside the project workspace"
    );
    assert!(
        storage.join("memory").join("archive").join(format!("{old}.md")).exists(),
        "old daily file should exist in memory/archive"
    );
    assert!(today_file.exists(), "today file should remain in place");
}

/// 测试归档过期的会话文件
///
/// 验证超过归档期限的会话日志文件被移动到 sessions/archive 目录。
///
/// # 测试步骤
///
/// 1. 创建临时工作空间和 sessions 目录
/// 2. 创建一个 10 天前的旧会话文件
/// 3. 运行清理任务
/// 4. 验证旧会话文件被移至 archive 目录
#[test]
fn archives_old_session_files() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    fs::create_dir_all(storage.join("sessions")).unwrap();

    let old = (Local::now().date_naive() - Duration::days(10)).format("%Y-%m-%d").to_string();
    let old_name = format!("{old}-agent.log");
    let old_file = storage.join("sessions").join(&old_name);
    fs::write(&old_file, "old session").unwrap();

    run_if_due(&default_cfg(), workspace).unwrap();

    assert!(!old_file.exists(), "old session file should be archived");
    assert!(
        storage.join("sessions").join("archive").join(&old_name).exists(),
        "archived session file should exist"
    );
}

/// 测试执行节流机制
///
/// 验证当两次运行间隔小于配置的时间间隔（cadence）时，
/// 第二次运行会被跳过，防止过于频繁的清理操作。
///
/// # 测试步骤
///
/// 1. 创建临时工作空间
/// 2. 创建第一个过期文件并运行清理（应成功归档）
/// 3. 创建第二个过期文件并立即再次运行清理（应被节流跳过）
/// 4. 验证第二个文件因节流而未被归档
#[test]
fn skips_second_run_within_cadence_window() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    fs::create_dir_all(storage.join("memory")).unwrap();

    let old_a = (Local::now().date_naive() - Duration::days(10)).format("%Y-%m-%d").to_string();
    let file_a = storage.join("memory").join(format!("{old_a}.md"));
    fs::write(&file_a, "first").unwrap();

    run_if_due(&default_cfg(), workspace).unwrap();
    assert!(!file_a.exists(), "first old file should be archived");

    let old_b = (Local::now().date_naive() - Duration::days(9)).format("%Y-%m-%d").to_string();
    let file_b = storage.join("memory").join(format!("{old_b}.md"));
    fs::write(&file_b, "second").unwrap();

    // 由于节流门控机制阻止连续执行，此次运行应被跳过
    run_if_due(&default_cfg(), workspace).unwrap();
    assert!(file_b.exists(), "second file should remain because run is throttled");
}

/// 测试清理过期的归档文件
///
/// 验证已归档且超过清理期限的文件会被永久删除，
/// 而仍在保留期内的归档文件不会被删除。
///
/// # 测试步骤
///
/// 1. 创建临时工作空间和 memory/archive 目录
/// 2. 创建一个 40 天前的过期归档文件和一个 5 天前的近期归档文件
/// 3. 运行清理任务
/// 4. 验证过期文件被删除，近期文件保留
#[test]
fn purges_old_memory_archives() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let storage = paths::project_data_dir(workspace).unwrap();
    let archive_dir = storage.join("memory").join("archive");
    fs::create_dir_all(&archive_dir).unwrap();

    let old = (Local::now().date_naive() - Duration::days(40)).format("%Y-%m-%d").to_string();
    let keep = (Local::now().date_naive() - Duration::days(5)).format("%Y-%m-%d").to_string();

    let old_file = archive_dir.join(format!("{old}.md"));
    let keep_file = archive_dir.join(format!("{keep}.md"));
    fs::write(&old_file, "expired").unwrap();
    fs::write(&keep_file, "recent").unwrap();

    run_if_due(&default_cfg(), workspace).unwrap();

    assert!(!old_file.exists(), "old archived file should be purged");
    assert!(keep_file.exists(), "recent archived file should remain");
}

/// 测试修剪 SQLite 后端中过期的对话记录
///
/// 验证 SQLite 数据库中超过保留期限的对话类型记忆被正确修剪，
/// 而近期对话和过期核心记忆均被保留。
///
/// # 测试步骤
///
/// 1. 创建临时工作空间并初始化 SQLiteMemory 后端
/// 2. 存储过期对话、近期对话和核心记录
/// 3. 手动修改数据库，将过期对话和核心记录的时间设为 60 天前
/// 4. 以 30 天保留期修剪对话并验证删除计数
/// 5. 验证仅过期对话被修剪，其他两条记录保留
///
/// # 技术细节
///
/// - 使用 `UPDATE` SQL 语句直接修改 `created_at` 和 `updated_at` 字段
/// - 验证不同 `MemoryCategory` 类型的不同保留策略
#[tokio::test]
async fn prunes_only_old_conversation_rows_in_sqlite_backend() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();

    // 初始化 SQLite 后端并存储测试数据
    let storage = paths::project_data_dir(workspace).unwrap();
    let mem: SqliteMemory = SqliteMemory::new(workspace).unwrap();
    mem.store("conv_old", "outdated", MemoryCategory::Conversation, None).await.unwrap();
    mem.store("conv_recent", "current", MemoryCategory::Conversation, None).await.unwrap();
    mem.store("core_keep", "durable", MemoryCategory::Core, None).await.unwrap();
    drop(mem);

    // 直接修改数据库，模拟过期数据
    let db_path = storage.join("memory").join("brain.db");
    let conn = Connection::open(&db_path).unwrap();
    let old_cutoff = (Local::now() - Duration::days(60)).to_rfc3339();
    conn.execute(
        "UPDATE memories SET created_at = ?1, updated_at = ?1 \
         WHERE key IN ('conv_old', 'core_keep')",
        params![old_cutoff],
    )
    .unwrap();
    drop(conn);

    assert_eq!(prune_conversation_rows(&storage, 30).unwrap(), 1);

    // 验证修剪结果
    let mem2: SqliteMemory = SqliteMemory::new(workspace).unwrap();
    let conv_old = mem2.get("conv_old").await.unwrap();
    assert!(conv_old.is_none(), "old conversation rows should be pruned");
    let conv_recent = mem2.get("conv_recent").await.unwrap();
    assert!(conv_recent.is_some(), "recent conversation rows should remain");
    let core_keep = mem2.get("core_keep").await.unwrap();
    assert!(core_keep.is_some(), "old core memory should remain");
}

#[test]
fn prune_conversation_rows_reports_missing_memories_table() {
    let tmp = TempDir::new().unwrap();
    let memory_dir = tmp.path().join("memory");
    fs::create_dir_all(&memory_dir).unwrap();
    let connection = Connection::open(memory_dir.join("brain.db")).unwrap();
    drop(connection);

    let error = prune_conversation_rows(tmp.path(), 30).unwrap_err();

    assert!(error.to_string().contains("memories"));
}

#[test]
fn move_to_archive_handles_nameless_and_missing_sources_conservatively() {
    let tmp = TempDir::new().unwrap();
    let archive_dir = tmp.path().join("archive");
    fs::create_dir_all(&archive_dir).unwrap();

    move_to_archive(Path::new(""), &archive_dir).unwrap();

    let missing = tmp.path().join("missing.log");
    assert!(move_to_archive(&missing, &archive_dir).is_err());
    assert!(!archive_dir.join("missing.log").exists());
}

#[test]
fn date_helpers_reject_ambiguous_and_unicode_prefixes() {
    let leap_day = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();

    assert_eq!(memory_date_from_filename("2024-02-29_.md"), Some(leap_day));
    assert_eq!(memory_date_from_filename("2024-02-29_notes.MD"), None);
    assert_eq!(memory_date_from_filename("2024-02-29.md.backup"), None);
    assert_eq!(date_prefix("2024-02-29"), Some(leap_day));
    assert_eq!(date_prefix("数据2024-02-29-session"), None);
    assert_eq!(split_name("report."), ("report", ""));
    assert_eq!(split_name(".env"), ("", "env"));
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn filesystem_scans_ignore_non_utf8_entries() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let tmp = TempDir::new().unwrap();
    let storage = tmp.path();
    let memory_dir = storage.join("memory");
    let sessions_dir = storage.join("sessions");
    let memory_archive = memory_dir.join("archive");
    let sessions_archive = sessions_dir.join("archive");
    fs::create_dir_all(&memory_archive).unwrap();
    fs::create_dir_all(&sessions_archive).unwrap();

    let invalid_markdown = OsString::from_vec(vec![0xff, b'.', b'm', b'd']);
    let invalid_session = OsString::from_vec(vec![0xff, b'.', b'l', b'o', b'g']);
    let memory_file = memory_dir.join(&invalid_markdown);
    let session_file = sessions_dir.join(&invalid_session);
    let archived_memory_file = memory_archive.join(&invalid_markdown);
    let archived_session_file = sessions_archive.join(&invalid_session);
    for path in [&memory_file, &session_file, &archived_memory_file, &archived_session_file] {
        if let Err(error) = fs::write(path, "keep") {
            if error.raw_os_error() == Some(92) {
                return;
            }
            panic!("non-UTF-8 fixture should be written: {error}");
        }
    }

    assert_eq!(archive_daily_memory_files(storage, 1).unwrap(), 0);
    assert_eq!(archive_session_files(storage, 1).unwrap(), 0);
    assert_eq!(purge_memory_archives(storage, 1).unwrap(), 0);
    assert_eq!(purge_session_archives(storage, 1).unwrap(), 0);
    assert!(memory_file.exists());
    assert!(session_file.exists());
    assert!(archived_memory_file.exists());
    assert!(archived_session_file.exists());
}
