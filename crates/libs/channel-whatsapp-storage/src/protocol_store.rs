//! WhatsApp 协议存储 trait 实现
//!
//! 本模块实现了 WhatsApp Web 协议对齐操作所需的存储接口，主要包括：
//! - SKDM（Sender Key Distribution Message，发送方密钥分发消息）追踪
//! - LID-PN（Link ID - Phone Number，链接ID与手机号）映射管理
//! - 基础密钥碰撞检测
//! - 设备注册表管理
//! - 发送方密钥状态（延迟删除机制）
//! - TcToken（信任链令牌）存储

#[cfg(feature = "whatsapp-web")]
use async_trait::async_trait;
#[cfg(feature = "whatsapp-web")]
use rusqlite::params;

#[cfg(feature = "whatsapp-web")]
use wa_rs_binary::jid::Jid;
#[cfg(feature = "whatsapp-web")]
use wa_rs_core::store::traits::{
    DeviceInfo, DeviceListRecord, LidPnMappingEntry, ProtocolStore, TcTokenEntry,
};

use super::RusqliteStore;

/// 为 RusqliteStore 实现 ProtocolStore trait
///
/// 该实现提供了 WhatsApp Web 协议所需的各种存储操作，包括：
/// - 群组 SKDM 接收者管理
/// - LID 与手机号的双向映射
/// - 基础密钥存储与碰撞检测
/// - 设备列表注册与查询
/// - 发送方密钥的延迟删除标记
/// - 信任链令牌的存储与过期清理
///
/// 所有操作都基于 SQLite 数据库实现，并通过 device_id 进行设备隔离
#[cfg(feature = "whatsapp-web")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ProtocolStore for RusqliteStore {
    /// 获取指定群组的 SKDM 接收者列表
    ///
    /// SKDM（Sender Key Distribution Message）用于在群组通信中分发发送方密钥，
    /// 此方法返回需要接收该消息的设备 JID 列表
    ///
    /// # 参数
    /// - `group_jid`: 群组的 JID（Jabber ID）标识符
    ///
    /// # 返回值
    /// - `Ok(Vec<Jid>)`: 接收者设备的 JID 列表
    /// - `Err`: 数据库操作错误
    ///
    /// # 示例
    /// ```ignore
    /// let recipients = store.get_skdm_recipients("group@example.com").await?;
    /// for jid in recipients {
    ///     println!("需要发送 SKDM 到: {}", jid);
    /// }
    /// ```
    async fn get_skdm_recipients(
        &self,
        group_jid: &str,
    ) -> wa_rs_core::store::error::Result<Vec<Jid>> {
        // 获取数据库连接锁
        let conn = self.conn.lock();

        // 准备查询语句，获取指定群组和设备的 SKDM 接收者
        let mut stmt = to_store_err!(conn.prepare(
            "SELECT device_jid FROM skdm_recipients WHERE group_jid = ?1 AND device_id = ?2"
        ))?;

        // 执行查询并映射结果为字符串
        let rows = to_store_err!(
            stmt.query_map(params![group_jid, self.device_id], |row| { row.get::<_, String>(0) })
        )?;

        // 将查询结果转换为 Jid 对象
        let mut result = Vec::new();
        for row in rows {
            let jid_str = to_store_err!(row)?;
            // 尝试将字符串解析为 Jid，忽略解析失败的条目
            if let Ok(jid) = jid_str.parse() {
                result.push(jid);
            }
        }

        Ok(result)
    }

    /// 添加 SKDM 接收者到指定群组
    ///
    /// 将一组设备 JID 添加到群组的 SKDM 接收者列表中，用于后续的密钥分发
    ///
    /// # 参数
    /// - `group_jid`: 目标群组的 JID
    /// - `device_jids`: 需要添加的设备 JID 列表
    ///
    /// # 返回值
    /// - `Ok(())`: 添加成功
    /// - `Err`: 数据库操作错误
    ///
    /// # 说明
    /// 使用 `INSERT OR IGNORE` 策略，避免重复插入已存在的记录
    async fn add_skdm_recipients(
        &self,
        group_jid: &str,
        device_jids: &[Jid],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        // 获取当前时间戳用于记录创建时间
        let now = chrono::Utc::now().timestamp();

        // 逐个插入接收者记录
        for device_jid in device_jids {
            to_store_err!(execute: conn.execute(
                "INSERT OR IGNORE INTO skdm_recipients (group_jid, device_jid, device_id, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![group_jid, device_jid.to_string(), self.device_id, now],
            ))?;
        }

        Ok(())
    }

    /// 清除指定群组的所有 SKDM 接收者
    ///
    /// 在密钥分发完成后或群组状态变更时，需要清理接收者列表
    ///
    /// # 参数
    /// - `group_jid`: 要清理的群组 JID
    ///
    /// # 返回值
    /// - `Ok(())`: 清除成功
    /// - `Err`: 数据库操作错误
    async fn clear_skdm_recipients(&self, group_jid: &str) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM skdm_recipients WHERE group_jid = ?1 AND device_id = ?2",
            params![group_jid, self.device_id],
        ))
    }

    /// 根据 LID（Link ID）查询映射条目
    ///
    /// LID 是 WhatsApp 中的链接标识符，此方法用于从 LID 查找对应的手机号映射
    ///
    /// # 参数
    /// - `lid`: 链接标识符
    ///
    /// # 返回值
    /// - `Ok(Some(LidPnMappingEntry))`: 找到映射条目
    /// - `Ok(None)`: 未找到映射
    /// - `Err`: 数据库操作错误
    ///
    /// # 示例
    /// ```ignore
    /// if let Some(entry) = store.get_lid_mapping("user_lid_123").await? {
    ///     println!("手机号: {}", entry.phone_number);
    /// }
    /// ```
    async fn get_lid_mapping(
        &self,
        lid: &str,
    ) -> wa_rs_core::store::error::Result<Option<LidPnMappingEntry>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT lid, phone_number, created_at, learning_source, updated_at
             FROM lid_pn_mapping WHERE lid = ?1 AND device_id = ?2",
            params![lid, self.device_id],
            |row| {
                // 将数据库行映射为 LidPnMappingEntry 结构体
                Ok(LidPnMappingEntry {
                    lid: row.get(0)?,
                    phone_number: row.get(1)?,
                    created_at: row.get(2)?,
                    learning_source: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        );

        // 处理查询结果：成功返回条目，无结果返回 None，其他错误抛出
        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    /// 根据手机号查询 LID 映射条目
    ///
    /// 这是 get_lid_mapping 的反向查询，通过手机号查找对应的 LID
    /// 如果存在多个映射，返回最近更新的那个
    ///
    /// # 参数
    /// - `phone`: 手机号码
    ///
    /// # 返回值
    /// - `Ok(Some(LidPnMappingEntry))`: 找到映射条目（最新的）
    /// - `Ok(None)`: 未找到映射
    /// - `Err`: 数据库操作错误
    async fn get_pn_mapping(
        &self,
        phone: &str,
    ) -> wa_rs_core::store::error::Result<Option<LidPnMappingEntry>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT lid, phone_number, created_at, learning_source, updated_at
             FROM lid_pn_mapping WHERE phone_number = ?1 AND device_id = ?2
             ORDER BY updated_at DESC LIMIT 1",
            params![phone, self.device_id],
            |row| {
                Ok(LidPnMappingEntry {
                    lid: row.get(0)?,
                    phone_number: row.get(1)?,
                    created_at: row.get(2)?,
                    learning_source: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    /// 存储或更新 LID-PN 映射条目
    ///
    /// 保存 LID 与手机号的映射关系，使用 `INSERT OR REPLACE` 策略，
    /// 如果映射已存在则更新
    ///
    /// # 参数
    /// - `entry`: 包含完整映射信息的条目
    ///
    /// # 返回值
    /// - `Ok(())`: 保存成功
    /// - `Err`: 数据库操作错误
    async fn put_lid_mapping(
        &self,
        entry: &LidPnMappingEntry,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO lid_pn_mapping
             (lid, phone_number, created_at, learning_source, updated_at, device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.lid,
                entry.phone_number,
                entry.created_at,
                entry.learning_source,
                entry.updated_at,
                self.device_id,
            ],
        ))
    }

    /// 获取当前设备的所有 LID-PN 映射
    ///
    /// 用于批量查询或同步操作，返回当前设备的所有映射条目
    ///
    /// # 返回值
    /// - `Ok(Vec<LidPnMappingEntry>)`: 所有映射条目的列表
    /// - `Err`: 数据库操作错误
    async fn get_all_lid_mappings(
        &self,
    ) -> wa_rs_core::store::error::Result<Vec<LidPnMappingEntry>> {
        let conn = self.conn.lock();
        let mut stmt = to_store_err!(conn.prepare(
            "SELECT lid, phone_number, created_at, learning_source, updated_at
             FROM lid_pn_mapping WHERE device_id = ?1"
        ))?;

        // 映射查询结果
        let rows = to_store_err!(stmt.query_map(params![self.device_id], |row| {
            Ok(LidPnMappingEntry {
                lid: row.get(0)?,
                phone_number: row.get(1)?,
                created_at: row.get(2)?,
                learning_source: row.get(3)?,
                updated_at: row.get(4)?,
            })
        }))?;

        // 收集所有结果
        let mut result = Vec::new();
        for row in rows {
            result.push(to_store_err!(row)?);
        }

        Ok(result)
    }

    /// 保存基础密钥
    ///
    /// 基础密钥用于加密消息，此方法将密钥与地址和消息ID关联存储，
    /// 用于后续的密钥碰撞检测
    ///
    /// # 参数
    /// - `address`: 密钥关联的地址
    /// - `message_id`: 密钥关联的消息ID
    /// - `base_key`: 密钥数据（字节序列）
    ///
    /// # 返回值
    /// - `Ok(())`: 保存成功
    /// - `Err`: 数据库操作错误
    async fn save_base_key(
        &self,
        address: &str,
        message_id: &str,
        base_key: &[u8],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        // 记录创建时间
        let now = chrono::Utc::now().timestamp();

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO base_keys (address, message_id, base_key, device_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![address, message_id, base_key, self.device_id, now],
        ))
    }

    /// 检查是否存在相同的基础密钥（碰撞检测）
    ///
    /// 用于检测密钥重放攻击或重复消息，比较当前密钥与存储的密钥是否一致
    ///
    /// # 参数
    /// - `address`: 密钥关联的地址
    /// - `message_id`: 密钥关联的消息ID
    /// - `current_base_key`: 要检查的密钥数据
    ///
    /// # 返回值
    /// - `Ok(true)`: 存在相同的密钥（可能是重复消息）
    /// - `Ok(false)`: 密钥不同或不存在
    /// - `Err`: 数据库操作错误
    ///
    /// # 安全说明
    /// 此方法对于防止消息重放攻击至关重要
    async fn has_same_base_key(
        &self,
        address: &str,
        message_id: &str,
        current_base_key: &[u8],
    ) -> wa_rs_core::store::error::Result<bool> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT base_key FROM base_keys
             WHERE address = ?1 AND message_id = ?2 AND device_id = ?3",
            params![address, message_id, self.device_id],
            |row| {
                // 获取存储的密钥并与当前密钥比较
                let saved_key: Vec<u8> = row.get(0)?;
                Ok(saved_key == current_base_key)
            },
        );

        match result {
            Ok(same) => Ok(same),
            // 未找到记录表示没有碰撞
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    /// 删除基础密钥
    ///
    /// 清理不再需要的密钥记录，用于密钥轮换或清理过期数据
    ///
    /// # 参数
    /// - `address`: 密钥关联的地址
    /// - `message_id`: 密钥关联的消息ID
    ///
    /// # 返回值
    /// - `Ok(())`: 删除成功（即使记录不存在也算成功）
    /// - `Err`: 数据库操作错误
    async fn delete_base_key(
        &self,
        address: &str,
        message_id: &str,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM base_keys WHERE address = ?1 AND message_id = ?2 AND device_id = ?3",
            params![address, message_id, self.device_id],
        ))
    }

    /// 更新设备列表记录
    ///
    /// 存储或更新用户的设备列表信息，用于多设备同步和管理
    ///
    /// # 参数
    /// - `record`: 设备列表记录，包含用户ID、设备列表、时间戳和 phash
    ///
    /// # 返回值
    /// - `Ok(())`: 更新成功
    /// - `Err`: 数据库或序列化错误
    ///
    /// # 说明
    /// 设备列表以 JSON 格式存储在 devices_json 字段中
    async fn update_device_list(
        &self,
        record: DeviceListRecord,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        // 将设备列表序列化为 JSON
        let devices_json = to_store_err!(serde_json::to_string(&record.devices))?;
        let now = chrono::Utc::now().timestamp();

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO device_registry
             (user_id, devices_json, timestamp, phash, device_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.user,
                devices_json,
                record.timestamp,
                record.phash,
                self.device_id,
                now,
            ],
        ))
    }

    /// 获取用户的设备列表
    ///
    /// 查询指定用户的注册设备信息
    ///
    /// # 参数
    /// - `user`: 用户标识符
    ///
    /// # 返回值
    /// - `Ok(Some(DeviceListRecord))`: 找到设备列表记录
    /// - `Ok(None)`: 用户未注册或无设备记录
    /// - `Err`: 数据库或反序列化错误
    ///
    /// # 示例
    /// ```ignore
    /// if let Some(record) = store.get_devices("user@example.com").await? {
    ///     println!("用户有 {} 个设备", record.devices.len());
    /// }
    /// ```
    async fn get_devices(
        &self,
        user: &str,
    ) -> wa_rs_core::store::error::Result<Option<DeviceListRecord>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT user_id, devices_json, timestamp, phash
             FROM device_registry WHERE user_id = ?1 AND device_id = ?2",
            params![user, self.device_id],
            |row| {
                // 错误转换辅助函数：将通用错误转换为 rusqlite 错误
                fn to_rusqlite_err<E: std::error::Error + Send + Sync + 'static>(
                    e: E,
                ) -> rusqlite::Error {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                }

                // 从 JSON 反序列化设备列表
                let devices_json: String = row.get(1)?;
                let devices: Vec<DeviceInfo> =
                    serde_json::from_str(&devices_json).map_err(to_rusqlite_err)?;

                Ok(DeviceListRecord {
                    user: row.get(0)?,
                    devices,
                    timestamp: row.get(2)?,
                    phash: row.get(3)?,
                })
            },
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    /// 标记发送方密钥为待删除（延迟删除机制）
    ///
    /// 使用延迟删除模式，先将密钥标记为"忘记"状态，
    /// 后续通过 consume_forget_marks 统一处理删除
    ///
    /// # 参数
    /// - `group_jid`: 群组 JID
    /// - `participant`: 参与者 JID（发送方）
    ///
    /// # 返回值
    /// - `Ok(())`: 标记成功
    /// - `Err`: 数据库操作错误
    ///
    /// # 设计说明
    /// 延迟删除允许在删除前进行必要的清理操作，
    /// 避免在活跃通信中立即删除导致的问题
    async fn mark_forget_sender_key(
        &self,
        group_jid: &str,
        participant: &str,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        // 记录标记时间
        let now = chrono::Utc::now().timestamp();

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO sender_key_status (group_jid, participant, device_id, marked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![group_jid, participant, self.device_id, now],
        ))
    }

    /// 消费（获取并清除）所有待删除的发送方密钥标记
    ///
    /// 获取群组中所有被标记为待删除的参与者列表，
    /// 并清除这些标记（原子操作）
    ///
    /// # 参数
    /// - `group_jid`: 群组 JID
    ///
    /// # 返回值
    /// - `Ok(Vec<String>)`: 待删除的参与者 JID 列表
    /// - `Err`: 数据库操作错误
    ///
    /// # 说明
    /// 此方法会先查询所有标记，然后删除这些标记记录，
    /// 调用方应根据返回的列表执行实际的密钥删除操作
    async fn consume_forget_marks(
        &self,
        group_jid: &str,
    ) -> wa_rs_core::store::error::Result<Vec<String>> {
        let conn = self.conn.lock();

        // 查询所有待删除的参与者
        let mut stmt = to_store_err!(conn.prepare(
            "SELECT participant FROM sender_key_status
             WHERE group_jid = ?1 AND device_id = ?2"
        ))?;

        let rows = to_store_err!(
            stmt.query_map(params![group_jid, self.device_id], |row| { row.get::<_, String>(0) })
        )?;

        // 收集所有参与者 JID
        let mut result = Vec::new();
        for row in rows {
            result.push(to_store_err!(row)?);
        }

        // 删除所有已消费的标记
        to_store_err!(execute: conn.execute(
            "DELETE FROM sender_key_status WHERE group_jid = ?1 AND device_id = ?2",
            params![group_jid, self.device_id],
        ))?;

        Ok(result)
    }

    /// 获取指定 JID 的 TcToken（信任链令牌）
    ///
    /// TcToken 用于 WhatsApp 的信任链验证机制
    ///
    /// # 参数
    /// - `jid`: 目标 JID
    ///
    /// # 返回值
    /// - `Ok(Some(TcTokenEntry))`: 找到令牌条目
    /// - `Ok(None)`: 未找到令牌
    /// - `Err`: 数据库操作错误
    async fn get_tc_token(
        &self,
        jid: &str,
    ) -> wa_rs_core::store::error::Result<Option<TcTokenEntry>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT token, token_timestamp, sender_timestamp FROM tc_tokens
             WHERE jid = ?1 AND device_id = ?2",
            params![jid, self.device_id],
            |row| {
                Ok(TcTokenEntry {
                    token: row.get(0)?,
                    token_timestamp: row.get(1)?,
                    sender_timestamp: row.get(2)?,
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    /// 存储或更新 TcToken
    ///
    /// 保存信任链令牌及其时间戳信息
    ///
    /// # 参数
    /// - `jid`: 关联的 JID
    /// - `entry`: 令牌条目，包含令牌数据和两个时间戳
    ///
    /// # 返回值
    /// - `Ok(())`: 保存成功
    /// - `Err`: 数据库操作错误
    async fn put_tc_token(
        &self,
        jid: &str,
        entry: &TcTokenEntry,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        // 记录更新时间
        let now = chrono::Utc::now().timestamp();

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO tc_tokens
             (jid, token, token_timestamp, sender_timestamp, device_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                jid,
                entry.token,
                entry.token_timestamp,
                entry.sender_timestamp,
                self.device_id,
                now,
            ],
        ))
    }

    /// 删除指定 JID 的 TcToken
    ///
    /// 清理不再需要的信任链令牌
    ///
    /// # 参数
    /// - `jid`: 要删除令牌的 JID
    ///
    /// # 返回值
    /// - `Ok(())`: 删除成功
    /// - `Err`: 数据库操作错误
    async fn delete_tc_token(&self, jid: &str) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM tc_tokens WHERE jid = ?1 AND device_id = ?2",
            params![jid, self.device_id],
        ))
    }

    /// 获取所有存储了 TcToken 的 JID 列表
    ///
    /// 用于批量操作或监控所有活跃的信任链令牌
    ///
    /// # 返回值
    /// - `Ok(Vec<String>)`: 所有拥有令牌的 JID 列表
    /// - `Err`: 数据库操作错误
    async fn get_all_tc_token_jids(&self) -> wa_rs_core::store::error::Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt =
            to_store_err!(conn.prepare("SELECT jid FROM tc_tokens WHERE device_id = ?1"))?;

        let rows = to_store_err!(
            stmt.query_map(params![self.device_id], |row| { row.get::<_, String>(0) })
        )?;

        // 收集所有 JID
        let mut result = Vec::new();
        for row in rows {
            result.push(to_store_err!(row)?);
        }

        Ok(result)
    }

    /// 删除过期的 TcToken
    ///
    /// 清理指定时间戳之前的所有令牌，用于定期维护
    ///
    /// # 参数
    /// - `cutoff_timestamp`: 截止时间戳，早于此时间的令牌将被删除
    ///
    /// # 返回值
    /// - `Ok(u32)`: 被删除的令牌数量
    /// - `Err`: 数据库操作错误或整数溢出
    ///
    /// # 示例
    /// ```ignore
    /// // 删除 30 天前的令牌
    /// let cutoff = chrono::Utc::now().timestamp() - 30 * 24 * 60 * 60;
    /// let deleted_count = store.delete_expired_tc_tokens(cutoff).await?;
    /// println!("删除了 {} 个过期令牌", deleted_count);
    /// ```
    async fn delete_expired_tc_tokens(
        &self,
        cutoff_timestamp: i64,
    ) -> wa_rs_core::store::error::Result<u32> {
        let conn = self.conn.lock();

        // 执行删除操作并获取影响的行数
        let deleted = conn
            .execute(
                "DELETE FROM tc_tokens WHERE token_timestamp < ?1 AND device_id = ?2",
                params![cutoff_timestamp, self.device_id],
            )
            .map_err(|e| wa_rs_core::store::error::StoreError::Database(e.to_string()))?;

        // 将行数转换为 u32，处理可能的溢出
        let deleted = u32::try_from(deleted).map_err(|_| {
            wa_rs_core::store::error::StoreError::Database(format!(
                "Affected row count overflowed u32: {deleted}"
            ))
        })?;

        Ok(deleted)
    }
}
