//! WhatsApp rusqlite 存储后端测试。
//!
//! 本模块在启用 `whatsapp-web` 特性时验证自定义 rusqlite 存储实现的关键路径：
//! 数据库初始化、LID/手机号映射往返，以及过期 TC token 清理。测试使用临时文件，
//! 避免污染真实会话数据库。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    #[cfg(feature = "whatsapp-web")]
    use wa_rs_core::store::traits::{LidPnMappingEntry, ProtocolStore, TcTokenEntry};

    #[cfg(feature = "whatsapp-web")]
    #[test]
    fn rusqlite_store_creates_database() {
        // 临时文件覆盖“路径已存在但内容为空”的常见启动场景。
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let store = RusqliteStore::new(tmp.path()).unwrap();
        assert_eq!(store.device_id, 1);
    }

    #[cfg(feature = "whatsapp-web")]
    #[tokio::test]
    async fn lid_mapping_round_trip_preserves_learning_source_and_updated_at() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let store = RusqliteStore::new(tmp.path()).unwrap();
        // learning_source 与 updated_at 会影响 wa-rs 后续同步判断，必须在双向索引
        // 读取时都保持一致，不能只验证 lid 主查询路径。
        let entry = LidPnMappingEntry {
            lid: "100000012345678".to_string(),
            phone_number: "15551234567".to_string(),
            created_at: 1_700_000_000,
            updated_at: 1_700_000_100,
            learning_source: "usync".to_string(),
        };

        ProtocolStore::put_lid_mapping(&store, &entry).await.unwrap();

        let loaded = ProtocolStore::get_lid_mapping(&store, &entry.lid)
            .await
            .unwrap()
            .expect("expected lid mapping to be present");
        assert_eq!(loaded.learning_source, entry.learning_source);
        assert_eq!(loaded.updated_at, entry.updated_at);

        let loaded_by_pn = ProtocolStore::get_pn_mapping(&store, &entry.phone_number)
            .await
            .unwrap()
            .expect("expected pn mapping to be present");
        assert_eq!(loaded_by_pn.learning_source, entry.learning_source);
        assert_eq!(loaded_by_pn.updated_at, entry.updated_at);
    }

    #[cfg(feature = "whatsapp-web")]
    #[tokio::test]
    async fn delete_expired_tc_tokens_returns_deleted_row_count() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let store = RusqliteStore::new(tmp.path()).unwrap();

        // 同时写入过期和未过期 token，确保删除逻辑按阈值筛选，
        // 并返回实际删除行数供调用方记录维护结果。
        let expired =
            TcTokenEntry { token: vec![1, 2, 3], token_timestamp: 10, sender_timestamp: None };
        let fresh = TcTokenEntry {
            token: vec![4, 5, 6],
            token_timestamp: 1000,
            sender_timestamp: Some(1000),
        };

        ProtocolStore::put_tc_token(&store, "15550000001", &expired).await.unwrap();
        ProtocolStore::put_tc_token(&store, "15550000002", &fresh).await.unwrap();

        let deleted = ProtocolStore::delete_expired_tc_tokens(&store, 100).await.unwrap();
        assert_eq!(deleted, 1);
        assert!(ProtocolStore::get_tc_token(&store, "15550000001").await.unwrap().is_none());
        assert!(ProtocolStore::get_tc_token(&store, "15550000002").await.unwrap().is_some());
    }
}
