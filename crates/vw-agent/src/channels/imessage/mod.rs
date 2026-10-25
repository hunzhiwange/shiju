//! iMessage 通道模块
//!
//! 本模块实现了基于 macOS iMessage 的消息通道，通过 AppleScript 桥接与系统 Messages 应用集成。
//!
//! # 核心功能
//!
//! - **消息监听**：轮询 macOS Messages 数据库 (`~/Library/Messages/chat.db`) 以检测新消息
//! - **消息发送**：通过 `osascript` 调用 AppleScript 发送回复消息
//! - **联系人过滤**：支持白名单机制，仅处理允许的联系人消息
//!
//! # 安全特性
//!
//! - **输入验证**：严格验证目标地址格式（电话号码或邮箱）
//! - **注入防护**：对 AppleScript 特殊字符进行转义，防止代码注入攻击 (CWE-78)
//! - **SQL 安全**：使用参数化查询，防止 SQL 注入 (CWE-89)
//! - **最小权限**：以只读模式访问数据库
//!
//! # 平台限制
//!
//! - 仅支持 macOS（需要 Full Disk Access 权限）
//! - 不支持 WASM 目标架构
//!
//! # 示例
//!
//! ```rust,no_run
//! use vibe_window::app::agent::channels::imessage::IMessageChannel;
//! use vibe_window::app::agent::channels::traits::Channel;
//!
//! // 创建允许的联系人列表（"*" 表示允许所有联系人）
//! let allowed_contacts = vec![
//!     "+8613800138000".to_string(),
//!     "user@example.com".to_string(),
//! ];
//!
//! let channel = IMessageChannel::new(allowed_contacts);
//! println!("Channel name: {}", channel.name());
//! ```

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// iMessage 通道实现
///
/// 通过 macOS 的 AppleScript 桥接与 iMessage 集成，实现消息的发送与接收。
///
/// # 工作原理
///
/// 1. **接收消息**：定期轮询 Messages SQLite 数据库，查询新消息记录
/// 2. **发送消息**：构造 AppleScript 脚本，通过 `osascript` 命令执行发送
///
/// # 安全注意事项
///
/// - 所有用户输入在传递给 AppleScript 前都会经过严格转义
/// - 目标地址必须符合电话号码（`+` 开头）或邮箱格式
/// - 仅处理来自白名单联系人的消息
#[derive(Clone)]
pub struct IMessageChannel {
    /// 允许的联系人列表
    ///
    /// - 特殊值 `"*"` 表示允许所有联系人
    /// - 支持电话号码格式（如 `+8613800138000`）
    /// - 支持邮箱格式（如 `user@example.com`）
    allowed_contacts: Vec<String>,

    /// 数据库轮询间隔（秒）
    ///
    /// 控制检查新消息的频率。默认值为 3 秒。
    /// 较小的值可以更快响应，但会增加系统负载。
    poll_interval_secs: u64,
}

impl IMessageChannel {
    /// 创建新的 iMessage 通道实例
    ///
    /// # 参数
    ///
    /// - `allowed_contacts`：允许的联系人列表
    ///   - 使用 `"*"` 允许所有联系人
    ///   - 电话号码格式：`+` 后跟数字（如 `+8613800138000`）
    ///   - 邮箱格式：标准邮箱地址（如 `user@example.com`）
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `IMessageChannel` 实例，默认轮询间隔为 3 秒
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibe_window::app::agent::channels::imessage::IMessageChannel;
    ///
    /// // 允许所有联系人
    /// let channel_all = IMessageChannel::new(vec!["*".to_string()]);
    ///
    /// // 允许特定联系人
    /// let channel_specific = IMessageChannel::new(vec![
    ///     "+8613800138000".to_string(),
    ///     "user@example.com".to_string(),
    /// ]);
    /// ```
    pub fn new(allowed_contacts: Vec<String>) -> Self {
        Self { allowed_contacts, poll_interval_secs: 3 }
    }

    /// 检查发送者是否在允许的联系人列表中
    ///
    /// # 参数
    ///
    /// - `sender`：消息发送者标识（电话号码或邮箱地址）
    ///
    /// # 返回值
    ///
    /// - `true`：发送者在白名单中或白名单包含 `"*"`
    /// - `false`：发送者不在白名单中
    ///
    /// # 匹配规则
    ///
    /// - 白名单中包含 `"*"` 时，允许所有联系人
    /// - 比较时忽略大小写（适用于邮箱地址）
    fn is_contact_allowed(&self, sender: &str) -> bool {
        // 如果白名单包含通配符 "*"，则允许所有联系人
        if self.allowed_contacts.iter().any(|u| u == "*") {
            return true;
        }
        // 检查发送者是否在白名单中（忽略大小写）
        self.allowed_contacts.iter().any(|u| u.eq_ignore_ascii_case(sender))
    }
}

