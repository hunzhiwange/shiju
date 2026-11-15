//! WhatsApp 存储的数据库模式初始化模块
//!
//! 本模块负责初始化 WhatsApp 存储所需的 SQLite 数据库表结构。
//! 包含设备信息、Signal 协议密钥、会话管理、应用状态同步等相关表的定义。
//!
//! # 数据表说明
//!
//! - `device`: 主设备表，存储设备核心信息（密钥、注册ID等）
//! - `identities`: Signal 协议身份密钥表
//! - `sessions`: Signal 协议会话表
//! - `prekeys`: 预共享密钥表（用于密钥交换）
//! - `signed_prekeys`: 签名预共享密钥表
//! - `sender_keys`: 群组消息发送方密钥表
//! - `app_state_keys`: 应用状态同步密钥表
//! - `app_state_versions`: 应用状态版本表
//! - `app_state_mutation_macs`: 应用状态变更 MAC 值表
//! - `lid_pn_mapping`: LID 与电话号码映射表
//! - `skdm_recipients`: SKDM 接收者追踪表
//! - `device_registry`: 多设备注册表
//! - `base_keys`: 基础密钥冲突检测表
//! - `sender_key_status`: 发送方密钥状态表（用于延迟删除）
//! - `tc_tokens`: 受信任联系人令牌表

