//! 子代理注册表测试模块
//!
//! 本模块提供 `SubAgentRegistry` 的完整单元测试套件，验证子代理会话管理的各项功能：
//! - 会话的插入、查询和列举
//! - 会话状态转换（完成、失败、终止）
//! - 按状态过滤会话列表
//! - 过期会话的自动清理
//! - 并发访问的线程安全性
//! - 任务描述截断功能
//! - 会话信息的序列化

use super::super::*;
use super::*;

/// 创建测试用的子代理会话实例
///
/// # 参数
///
/// - `id`: 会话的唯一标识符
/// - `agent`: 代理名称（如 "researcher"、"coder"）
/// - `task`: 分配给子代理的任务描述
///
/// # 返回值
///
/// 返回一个状态为 `Running` 的新建 `SubAgentSession` 实例，
/// 用于测试场景中快速创建会话对象。
fn make_session(id: &str, agent: &str, task: &str) -> SubAgentSession {
    SubAgentSession {
        id: id.to_string(),
        agent_name: agent.to_string(),
        title: None,
        task: task.to_string(),
        metadata: serde_json::Value::Object(Default::default()),
        status: SubAgentStatus::Running,
        started_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        result: None,
        handle: None,
    }
}

/// 测试会话的插入和列表功能
///
/// 验证：
/// - 能够成功插入多个会话
/// - `list("all")` 返回所有已插入的会话
/// - 返回的会话数量正确
#[test]
fn registry_insert_and_list() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "find info"));
    registry.insert(make_session("s2", "coder", "write code"));

    let all = registry.list(Some("all"));
    assert_eq!(all.len(), 2);
}

/// 测试会话完成功能
///
/// 验证：
/// - 能够将会话标记为已完成
/// - 完成后状态正确更新为 `Completed`
/// - `completed_at` 时间戳被正确设置
/// - 执行结果被正确存储
#[test]
fn registry_complete_session() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "find info"));

    registry.complete("s1", ToolResult { success: true, output: "done".to_string(), error: None });

    let snap = registry.get_status("s1").unwrap();
    assert_eq!(snap.status, SubAgentStatus::Completed);
    assert!(snap.completed_at.is_some());
    assert!(snap.result.unwrap().success);
}

/// 测试会话失败功能
///
/// 验证：
/// - 能够将会话标记为失败
/// - 失败后状态正确更新为 `Failed`
/// - 失败结果中 `success` 标志为 false
#[test]
fn registry_fail_session() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "find info"));

    registry.fail("s1", "provider error".to_string());

    let snap = registry.get_status("s1").unwrap();
    assert_eq!(snap.status, SubAgentStatus::Failed);
    assert!(!snap.result.unwrap().success);
}

/// 测试终止正在运行的会话
///
/// 验证：
/// - 能够成功终止运行中的会话
/// - 终止后状态更新为 `Killed`
/// - 结果中包含 "killed" 关键字的错误信息
#[test]
fn registry_kill_running_session() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "find info"));

    assert!(registry.kill("s1"));

    let snap = registry.get_status("s1").unwrap();
    assert_eq!(snap.status, SubAgentStatus::Killed);
    assert!(snap.result.unwrap().error.as_deref().unwrap().contains("killed"));
}

/// 测试终止非运行状态的会话返回 false
///
/// 验证：
/// - 对已完成的会话调用 `kill` 返回 false
/// - 不能终止非运行状态的会话
#[test]
fn registry_kill_non_running_returns_false() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "find info"));
    registry.complete("s1", ToolResult { success: true, output: "done".to_string(), error: None });

    assert!(!registry.kill("s1"));
}

/// 测试终止不存在的会话返回 false
///
/// 验证：
/// - 对不存在的会话 ID 调用 `kill` 返回 false
#[test]
fn registry_kill_unknown_returns_false() {
    let registry = SubAgentRegistry::new();
    assert!(!registry.kill("nonexistent"));
}