/// 对字符串进行 AppleScript 安全转义
///
/// 防止 AppleScript 代码注入攻击，通过转义特殊字符确保字符串安全插入到脚本中。
///
/// # 安全说明
///
/// 此函数实现了针对 AppleScript 注入的深度防御措施 (CWE-78: OS Command Injection)。
/// 所有用户控制的输入在插入 AppleScript 字符串前都必须经过此函数处理。
///
/// # 转义规则
///
/// - 反斜杠：`\` → `\\`
/// - 双引号：`"` → `\"`
/// - 换行符：`\n` → `\\n`（防止通过换行注入代码）
/// - 回车符：`\r` → `\\r`（防止通过回车注入代码）
///
/// # 参数
///
/// - `s`：需要转义的原始字符串
///
/// # 返回值
///
/// 返回转义后的安全字符串，可直接插入 AppleScript 双引号字符串中
///
/// # 示例
///
/// ```rust
/// # use vibe_window::app::agent::channels::imessage::escape_applescript;
/// assert_eq!(escape_applescript(r#"hello "world""#), r#"hello \"world\""#);
/// assert_eq!(escape_applescript("line1\nline2"), "line1\\nline2");
/// assert_eq!(escape_applescript(r#"path\to\file"#), r#"path\\to\\file"#);
/// ```
#[cfg(not(target_arch = "wasm32"))]
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

/// 验证 iMessage 目标地址格式
///
/// 作为深度防御措施，在将目标地址传递给 AppleScript 之前验证其格式，
/// 拒绝明显的恶意输入。
///
/// # 验证规则
///
/// ## 电话号码格式
///
/// - 必须以 `+` 开头
/// - 后跟数字，可包含空格或连字符
/// - 数字位数：7-15 位（覆盖国际电话号码范围）
/// - 示例：`+8613800138000`、`+1 234-567-8900`
///
/// ## 邮箱地址格式
///
/// - 必须包含 `@` 符号
/// - 本地部分（@ 前）：非空，字母数字或 `._+-`
/// - 域名部分（@ 后）：非空，包含 `.`，字母数字或 `.-`
/// - 示例：`user@example.com`、`user.name+tag@sub.domain.com`
///
/// # 参数
///
/// - `target`：待验证的目标地址字符串
///
/// # 返回值
///
/// - `true`：格式有效
/// - `false`：格式无效或为空
///
/// # 安全考虑
///
/// 此验证是第一道防线，即使攻击者绕过此检查，后续的 AppleScript 转义
/// 也能提供额外保护。
///
/// # 示例
///
/// ```rust
/// # use vibe_window::app::agent::channels::imessage::is_valid_imessage_target;
/// // 有效的电话号码
/// assert!(is_valid_imessage_target("+8613800138000"));
/// assert!(is_valid_imessage_target("+1 234-567-8900"));
///
/// // 有效的邮箱
/// assert!(is_valid_imessage_target("user@example.com"));
///
/// // 无效的格式
/// assert!(!is_valid_imessage_target("invalid"));
/// assert!(!is_valid_imessage_target("+123")); // 数字太少
/// ```
#[cfg(not(target_arch = "wasm32"))]
fn is_valid_imessage_target(target: &str) -> bool {
    // 去除首尾空白字符
    let target = target.trim();
    if target.is_empty() {
        return false;
    }

    // 验证电话号码格式：+1234567890 或 +1 234-567-8900
    if target.starts_with('+') {
        // 提取纯数字部分
        let digits_only: String = target.chars().filter(char::is_ascii_digit).collect();
        // 验证数字位数：7-15 位（覆盖最短和最长的有效电话号码）
        return digits_only.len() >= 7 && digits_only.len() <= 15;
    }

    // 验证邮箱格式：简单验证（包含 @ 且两侧都有字符）
    if let Some(at_pos) = target.find('@') {
        let local = &target[..at_pos];
        let domain = &target[at_pos + 1..];

        // 本地部分验证：非空，仅包含字母数字或允许的特殊字符
        let local_valid =
            !local.is_empty() && local.chars().all(|c| c.is_alphanumeric() || "._+-".contains(c));

        // 域名验证：非空，包含点号，仅包含字母数字或允许的特殊字符
        let domain_valid = !domain.is_empty()
            && domain.contains('.')
            && domain.chars().all(|c| c.is_alphanumeric() || ".-".contains(c));

        return local_valid && domain_valid;
    }

    false
}