use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
impl RusqliteStore {
    /// 初始化数据库模式
    ///
    /// 创建 WhatsApp 存储所需的所有数据库表。如果表已存在则不会重新创建。
    /// 该方法会批量执行所有 CREATE TABLE 语句，确保表结构的原子性创建。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 所有表创建成功
    /// - `Err(e)`: 数据库操作失败，返回错误信息
    ///
    /// # 错误处理
    ///
    /// 如果任何 SQL 语句执行失败，整个批次将回滚并返回错误。
    /// 使用 `to_store_err!` 宏将 rusqlite 错误转换为 anyhow 错误。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let store = RusqliteStore::new(path)?;
    /// store.init_schema()?; // 初始化所有表
    /// ```
    ///
    /// # 表结构详细说明
    ///
    /// ## device 表（主设备表）
    /// - `id`: 主键，设备唯一标识
    /// - `lid`: 登录标识符（Login ID）
    /// - `pn`: 电话号码
    /// - `registration_id`: WhatsApp 注册 ID
    /// - `noise_key`: Noise 协议密钥（用于加密握手）
    /// - `identity_key`: 身份密钥对
    /// - `signed_pre_key`: 签名预共享密钥
    /// - `signed_pre_key_id`: 签名预共享密钥 ID
    /// - `signed_pre_key_signature`: 签名预共享密钥的签名
    /// - `adv_secret_key`: 广告密钥
    /// - `account`: 账户数据（BLOB 格式）
    /// - `push_name`: 推送通知名称（用户昵称）
    /// - `app_version_primary/secondary/tertiary`: 应用版本号
    /// - `app_version_last_fetched_ms`: 最后获取版本的时间戳（毫秒）
    /// - `edge_routing_info`: 边缘路由信息
    /// - `props_hash`: 属性哈希值
    ///
    /// ## identities 表（Signal 身份密钥表）
    /// - `address`: 用户地址（JID）
    /// - `key`: 身份公钥
    /// - `device_id`: 设备 ID（支持多设备）
    /// - 主键: (address, device_id)
    ///
    /// ## sessions 表（Signal 协议会话表）
    /// - `address`: 会话对方地址
    /// - `record`: 会话记录（序列化的会话状态）
    /// - `device_id`: 设备 ID
    /// - 主键: (address, device_id)
    ///
    /// ## prekeys 表（预共享密钥表）
    /// - `id`: 预共享密钥 ID
    /// - `key`: 密钥数据
    /// - `uploaded`: 是否已上传到服务器（0=否，1=是）
    /// - `device_id`: 设备 ID
    /// - 主键: (id, device_id)
    ///
    /// ## signed_prekeys 表（签名预共享密钥表）
    /// - `id`: 签名预共享密钥 ID
    /// - `record`: 完整的签名预共享密钥记录
    /// - `device_id`: 设备 ID
    /// - 主键: (id, device_id)
    ///
    /// ## sender_keys 表（群组发送方密钥表）
    /// - `address`: 发送方地址
    /// - `record`: 发送方密钥记录
    /// - `device_id`: 设备 ID
    /// - 主键: (address, device_id)
    /// - 用于群组加密消息的密钥分发
    ///
    /// ## app_state_keys 表（应用状态同步密钥表）
    /// - `key_id`: 密钥标识
    /// - `key_data`: 密钥数据
    /// - `device_id`: 设备 ID
    /// - 主键: (key_id, device_id)
    ///
    /// ## app_state_versions 表（应用状态版本表）
    /// - `name`: 状态名称
    /// - `state_data`: 状态数据
    /// - `device_id`: 设备 ID
    /// - 主键: (name, device_id)
    ///
    /// ## app_state_mutation_macs 表（应用状态变更 MAC 表）
    /// - `name`: 状态名称
    /// - `version`: 版本号
    /// - `index_mac`: 索引 MAC 值
    /// - `value_mac`: 值 MAC 值
    /// - `device_id`: 设备 ID
    /// - 主键: (name, index_mac, device_id)
    /// - 用于验证应用状态变更的完整性
    ///
    /// ## lid_pn_mapping 表（LID 与电话号码映射表）
    /// - `lid`: 登录标识符
    /// - `phone_number`: 电话号码
    /// - `created_at`: 创建时间戳
    /// - `learning_source`: 学习来源（记录从何处获知此映射）
    /// - `updated_at`: 更新时间戳
    /// - `device_id`: 设备 ID
    /// - 主键: (lid, device_id)
    ///
    /// ## skdm_recipients 表（SKDM 接收者追踪表）
    /// - `group_jid`: 群组 JID
    /// - `device_jid`: 设备 JID
    /// - `device_id`: 设备 ID
    /// - `created_at`: 创建时间戳
    /// - 主键: (group_jid, device_jid, device_id)
    /// - SKDM (Sender Key Distribution Message) 用于追踪群组密钥分发
    ///
    /// ## device_registry 表（多设备注册表）
    /// - `user_id`: 用户 ID
    /// - `devices_json`: 设备列表（JSON 格式）
    /// - `timestamp`: 时间戳
    /// - `phash`: 属性哈希
    /// - `device_id`: 设备 ID
    /// - `updated_at`: 更新时间戳
    /// - 主键: (user_id, device_id)
    ///
    /// ## base_keys 表（基础密钥冲突检测表）
    /// - `address`: 用户地址
    /// - `message_id`: 消息 ID
    /// - `base_key`: 基础密钥
    /// - `device_id`: 设备 ID
    /// - `created_at`: 创建时间戳
    /// - 主键: (address, message_id, device_id)
    /// - 用于检测和防止密钥冲突
    ///
    /// ## sender_key_status 表（发送方密钥状态表）
    /// - `group_jid`: 群组 JID
    /// - `participant`: 参与者 JID
    /// - `device_id`: 设备 ID
    /// - `marked_at`: 标记时间戳
    /// - 主键: (group_jid, participant, device_id)
    /// - 用于延迟删除机制，追踪待删除的发送方密钥
    ///
    /// ## tc_tokens 表（受信任联系人令牌表）
    /// - `jid`: 用户 JID
    /// - `token`: 令牌数据
    /// - `token_timestamp`: 令牌时间戳
    /// - `sender_timestamp`: 发送者时间戳（可选）
    /// - `device_id`: 设备 ID
    /// - `updated_at`: 更新时间戳
    /// - 主键: (jid, device_id)
    /// - 存储受信任联系人的验证令牌
    pub(super) fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(conn.execute_batch(
            "-- 主设备表：存储 WhatsApp 设备的核心信息
            CREATE TABLE IF NOT EXISTS device (
                id INTEGER PRIMARY KEY,
                lid TEXT,
                pn TEXT,
                registration_id INTEGER NOT NULL,
                noise_key BLOB NOT NULL,
                identity_key BLOB NOT NULL,
                signed_pre_key BLOB NOT NULL,
                signed_pre_key_id INTEGER NOT NULL,
                signed_pre_key_signature BLOB NOT NULL,
                adv_secret_key BLOB NOT NULL,
                account BLOB,
                push_name TEXT NOT NULL,
                app_version_primary INTEGER NOT NULL,
                app_version_secondary INTEGER NOT NULL,
                app_version_tertiary INTEGER NOT NULL,
                app_version_last_fetched_ms INTEGER NOT NULL,
                edge_routing_info BLOB,
                props_hash TEXT
            );

            -- Signal 协议身份密钥表：存储联系人的身份公钥
            CREATE TABLE IF NOT EXISTS identities (
                address TEXT NOT NULL,
                key BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (address, device_id)
            );

            -- Signal 协议会话表：存储加密会话状态
            CREATE TABLE IF NOT EXISTS sessions (
                address TEXT NOT NULL,
                record BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (address, device_id)
            );

            -- 预共享密钥表：用于 Signal 协议密钥交换
            CREATE TABLE IF NOT EXISTS prekeys (
                id INTEGER NOT NULL,
                key BLOB NOT NULL,
                uploaded INTEGER NOT NULL DEFAULT 0,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (id, device_id)
            );

            -- 签名预共享密钥表：存储带签名的预共享密钥
            CREATE TABLE IF NOT EXISTS signed_prekeys (
                id INTEGER NOT NULL,
                record BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (id, device_id)
            );

            -- 群组消息发送方密钥表：用于群组加密消息的密钥分发
            CREATE TABLE IF NOT EXISTS sender_keys (
                address TEXT NOT NULL,
                record BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (address, device_id)
            );

            -- 应用状态同步密钥表：用于跨设备状态同步
            CREATE TABLE IF NOT EXISTS app_state_keys (
                key_id BLOB NOT NULL,
                key_data BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (key_id, device_id)
            );

            -- 应用状态版本表：跟踪各状态数据的版本
            CREATE TABLE IF NOT EXISTS app_state_versions (
                name TEXT NOT NULL,
                state_data BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (name, device_id)
            );

            -- 应用状态变更 MAC 表：验证状态变更的完整性
            CREATE TABLE IF NOT EXISTS app_state_mutation_macs (
                name TEXT NOT NULL,
                version INTEGER NOT NULL,
                index_mac BLOB NOT NULL,
                value_mac BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (name, index_mac, device_id)
            );

            -- LID 与电话号码映射表：维护登录ID与电话号码的对应关系
            CREATE TABLE IF NOT EXISTS lid_pn_mapping (
                lid TEXT NOT NULL,
                phone_number TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                learning_source TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                device_id INTEGER NOT NULL,
                PRIMARY KEY (lid, device_id)
            );

            -- SKDM 接收者追踪表：追踪群组密钥分发消息的接收者
            CREATE TABLE IF NOT EXISTS skdm_recipients (
                group_jid TEXT NOT NULL,
                device_jid TEXT NOT NULL,
                device_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (group_jid, device_jid, device_id)
            );

            -- 多设备注册表：存储用户的多设备信息
            CREATE TABLE IF NOT EXISTS device_registry (
                user_id TEXT NOT NULL,
                devices_json TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                phash TEXT,
                device_id INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (user_id, device_id)
            );

            -- 基础密钥冲突检测表：用于检测和防止密钥冲突
            CREATE TABLE IF NOT EXISTS base_keys (
                address TEXT NOT NULL,
                message_id TEXT NOT NULL,
                base_key BLOB NOT NULL,
                device_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (address, message_id, device_id)
            );

            -- 发送方密钥状态表：用于延迟删除机制
            CREATE TABLE IF NOT EXISTS sender_key_status (
                group_jid TEXT NOT NULL,
                participant TEXT NOT NULL,
                device_id INTEGER NOT NULL,
                marked_at INTEGER NOT NULL,
                PRIMARY KEY (group_jid, participant, device_id)
            );

            -- 受信任联系人令牌表：存储联系人的信任令牌
            CREATE TABLE IF NOT EXISTS tc_tokens (
                jid TEXT NOT NULL,
                token BLOB NOT NULL,
                token_timestamp INTEGER NOT NULL,
                sender_timestamp INTEGER,
                device_id INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (jid, device_id)
            );",
        ))?;
        Ok(())
    }
}