/// 测试按状态过滤会话列表
///
/// 验证：
/// - `list("running")` 只返回运行中的会话
/// - `list("completed")` 只返回已完成的会话
/// - 过滤结果正确匹配对应状态
#[test]
fn registry_list_filters_by_status() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "task1"));
    registry.insert(make_session("s2", "coder", "task2"));

    registry.complete("s1", ToolResult { success: true, output: "done".to_string(), error: None });

    let running = registry.list(Some("running"));
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].session_id, "s2");

    let completed = registry.list(Some("completed"));
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].session_id, "s1");
}

/// 测试查询不存在会话的状态
///
/// 验证：
/// - 查询不存在的会话 ID 返回 `None`
#[test]
fn registry_get_status_unknown() {
    let registry = SubAgentRegistry::new();
    assert!(registry.get_status("nonexistent").is_none());
}

/// 测试会话存在性检查
///
/// 验证：
/// - `exists` 对存在的会话返回 true
/// - `exists` 对不存在的会话返回 false
#[test]
fn registry_exists() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "task"));
    assert!(registry.exists("s1"));
    assert!(!registry.exists("nonexistent"));
}

/// 测试更新标题和元数据会同步写入快照。
#[test]
fn registry_update_metadata() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "task"));

    assert!(registry.update_metadata(
        "s1",
        Some(Some("更友好的标题".to_string())),
        Some(serde_json::json!({ "priority": "high" })),
    ));

    let snap = registry.get_status("s1").unwrap();
    assert_eq!(snap.title.as_deref(), Some("更友好的标题"));
    assert_eq!(snap.metadata["priority"], "high");
}

/// 测试运行中会话计数功能
///
/// 验证：
/// - 初始状态下计数为 0
/// - 插入会话后计数增加
/// - 会话完成后计数减少
#[test]
fn registry_running_count() {
    let registry = SubAgentRegistry::new();
    assert_eq!(registry.running_count(), 0);

    registry.insert(make_session("s1", "a", "t1"));
    registry.insert(make_session("s2", "b", "t2"));
    assert_eq!(registry.running_count(), 2);

    registry.complete("s1", ToolResult { success: true, output: "done".to_string(), error: None });
    assert_eq!(registry.running_count(), 1);
}

/// 测试过期会话的自动清理
///
/// 验证：
/// - 调用 `list` 时自动清理超过 `SESSION_MAX_AGE_SECS` 的已结束会话
/// - 最近完成的会话保留
/// - 仅清理已结束（非运行中）的过期会话
#[test]
fn registry_cleanup_old_sessions() {
    let registry = SubAgentRegistry::new();

    // 插入一个已完成的会话，设置完成时间为超过最大保留时长
    let mut session = make_session("old", "agent", "task");
    session.status = SubAgentStatus::Completed;
    session.completed_at = Some(Utc::now() - chrono::Duration::seconds(SESSION_MAX_AGE_SECS + 1));
    session.result =
        Some(ToolResult { success: true, output: "old result".to_string(), error: None });
    registry.insert(session);

    // 插入一个最近完成的会话
    registry.insert(make_session("recent", "agent", "task"));
    registry.complete(
        "recent",
        ToolResult { success: true, output: "recent result".to_string(), error: None },
    );

    // 调用 list 触发清理机制
    let all = registry.list(Some("all"));
    // 旧会话应被清理，最近会话应保留
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].session_id, "recent");
}

/// 测试短任务描述不被截断
///
/// 验证：
/// - 长度小于限制的任务描述保持原样
#[test]
fn truncate_task_short() {
    assert_eq!(truncate_task("short", 100), "short");
}

/// 测试长任务描述的截断
///
/// 验证：
/// - 超过限制长度的任务描述被截断
/// - 截断后以 "..." 结尾
/// - 截断后总字符数正确（限制长度 + 3 个点）
#[test]
fn truncate_task_long() {
    let long = "a".repeat(150);
    let truncated = truncate_task(&long, 100);
    assert!(truncated.ends_with("..."));
    assert_eq!(truncated.chars().count(), 103); // 100 字符 + "..."
}