#[cfg(not(target_arch = "wasm32"))]
fn build_send_script(message: &SendMessage) -> anyhow::Result<String> {
    if !is_valid_imessage_target(&message.recipient) {
        anyhow::bail!(
            "Invalid iMessage target: must be a phone number (+1234567890) or email (user@example.com)"
        );
    }

    let escaped_msg = escape_applescript(&message.content);
    let escaped_target = escape_applescript(&message.recipient);

    Ok(format!(
        r#"tell application "Messages"
    set targetService to 1st account whose service type = iMessage
    set targetBuddy to participant "{escaped_target}" of targetService
    send "{escaped_msg}" to targetBuddy
end tell"#
    ))
}

#[cfg(not(target_arch = "wasm32"))]
async fn forward_imessage_messages(
    channel: &IMessageChannel,
    tx: &mpsc::Sender<ChannelMessage>,
    last_rowid: &mut i64,
    messages: Vec<(i64, String, String)>,
) -> bool {
    for (rowid, sender, text) in messages {
        if rowid > *last_rowid {
            *last_rowid = rowid;
        }

        if !channel.is_contact_allowed(&sender) {
            continue;
        }

        if text.trim().is_empty() {
            continue;
        }

        let reply_target = sender.clone();
        let msg = ChannelMessage {
            instance_id: "default".to_string(),
            id: rowid.to_string(),
            sender,
            reply_target,
            content: text,
            channel: "imessage".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            thread_ts: None,
        };

        if tx.send(msg).await.is_err() {
            return false;
        }
    }

    true
}

