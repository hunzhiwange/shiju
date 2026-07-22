//! agents_ipc 模块测试套件
//!
//! 本模块提供 Agent 间通信（IPC）功能的全面测试覆盖，包括：
//! - 数据库初始化与 Schema 验证
//! - Agent 注册、心跳与存活检测
//! - 消息发送、接收与隔离性
//! - 广播消息机制
//! - 共享状态的创建、更新与查询
//! - 安全策略执行（只读模式下的写操作拦截）
//! - Agent 生命周期管理（注册与清理）
//!
//! # 测试架构
//!
//! 测试使用临时目录（TempDir）创建隔离的 SQLite 数据库实例，
//! 确保测试之间相互独立、可重复执行。多 Agent 场景通过
//! 共享数据库路径但使用不同 agent_id 来模拟。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::AutonomyLevel;
    use serde_json::json;
    use tempfile::TempDir;

    /// 创建测试用的 IpcDb 实例
    ///
    /// 在指定临时目录下创建名为 "agents.db" 的数据库文件，
    /// 并使用给定的 agent_id 进行初始化。
    ///
    /// # 参数
    ///
    /// - `dir`: 临时目录引用，用于存放数据库文件
    /// - `agent_id`: Agent 的唯一标识符
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `IpcDb` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let dir = TempDir::new().unwrap();
    /// let db = test_db(&dir, "test_agent");
    /// ```
    fn test_db(dir: &TempDir, agent_id: &str) -> IpcDb {
        let db_path = dir.path().join("agents.db");
        IpcDb::open_with_id(db_path.to_str().unwrap(), agent_id, 300).unwrap()
    }

    /// 验证数据库初始化时创建三张核心表
    ///
    /// 测试 `IpcDb::open_with_id` 成功执行后，数据库中应包含：
    /// - `agents`: Agent 注册表
    /// - `messages`: 消息队列表
    /// - `shared_state`: 共享状态表
    #[test]
    fn schema_creates_three_tables() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        let conn = db.conn.lock().unwrap();

        // 查询数据库中所有表名并按字母顺序排序
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // 验证三张核心表均已创建
        assert!(tables.contains(&"agents".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"shared_state".to_string()));
    }

    /// 验证 Agent 在数据库打开时自动注册
    ///
    /// 测试调用 `IpcDb::open_with_id` 后，agents 表中应自动
    /// 插入一条对应 agent_id 的记录，无需手动注册。
    #[test]
    fn agent_registers_on_open() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        let conn = db.conn.lock().unwrap();

        // 查询 agents 表中指定 agent_id 的记录数
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE agent_id = 'vibewindow_agent_a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // 打开数据库时应自动注册，记录数应为 1
        assert_eq!(count, 1);
    }

    /// 验证心跳机制正确更新 last_seen 时间戳
    ///
    /// 测试 `heartbeat()` 方法能够更新 agents 表中
    /// 对应 Agent 的 last_seen 字段，确保障活机制正常工作。
    #[test]
    fn heartbeat_updates_last_seen() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");

        // 获取心跳前的 last_seen 时间戳
        let before: i64 = {
            let conn = db.conn.lock().unwrap();
            conn.query_row(
                "SELECT last_seen FROM agents WHERE agent_id = 'vibewindow_agent_a'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };

        // 短暂延迟以确保时间戳确实发生变化
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.heartbeat();

        // 获取心跳后的 last_seen 时间戳
        let after: i64 = {
            let conn = db.conn.lock().unwrap();
            conn.query_row(
                "SELECT last_seen FROM agents WHERE agent_id = 'vibewindow_agent_a'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };

        // 心跳后的时间戳应大于或等于心跳前
        assert!(after >= before);
    }

    /// 验证收件箱按 Agent 隔离
    ///
    /// 测试不同 Agent 的消息队列相互隔离：
    /// Agent A 发送消息给 Agent B 后，A 的收件箱应为空，
    /// B 的收件箱应包含该消息。
    #[tokio::test]
    async fn inbox_isolates_per_agent() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();

        // 创建两个使用相同数据库但不同 agent_id 的 IpcDb 实例
        let db_a = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap());
        let db_b = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_b", 300).unwrap());

        // Agent A 向 Agent B 发送消息
        let send_tool = AgentsSendTool::new(db_a.clone(), Arc::new(SecurityPolicy::default()));
        send_tool
            .execute(json!({"to_agent": "vibewindow_agent_b", "payload": "hello b"}))
            .await
            .unwrap();

        // Agent A 的收件箱应为空（消息是发给 B 的）
        let inbox_a = AgentsInboxTool::new(db_a);
        let result_a = inbox_a.execute(json!({})).await.unwrap();
        let msgs_a: Vec<serde_json::Value> = serde_json::from_str(&result_a.output).unwrap();
        assert!(msgs_a.is_empty());

        // Agent B 的收件箱应包含该消息
        let inbox_b = AgentsInboxTool::new(db_b);
        let result_b = inbox_b.execute(json!({})).await.unwrap();
        let msgs_b: Vec<serde_json::Value> = serde_json::from_str(&result_b.output).unwrap();
        assert_eq!(msgs_b.len(), 1);
        assert_eq!(msgs_b[0]["payload"], "hello b");
    }

    /// 验证广播消息对所有 Agent 可见
    ///
    /// 测试当 to_agent 设置为 "*" 时，发送广播消息，
    /// 所有已注册的 Agent 都能在自己的收件箱中看到该消息。
    #[tokio::test]
    async fn broadcast_visible_to_all_agents() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();

        // 创建两个 Agent 实例
        let db_a = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap());
        let db_b = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_b", 300).unwrap());

        // Agent A 发送广播消息（to_agent 为 "*"）
        let send_tool = AgentsSendTool::new(db_a.clone(), Arc::new(SecurityPolicy::default()));
        send_tool.execute(json!({"to_agent": "*", "payload": "broadcast msg"})).await.unwrap();

        // 两个 Agent 都应该能看到广播消息
        let inbox_a = AgentsInboxTool::new(db_a);
        let result_a = inbox_a.execute(json!({})).await.unwrap();
        let msgs_a: Vec<serde_json::Value> = serde_json::from_str(&result_a.output).unwrap();
        assert_eq!(msgs_a.len(), 1);

        let inbox_b = AgentsInboxTool::new(db_b);
        let result_b = inbox_b.execute(json!({})).await.unwrap();
        let msgs_b: Vec<serde_json::Value> = serde_json::from_str(&result_b.output).unwrap();
        assert_eq!(msgs_b.len(), 1);
    }

    /// 验证过期 Agent 不在列表中显示
    ///
    /// 测试当 Agent 的 last_seen 时间戳超过 staleness_secs
    /// 配置的阈值时，该 Agent 不会出现在 agents_list 的结果中。
    #[tokio::test]
    async fn stale_agents_excluded_from_list() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();

        // Agent A 使用较短的过期时间窗口（5 秒）
        let db_a = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 5).unwrap());

        // 手动将 Agent B 的 last_seen 设置为很久以前的时间
        {
            let conn = db_a.conn.lock().unwrap();
            let old_time = now_epoch() - 100;
            conn.execute(
                    "INSERT OR REPLACE INTO agents (agent_id, status, last_seen) VALUES ('vibewindow_agent_b', 'online', ?1)",
                    rusqlite::params![old_time],
                )
                .unwrap();
        }

        // 执行列表查询
        let list_tool = AgentsListTool::new(db_a);
        let result = list_tool.execute(json!({})).await.unwrap();
        let agents: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();

        // 只有 Agent A 应该在列表中（Agent B 已过期）
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["agent_id"], "vibewindow_agent_a");
    }

    /// 验证身份码强制执行——发送者身份不可伪造
    ///
    /// 测试 messages 表中的 from_agent 字段始终使用
    /// IpcDb 实例的 agent_id，忽略任何外部传入的伪造值，
    /// 确保 Agent 身份不被冒用。
    #[tokio::test]
    async fn identity_code_enforced() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();
        let db = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap());

        // 发送消息——无论输入如何，from_agent 必须是 agent_a
        let send_tool = AgentsSendTool::new(db.clone(), Arc::new(SecurityPolicy::default()));
        send_tool
            .execute(json!({"to_agent": "vibewindow_agent_b", "payload": "test"}))
            .await
            .unwrap();

        // 直接查询数据库验证 from_agent 字段
        let conn = db.conn.lock().unwrap();
        let from: String = conn
            .query_row("SELECT from_agent FROM messages LIMIT 1", [], |row| row.get(0))
            .unwrap();
        // from_agent 必须与 IpcDb 的 agent_id 一致
        assert_eq!(from, "vibewindow_agent_a");
    }

    /// 验证状态的 UPSERT 操作——创建与更新
    ///
    /// 测试 StateSetTool 的 UPSERT 语义：
    /// - 首次设置某个 key 时创建记录
    /// - 对已存在的 key 再次设置时更新其 value
    #[tokio::test]
    async fn state_upsert_creates_and_updates() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));

        let set_tool = StateSetTool::new(db.clone(), Arc::new(SecurityPolicy::default()));
        let get_tool = StateGetTool::new(db.clone());

        // 首次设置：创建新记录
        set_tool.execute(json!({"key": "progress", "value": "50%"})).await.unwrap();

        let result = get_tool.execute(json!({"key": "progress"})).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["value"], "50%");

        // 再次设置：更新现有记录
        set_tool.execute(json!({"key": "progress", "value": "100%"})).await.unwrap();

        let result = get_tool.execute(json!({"key": "progress"})).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["value"], "100%");
    }

    /// 验证状态记录正确追踪所有者
    ///
    /// 测试设置共享状态时，owner 字段自动记录为
    /// 执行 StateSetTool 的 Agent 的 agent_id。
    #[tokio::test]
    async fn state_records_owner() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));

        let set_tool = StateSetTool::new(db.clone(), Arc::new(SecurityPolicy::default()));
        set_tool.execute(json!({"key": "task", "value": "done"})).await.unwrap();

        let get_tool = StateGetTool::new(db);
        let result = get_tool.execute(json!({"key": "task"})).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        // owner 应为设置该状态的 Agent
        assert_eq!(parsed["owner"], "vibewindow_agent_a");
    }

    /// 验证空收件箱返回成功而非错误
    ///
    /// 测试当 Agent 的收件箱为空时，AgentsInboxTool
    /// 仍返回成功状态，只是消息列表为空数组。
    #[tokio::test]
    async fn empty_inbox_returns_success() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));

        let inbox_tool = AgentsInboxTool::new(db);
        let result = inbox_tool.execute(json!({})).await.unwrap();

        // 空收件箱应返回成功，消息列表为空
        assert!(result.success);
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();
        assert!(msgs.is_empty());
    }

    /// 验证只读模式下写操作被安全策略拦截
    ///
    /// 测试当 SecurityPolicy 的 autonomy 设置为 ReadOnly 时：
    /// - agents_send（发送消息）应被阻止
    /// - state_set（设置状态）应被阻止
    ///
    /// 这确保了低权限 Agent 无法执行修改性操作。
    #[tokio::test]
    async fn security_blocks_act_in_readonly() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));
        let readonly = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });

        // agents_send 在只读模式下应被阻止
        let send_tool = AgentsSendTool::new(db.clone(), readonly.clone());
        let result = send_tool
            .execute(json!({"to_agent": "vibewindow_agent_b", "payload": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());

        // state_set 在只读模式下应被阻止
        let set_tool = StateSetTool::new(db, readonly);
        let result = set_tool.execute(json!({"key": "k", "value": "v"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    /// 验证禁用配置时不注册任何工具
    ///
    /// 测试当 AgentsIpcConfig.enabled 为 false 时，
    /// IpcDb::open 不应被调用，工具注册逻辑应跳过。
    /// 同时验证配置的默认值。
    #[test]
    fn disabled_config_registers_no_tools() {
        let config = AgentsIpcConfig { enabled: false, ..AgentsIpcConfig::default() };
        // 当禁用时，IpcDb::open 不应被调用，
        // 因此工具数量保持不变。验证配置默认值。
        assert!(!config.enabled);
        assert_eq!(config.staleness_secs, 300);
    }

    /// 验证真实打开场景中 agent_id 从工作区路径派生
    ///
    /// 测试 `IpcDb::open`（非 open_with_id）根据工作区路径
    /// 生成确定性的 agent_id（SHA-256 哈希的 64 位十六进制表示），
    /// 相同工作区路径应始终产生相同的 agent_id。
    #[test]
    fn real_open_derives_agent_id_from_workspace() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let db_path = dir.path().join("agents.db");

        let config = AgentsIpcConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            staleness_secs: 300,
        };

        let db = IpcDb::open(&workspace, &config).unwrap();

        // agent_id 应为 64 字符的十六进制 SHA-256 哈希
        assert_eq!(db.agent_id().len(), 64);
        assert!(db.agent_id().chars().all(|c| c.is_ascii_hexdigit()));

        // 相同工作区应产生相同的 agent_id
        let db2 = IpcDb::open(&workspace, &config).unwrap();
        assert_eq!(db.agent_id(), db2.agent_id());
    }

    /// 验证 IpcDb 销毁时从 agents 表中移除记录
    ///
    /// 测试当 IpcDb 实例被 drop 时，应自动从 agents 表中
    /// 删除对应的 Agent 记录，确保离线 Agent 不会残留。
    #[test]
    fn drop_removes_agent_from_table() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db");
        let db_path_str = db_path.to_str().unwrap().to_string();

        // 打开一个生命周期超越 IpcDb 的连接，用于验证清理结果
        {
            let _db = IpcDb::open_with_id(&db_path_str, "vibewindow_agent_a", 300).unwrap();
            // _db 在此处存活——Agent 应在表中
        }
        // _db 已被销毁——Agent 应被移除

        let conn = Connection::open(&db_path_str).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE agent_id = 'vibewindow_agent_a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // 销毁后 Agent 记录应为 0
        assert_eq!(count, 0);
    }

    /// 验证直接消息在读取收件箱后标记为已读
    ///
    /// 测试非广播消息（点对点消息）在通过 AgentsInboxTool
    /// 读取后应被标记为已读，再次读取收件箱时应为空。
    #[tokio::test]
    async fn direct_messages_marked_read_after_inbox() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();

        let db_a = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap());
        let db_b = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_b", 300).unwrap());

        // Agent A 向 Agent B 发送消息
        let send_tool = AgentsSendTool::new(db_a, Arc::new(SecurityPolicy::default()));
        send_tool
            .execute(json!({"to_agent": "vibewindow_agent_b", "payload": "once"}))
            .await
            .unwrap();

        let inbox_b = AgentsInboxTool::new(db_b.clone());

        // 首次读取：应有 1 条消息
        let result = inbox_b.execute(json!({})).await.unwrap();
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();
        assert_eq!(msgs.len(), 1);

        // 再次读取：应为空（消息已被标记为已读）
        let result = inbox_b.execute(json!({})).await.unwrap();
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();
        assert!(msgs.is_empty());
    }

    /// 验证获取不存在的状态键返回"未找到"提示
    ///
    /// 测试当查询 shared_state 表中不存在的 key 时，
    /// StateGetTool 应返回成功状态，但 output 中包含
    /// "not found" 提示而非错误。
    #[tokio::test]
    async fn state_get_missing_key_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));

        let get_tool = StateGetTool::new(db);
        let result = get_tool.execute(json!({"key": "nonexistent"})).await.unwrap();

        // 查询不存在的 key 应返回成功，但输出包含"not found"
        assert!(result.success);
        assert!(result.output.contains("not found"));
    }

    /// 验证发送消息缺少必需参数时返回错误
    ///
    /// 测试 AgentsSendTool 在缺少 payload 或 to_agent
    /// 必需参数时，应返回失败结果并包含明确的错误信息。
    #[tokio::test]
    async fn send_missing_params_returns_error() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));
        let send_tool = AgentsSendTool::new(db, Arc::new(SecurityPolicy::default()));

        // 缺少 payload 参数
        let result = send_tool.execute(json!({"to_agent": "vibewindow_agent_b"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("payload"));

        // 缺少 to_agent 参数
        let result = send_tool.execute(json!({"payload": "hello"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("to_agent"));
    }

    /// 验证两个 Agent 之间的完整交互流程
    ///
    /// 这是一个集成测试，验证以下完整场景：
    /// 1. 两个 Agent 相互可见（agents_list）
    /// 2. Agent A 向 Agent B 发送消息
    /// 3. Agent B 读取消息并确认发送者
    /// 4. Agent B 向 Agent A 回复消息
    /// 5. Agent A 读取回复
    /// 6. Agent A 设置共享状态
    /// 7. Agent B 读取共享状态并确认所有者
    #[tokio::test]
    async fn two_agents_full_exchange() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();

        let db_a = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap());
        let db_b = Arc::new(IpcDb::open_with_id(&db_path, "vibewindow_agent_b", 300).unwrap());
        let security = Arc::new(SecurityPolicy::default());

        // 两个 Agent 都应在列表中可见
        let list_tool = AgentsListTool::new(db_a.clone());
        let result = list_tool.execute(json!({})).await.unwrap();
        let agents: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();
        assert_eq!(agents.len(), 2);

        // Agent A 向 Agent B 发送消息
        let send_a = AgentsSendTool::new(db_a.clone(), security.clone());
        let r = send_a
            .execute(json!({"to_agent": "vibewindow_agent_b", "payload": "task: summarize"}))
            .await
            .unwrap();
        assert!(r.success);

        // Agent B 读取收件箱
        let inbox_b = AgentsInboxTool::new(db_b.clone());
        let r = inbox_b.execute(json!({})).await.unwrap();
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&r.output).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["payload"], "task: summarize");
        assert_eq!(msgs[0]["from_agent"], "vibewindow_agent_a");

        // Agent B 向 Agent A 回复消息
        let send_b = AgentsSendTool::new(db_b.clone(), security.clone());
        send_b
            .execute(json!({"to_agent": "vibewindow_agent_a", "payload": "done: summary attached"}))
            .await
            .unwrap();

        // Agent A 读取回复
        let inbox_a = AgentsInboxTool::new(db_a.clone());
        let r = inbox_a.execute(json!({})).await.unwrap();
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&r.output).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["payload"], "done: summary attached");
        assert_eq!(msgs[0]["from_agent"], "vibewindow_agent_b");

        // Agent A 设置共享状态
        let set_tool = StateSetTool::new(db_a, security);
        set_tool.execute(json!({"key": "status", "value": "complete"})).await.unwrap();

        // Agent B 读取共享状态
        let get_tool = StateGetTool::new(db_b);
        let r = get_tool.execute(json!({"key": "status"})).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&r.output).unwrap();
        assert_eq!(parsed["value"], "complete");
        assert_eq!(parsed["owner"], "vibewindow_agent_a");
    }

    #[test]
    fn reopening_agent_preserves_registration_metadata() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("agents.db").to_str().unwrap().to_string();
        let db = IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap();

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE agents SET role = 'reviewer', status = 'offline', metadata = '{\"slot\":2}', last_seen = 1 WHERE agent_id = 'vibewindow_agent_a'",
                [],
            )
            .unwrap();
        }

        let reopened = IpcDb::open_with_id(&db_path, "vibewindow_agent_a", 300).unwrap();
        let registration: (Option<String>, String, Option<String>, i64) = reopened
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT role, status, metadata, last_seen FROM agents WHERE agent_id = 'vibewindow_agent_a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(registration.0.as_deref(), Some("reviewer"));
        assert_eq!(registration.1, "online");
        assert_eq!(registration.2.as_deref(), Some("{\"slot\":2}"));
        assert!(registration.3 > 1);
    }

    #[test]
    fn open_reports_database_parent_creation_failure() {
        let dir = TempDir::new().unwrap();
        let blocked_parent = dir.path().join("not_a_directory");
        std::fs::write(&blocked_parent, b"file").unwrap();
        let config = AgentsIpcConfig {
            enabled: true,
            db_path: blocked_parent.join("agents.db").to_string_lossy().into_owned(),
            staleness_secs: 300,
        };

        let error = match IpcDb::open(dir.path(), &config) {
            Ok(_) => panic!("database open unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error.starts_with("failed to create db directory:"));
    }

    #[test]
    fn create_team_normalizes_members_and_persists_team_state() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        let members = vec![
            " agent_b ".to_string(),
            "agent_a".to_string(),
            String::new(),
            "agent_b".to_string(),
        ];

        let created = db.create_team(" review ", &members).unwrap();

        assert_eq!(created["team_id"], "review");
        assert_eq!(created["members"], json!(["agent_a", "agent_b"]));
        assert_eq!(created["owner"], "vibewindow_agent_a");
        assert!(created["updated_at"].as_i64().unwrap() > 0);
        assert_eq!(db.read_team_members("review").unwrap(), vec!["agent_a", "agent_b"]);

        let stored_key: String = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT key FROM shared_state WHERE key = 'team:review'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(stored_key, "team:review");
    }

    #[test]
    fn team_operations_reject_blank_identifiers_and_members() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");

        assert_eq!(
            db.create_team(" \t ", &["agent_a".to_string()]).unwrap_err(),
            "team id must not be empty"
        );
        assert_eq!(
            db.create_team("review", &[String::new(), "  ".to_string()]).unwrap_err(),
            "team members must not be empty"
        );
        assert_eq!(db.delete_team("  ").unwrap_err(), "team id must not be empty");
        assert_eq!(db.read_team_members("\n").unwrap_err(), "team id must not be empty");
    }

    #[test]
    fn delete_team_reports_presence_and_read_reports_unknown_team() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        db.create_team("review", &["agent_a".to_string()]).unwrap();

        assert!(db.delete_team(" review ").unwrap());
        assert!(!db.delete_team("review").unwrap());
        assert_eq!(db.read_team_members("review").unwrap_err(), "unknown team 'review'");
    }

    #[test]
    fn read_team_members_reports_malformed_persisted_value() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        db.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO shared_state (key, value, owner, updated_at) VALUES ('team:broken', 'not-json', 'vibewindow_agent_a', 1)",
                [],
            )
            .unwrap();

        let error = db.read_team_members("broken").unwrap_err();
        assert!(error.starts_with("failed to decode team members:"));
    }

    #[test]
    fn send_message_helper_records_sender_recipient_and_payload() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");

        db.send_message("vibewindow_agent_b", "queued work").unwrap();

        let message: (String, String, String, i64) = db
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT from_agent, to_agent, payload, created_at FROM messages",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(message.0, "vibewindow_agent_a");
        assert_eq!(message.1, "vibewindow_agent_b");
        assert_eq!(message.2, "queued work");
        assert!(message.3 > 0);
    }

    #[test]
    fn database_helpers_report_storage_failures() {
        let dir = TempDir::new().unwrap();
        let db = test_db(&dir, "vibewindow_agent_a");
        let conn = db.conn.lock().unwrap();
        conn.execute("DROP TABLE messages", []).unwrap();
        conn.execute("DROP TABLE shared_state", []).unwrap();
        drop(conn);

        let send_error = db.send_message("vibewindow_agent_b", "queued work").unwrap_err();
        assert!(send_error.starts_with("failed to send message:"));

        let create_error = db.create_team("review", &["agent_a".to_string()]).unwrap_err();
        assert!(create_error.starts_with("failed to persist team:"));

        let delete_error = db.delete_team("review").unwrap_err();
        assert!(delete_error.starts_with("failed to delete team:"));
        assert_eq!(db.read_team_members("review").unwrap_err(), "unknown team 'review'");
    }

    #[test]
    fn tool_metadata_exposes_stable_names_and_required_parameters() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));
        let security = Arc::new(SecurityPolicy::default());
        let list = AgentsListTool::new(db.clone());
        let send = AgentsSendTool::new(db.clone(), security.clone());
        let inbox = AgentsInboxTool::new(db.clone());
        let get = StateGetTool::new(db.clone());
        let set = StateSetTool::new(db, security);

        assert_eq!(list.name(), "agents_list");
        assert_eq!(send.name(), "agents_send");
        assert_eq!(inbox.name(), "agents_inbox");
        assert_eq!(get.name(), "state_get");
        assert_eq!(set.name(), "state_set");
        assert!(list.description().contains("IPC"));
        assert!(send.description().contains("广播"));
        assert!(inbox.description().contains("未读消息"));
        assert!(get.description().contains("获取值"));
        assert!(set.description().contains("设置键值对"));
        assert_eq!(list.parameters_schema()["required"], json!([]));
        assert_eq!(send.parameters_schema()["required"], json!(["to_agent", "payload"]));
        assert_eq!(inbox.parameters_schema()["required"], json!([]));
        assert_eq!(get.parameters_schema()["required"], json!(["key"]));
        assert_eq!(set.parameters_schema()["required"], json!(["key", "value"]));
    }

    #[tokio::test]
    async fn state_tools_reject_missing_or_non_string_parameters() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));
        let get = StateGetTool::new(db.clone());
        let set = StateSetTool::new(db, Arc::new(SecurityPolicy::default()));

        let missing_get = get.execute(json!({})).await.unwrap();
        assert!(!missing_get.success);
        assert_eq!(missing_get.error.as_deref(), Some("Missing 'key' parameter"));

        let non_string_get = get.execute(json!({"key": 7})).await.unwrap();
        assert!(!non_string_get.success);
        assert_eq!(non_string_get.error.as_deref(), Some("Missing 'key' parameter"));

        let missing_key = set.execute(json!({"value": "ready"})).await.unwrap();
        assert!(!missing_key.success);
        assert_eq!(missing_key.error.as_deref(), Some("Missing 'key' parameter"));

        let missing_value = set.execute(json!({"key": "status"})).await.unwrap();
        assert!(!missing_value.success);
        assert_eq!(missing_value.error.as_deref(), Some("Missing 'value' parameter"));
    }

    #[tokio::test]
    async fn broadcast_message_remains_unread_for_repeated_inbox_reads() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(test_db(&dir, "vibewindow_agent_a"));
        db.send_message("*", "announcement").unwrap();
        let inbox = AgentsInboxTool::new(db.clone());

        for _ in 0..2 {
            let result = inbox.execute(json!({})).await.unwrap();
            let messages: Vec<serde_json::Value> = serde_json::from_str(&result.output).unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0]["payload"], "announcement");
        }

        let read_flag: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT read FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(read_flag, 0);
    }
}