/// 测试多字节字符（如 emoji）的安全截断
///
/// 验证：
/// - 多字节字符不会在截断时被切断
/// - 按字符数而非字节数进行截断
/// - 10 个 emoji（每个 4 字节）截断为 5 个字符 + "..."
#[test]
fn truncate_task_multibyte_safe() {
    // 每个 emoji 占 4 字节，10 个 emoji = 40 字节但只有 10 个字符
    let emojis = "🦀".repeat(10);
    let truncated = truncate_task(&emojis, 5);
    assert!(truncated.ends_with("..."));
    assert_eq!(truncated.chars().count(), 8); // 5 个 emoji + "..."
}

/// 测试状态枚举的字符串显示
///
/// 验证：
/// - `as_str()` 方法返回正确的字符串表示
/// - `Display` trait 实现正确
/// - 各状态对应的字符串：running、completed、failed、killed
#[test]
fn status_display() {
    assert_eq!(SubAgentStatus::Running.as_str(), "running");
    assert_eq!(SubAgentStatus::Completed.as_str(), "completed");
    assert_eq!(SubAgentStatus::Failed.as_str(), "failed");
    assert_eq!(SubAgentStatus::Killed.as_str(), "killed");
    assert_eq!(format!("{}", SubAgentStatus::Running), "running");
}

/// 测试注册表的默认构造
///
/// 验证：
/// - `default()` 创建空注册表
/// - 初始列表为空
#[test]
fn registry_default() {
    let registry = SubAgentRegistry::default();
    assert_eq!(registry.list(None).len(), 0);
}

/// 测试并发插入和列表操作
///
/// 验证：
/// - 多线程并发插入会话不会导致数据竞争
/// - 所有插入的会话都能正确保存
/// - `SubAgentRegistry` 的线程安全性
#[test]
fn concurrent_insert_and_list() {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(SubAgentRegistry::new());
    let mut handles = Vec::new();

    // 启动 10 个线程并发插入会话
    for i in 0..10 {
        let reg = registry.clone();
        handles.push(thread::spawn(move || {
            reg.insert(make_session(&format!("s{i}"), "agent", &format!("task {i}")));
        }));
    }

    // 等待所有线程完成
    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(registry.list(Some("all")).len(), 10);
}

/// 测试会话信息的 JSON 序列化
///
/// 验证：
/// - `SubAgentSessionInfo` 能够正确序列化为 JSON
/// - 序列化结果包含所有必要字段
/// - 字段值正确映射到 JSON 结构中
#[test]
fn session_info_serialization() {
    let info = SubAgentSessionInfo {
        session_id: "test-id".to_string(),
        agent: "researcher".to_string(),
        title: None,
        task: "find info".to_string(),
        metadata: serde_json::Value::Object(Default::default()),
        status: "running".to_string(),
        started_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        completed_at: None,
        duration_ms: None,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("test-id"));
    assert!(json.contains("researcher"));
}

#[test]
fn registry_update_metadata_trims_and_clears_blank_title() {
    let registry = SubAgentRegistry::new();
    registry.insert(make_session("s1", "researcher", "task"));

    assert!(registry.update_metadata("s1", Some(Some("  report  ".to_string())), None));
    assert_eq!(registry.get_status("s1").unwrap().title.as_deref(), Some("report"));

    assert!(registry.update_metadata("s1", Some(Some("   ".to_string())), None));
    assert_eq!(registry.get_status("s1").unwrap().title, None);
    assert!(!registry.update_metadata("missing", None, None));
}

#[test]
fn registry_list_retains_terminal_session_without_completion_timestamp() {
    let registry = SubAgentRegistry::new();
    let mut session = make_session("failed-without-time", "researcher", "task");
    session.status = SubAgentStatus::Failed;
    registry.insert(session);

    let failed = registry.list(Some("failed"));

    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].session_id, "failed-without-time");
    assert_eq!(failed[0].completed_at, None);
    assert_eq!(failed[0].duration_ms, None);
}