/// 实现 Channel trait
///
/// 为 IMessageChannel 提供标准的通道接口实现，包括消息发送、监听和健康检查功能。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for IMessageChannel {
    /// 返回通道名称
    ///
    /// # 返回值
    ///
    /// 固定返回 `"imessage"`
    fn name(&self) -> &str {
        "imessage"
    }

    /// 发送消息（WASM 平台实现）
    ///
    /// # 错误
    ///
    /// 在 WASM 架构上始终返回错误，因为 iMessage 不支持 WASM
    #[cfg(target_arch = "wasm32")]
    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        anyhow::bail!("iMessage channel is not supported on WASM")
    }

    /// 发送消息（非 WASM 平台实现）
    ///
    /// 通过 AppleScript 向指定联系人发送 iMessage 消息。
    ///
    /// # 参数
    ///
    /// - `message`：待发送的消息对象，包含收件人和内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：消息发送成功
    /// - `Err(...)`：发送失败（目标地址无效或 osascript 执行失败）
    ///
    /// # 安全措施
    ///
    /// 1. **目标地址验证**：在转义前验证格式，拒绝明显无效的地址
    /// 2. **内容转义**：对消息内容和目标地址进行 AppleScript 转义
    /// 3. **命令执行**：通过 `osascript` 安全执行 AppleScript
    ///
    /// # 错误处理
    ///
    /// - 目标地址格式无效时立即返回错误
    /// - osascript 执行失败时返回标准错误输出
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use vibe_window::app::agent::channels::imessage::IMessageChannel;
    /// use vibe_window::app::agent::channels::traits::{Channel, SendMessage};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let channel = IMessageChannel::new(vec!["*".to_string()]);
    /// let msg = SendMessage {
    ///     recipient: "+8613800138000".to_string(),
    ///     content: "Hello from VibeWindow!".to_string(),
    /// };
    /// channel.send(&msg).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let script = build_send_script(message)?;

        // 通过 osascript 执行 AppleScript
        let output =
            tokio::process::Command::new("osascript").arg("-e").arg(&script).output().await?;

        // 检查执行结果
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("iMessage send failed: {stderr}");
        }

        Ok(())
    }

    /// 监听新消息（WASM 平台实现）
    ///
    /// # 返回值
    ///
    /// 在 WASM 架构上直接返回 `Ok(())`，不执行任何操作
    #[cfg(target_arch = "wasm32")]
    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // WASM 平台不支持 iMessage，直接返回成功
        Ok(())
    }

    /// 监听新消息（非 WASM 平台实现）
    ///
    /// 持续轮询 macOS Messages 数据库，检测新收到的消息并通过通道发送。
    ///
    /// # 参数
    ///
    /// - `tx`：消息发送器，用于将接收到的消息发送到处理流程
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：通道接收器已关闭，正常退出
    /// - `Err(...)`：发生错误（数据库访问失败或轮询工作线程错误）
    ///
    /// # 工作流程
    ///
    /// 1. 定位 Messages 数据库文件 (`~/Library/Messages/chat.db`)
    /// 2. 建立只读数据库连接
    /// 3. 获取初始 ROWID，避免处理历史消息
    /// 4. 循环轮询：每隔 `poll_interval_secs` 秒查询新消息
    /// 5. 过滤：仅处理白名单联系人的非空消息
    /// 6. 发送：将有效消息通过 `tx` 发送到处理流程
    ///
    /// # 权限要求
    ///
    /// - 需要 macOS Full Disk Access 权限才能访问 Messages 数据库
    /// - Messages.app 需要至少运行过一次以创建数据库文件
    ///
    /// # 性能优化
    ///
    /// - 使用持久化数据库连接，避免每次轮询重新建立连接
    /// - 使用 `spawn_blocking` 在单独线程执行数据库操作，避免阻塞异步运行时
    /// - 每次查询限制最多 20 条消息，避免大量历史消息堆积
    ///
    /// # 错误处理
    ///
    /// - 数据库文件不存在时立即返回错误
    /// - 轮询错误时记录警告日志并继续运行
    /// - 通道接收器关闭时正常退出
    #[cfg(not(target_arch = "wasm32"))]
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!("iMessage channel listening (AppleScript bridge)...");

        let db_path = imessage_database_path()?;
        ensure_imessage_database_exists(&db_path)?;
        let conn = open_imessage_connection(&db_path).await?;
        let (mut conn, initial_rowid) = load_initial_rowid(conn).await?;
        let mut last_rowid = initial_rowid;

        // 主轮询循环
        loop {
            // 等待指定的轮询间隔
            tokio::time::sleep(tokio::time::Duration::from_secs(self.poll_interval_secs)).await;

            let since = last_rowid;
            let (returned_conn, poll_result) = poll_imessage_connection(conn, since).await?;
            conn = returned_conn;

            // 处理查询结果
            match poll_result {
                Ok(messages) => {
                    if !forward_imessage_messages(self, &tx, &mut last_rowid, messages).await {
                        return Ok(());
                    }
                }
                Err(e) => {
                    // 记录轮询错误但继续运行
                    tracing::warn!("iMessage poll error: {e}");
                }
            }
        }
    }

    /// 执行通道健康检查
    ///
    /// 验证 iMessage 通道是否可用且配置正确。
    ///
    /// # 返回值
    ///
    /// - `true`：通道健康（macOS 平台且数据库文件存在）
    /// - `false`：通道不可用（非 macOS 平台或数据库文件不存在）
    ///
    /// # 检查条件
    ///
    /// 1. 运行平台必须是 macOS
    /// 2. Messages 数据库文件 (`~/Library/Messages/chat.db`) 必须存在
    ///
    /// # 平台差异
    ///
    /// - **WASM**：始终返回 `false`
    /// - **非 macOS**：始终返回 `false`
    /// - **macOS（非 WASM）**：检查数据库文件是否存在
    async fn health_check(&self) -> bool {
        // 非 macOS 平台不支持
        if !cfg!(target_os = "macos") {
            return false;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 检查 Messages 数据库文件是否存在
            let db_path = UserDirs::new()
                .map(|u| u.home_dir().join("Library/Messages/chat.db"))
                .unwrap_or_default();

            db_path.exists()
        }
        #[cfg(target_arch = "wasm32")]
        {
            // WASM 架构不支持
            false
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn imessage_database_path() -> anyhow::Result<PathBuf> {
    UserDirs::new()
        .map(|u| u.home_dir().join("Library/Messages/chat.db"))
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_imessage_database_exists(db_path: &Path) -> anyhow::Result<()> {
    if db_path.exists() {
        return Ok(());
    }

    anyhow::bail!(
        "Messages database not found at {}. Ensure Messages.app is set up and Full Disk Access is granted.",
        db_path.display()
    );
}

#[cfg(not(target_arch = "wasm32"))]
async fn open_imessage_connection(db_path: &Path) -> anyhow::Result<Connection> {
    let path = db_path.to_path_buf();
    tokio::task::spawn_blocking(move || -> anyhow::Result<Connection> {
        Ok(Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?)
    })
    .await?
}

#[cfg(not(target_arch = "wasm32"))]
fn max_received_rowid(conn: &Connection) -> anyhow::Result<i64> {
    let mut stmt = conn.prepare("SELECT MAX(ROWID) FROM message WHERE is_from_me = 0")?;
    let rowid: Option<i64> = stmt.query_row([], |row| row.get(0))?;
    Ok(rowid.unwrap_or(0))
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_initial_rowid(conn: Connection) -> anyhow::Result<(Connection, i64)> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<(Connection, i64)> {
        let rowid = max_received_rowid(&conn)?;
        Ok((conn, rowid))
    })
    .await?
}

#[cfg(not(target_arch = "wasm32"))]
fn query_new_messages(
    conn: &Connection,
    since_rowid: i64,
) -> anyhow::Result<Vec<(i64, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT m.ROWID, h.id, m.text \
             FROM message m \
             JOIN handle h ON m.handle_id = h.ROWID \
             WHERE m.ROWID > ?1 \
             AND m.is_from_me = 0 \
             AND m.text IS NOT NULL \
             ORDER BY m.ROWID ASC \
             LIMIT 20",
    )?;
    let rows = stmt.query_map([since_rowid], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(not(target_arch = "wasm32"))]
async fn poll_imessage_connection(
    conn: Connection,
    since_rowid: i64,
) -> anyhow::Result<(Connection, anyhow::Result<Vec<(i64, String, String)>>)> {
    tokio::task::spawn_blocking(
        move || -> (Connection, anyhow::Result<Vec<(i64, String, String)>>) {
            let result = query_new_messages(&conn, since_rowid);
            (conn, result)
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("iMessage poll worker join error: {e}"))
}

/// 获取消息表中的最大 ROWID
///
/// 查询 Messages 数据库中收到的消息的最大 ROWID。
///
/// # 安全措施
///
/// 使用 rusqlite 参数化查询，防止 SQL 注入攻击 (CWE-89: SQL Injection)。
///
/// # 参数
///
/// - `db_path`：Messages 数据库文件路径
///
/// # 返回值
///
/// - `Ok(i64)`：最大 ROWID 值，如果没有消息则返回 0
/// - `Err(...)`：数据库访问失败
///
/// # 查询说明
///
/// 仅查询 `is_from_me = 0` 的消息（收到的消息），
/// 忽略自己发送的消息。
#[cfg(not(target_arch = "wasm32"))]
async fn get_max_rowid(db_path: &Path) -> anyhow::Result<i64> {
    let conn = open_imessage_connection(db_path).await?;
    let (_, rowid) = load_initial_rowid(conn).await?;
    Ok(rowid)
}

/// 获取指定 ROWID 之后的新消息
///
/// 查询 Messages 数据库中 ROWID 大于 `since_rowid` 的新收到的消息。
///
/// # 安全措施
///
/// 使用 rusqlite 参数化查询，`since_rowid` 参数通过绑定方式传递，
/// 有效防止 SQL 注入攻击 (CWE-89: SQL Injection)。
///
/// # 参数
///
/// - `db_path`：Messages 数据库文件路径
/// - `since_rowid`：起始 ROWID，查询所有 ROWID 大于此值的消息
///
/// # 返回值
///
/// - `Ok(Vec<(i64, String, String)>)`：新消息列表
///   - 元组格式：`(ROWID, 发送者ID, 消息文本)`
/// - `Err(...)`：数据库访问失败
///
/// # 查询逻辑
///
/// 1. 仅查询 `is_from_me = 0` 的消息（收到的消息）
/// 2. 仅查询 `text IS NOT NULL` 的消息（非空消息）
/// 3. 通过 JOIN handle 表获取发送者信息
/// 4. 按 ROWID 升序排列，确保消息顺序正确
/// 5. 限制每次最多返回 20 条消息
///
/// # 性能考虑
///
/// - 使用索引字段 ROWID 进行查询，性能良好
/// - 限制返回数量避免大量历史消息堆积
#[cfg(not(target_arch = "wasm32"))]
async fn fetch_new_messages(
    db_path: &Path,
    since_rowid: i64,
) -> anyhow::Result<Vec<(i64, String, String)>> {
    let conn = open_imessage_connection(db_path).await?;
    let (_, result) = poll_imessage_connection(conn, since_rowid).await?;
    result
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
