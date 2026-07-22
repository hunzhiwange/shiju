//! Nostr 通道模块单元测试
//!
//! 本模块包含 NostrChannel 的全面测试套件，验证以下功能：
//! - **允许列表（AllowList）**：访问控制列表的解析、通配符支持、公钥验证
//! - **通道初始化**：密钥解析、允许列表初始化、错误处理
//! - **健康检查**：中继连接状态验证
//! - **协议管理**：NIP-04/NIP-17 协议的默认值与动态更新
//!
//! # 测试分类
//!
//! | 类别 | 测试函数 | 验证内容 |
//! |------|---------|---------|
//! | 允许列表 | `allow_list_*` | 公钥过滤逻辑 |
//! | 通道创建 | `nostr_channel_*`, `new_*` | 初始化与参数验证 |
//! | 健康检查 | `health_check_*` | 中继连接状态 |
//! | 协议管理 | `*_protocol_*` | NIP 协议选择 |

use super::*;

/// 允许列表与通道功能测试模块
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试空允许列表拒绝所有公钥
    ///
    /// # 验证内容
    /// - 当允许列表为空时，任何公钥都应被拒绝
    /// - `AllowList::parse` 能正确处理空输入
    /// - `is_allowed` 对空列表返回 `false`
    #[test]
    fn allow_list_empty_denies_all() {
        // 解析空列表
        let al = AllowList::parse(&[]).unwrap();
        // 生成随机公钥
        let pk = Keys::generate().public_key();
        // 空列表应拒绝所有访问
        assert!(!al.is_allowed(&pk));
    }

    /// 测试通配符允许列表接受所有公钥
    ///
    /// # 验证内容
    /// - `"*"` 通配符表示允许所有公钥访问
    /// - 任意生成的公钥都应通过验证
    #[test]
    fn allow_list_wildcard_allows_all() {
        // 使用通配符创建允许列表
        let al = AllowList::parse(&["*".to_string()]).unwrap();
        // 生成随机公钥
        let pk = Keys::generate().public_key();
        // 通配符应允许所有访问
        assert!(al.is_allowed(&pk));
    }

    /// 测试指定公钥的允许列表精确匹配
    ///
    /// # 验证内容
    /// - 只有列表中明确指定的公钥才被允许
    /// - 未在列表中的公钥应被拒绝
    /// - 公钥以十六进制格式进行比较
    #[test]
    fn allow_list_specific_pubkeys() {
        // 生成三个不同的密钥对
        let k1 = Keys::generate();
        let k2 = Keys::generate();
        let k3 = Keys::generate();

        // 创建包含 k1 和 k2 公钥的允许列表
        let al = AllowList::parse(&[k1.public_key().to_hex(), k2.public_key().to_hex()]).unwrap();

        // k1 应被允许（在列表中）
        assert!(al.is_allowed(&k1.public_key()));
        // k2 应被允许（在列表中）
        assert!(al.is_allowed(&k2.public_key()));
        // k3 应被拒绝（不在列表中）
        assert!(!al.is_allowed(&k3.public_key()));
    }

    /// 测试允许列表拒绝无效公钥格式
    ///
    /// # 验证内容
    /// - 解析无效的公钥字符串应返回错误
    /// - `AllowList::parse` 对格式错误的输入进行严格验证
    #[test]
    fn allow_list_rejects_invalid_key() {
        // 尝试解析无效的公钥字符串
        let result = AllowList::parse(&["not-a-valid-pubkey".to_string()]);
        // 应返回错误
        assert!(result.is_err());
    }

    /// 测试 Nostr 通道名称固定为 "nostr"
    ///
    /// # 验证内容
    /// - `NostrChannel::name()` 方法始终返回 `"nostr"`
    /// - 用于通道识别和路由
    #[tokio::test]
    async fn nostr_channel_name_is_nostr() {
        // 生成测试密钥
        let keys = Keys::generate();
        // 创建 Nostr 通道实例
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        // 验证通道名称
        assert_eq!(ch.name(), "nostr");
    }

    /// 测试 Nostr 通道正确存储解析后的密钥
    ///
    /// # 验证内容
    /// - 从密钥字符串创建通道后，公钥应正确存储
    /// - `public_key` 字段与输入密钥对一致
    #[tokio::test]
    async fn nostr_channel_stores_parsed_keys() {
        // 生成测试密钥
        let keys = Keys::generate();
        // 使用密钥创建通道
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        // 验证存储的公钥与原始密钥一致
        assert_eq!(ch.public_key, keys.public_key());
    }

    /// 测试通道创建拒绝无效密钥格式
    ///
    /// # 验证内容
    /// - 使用无效的密钥字符串创建通道应返回错误
    /// - 防止使用格式错误的密钥初始化通道
    #[tokio::test]
    async fn new_rejects_invalid_key() {
        // 尝试使用无效密钥创建通道
        let result = NostrChannel::new("not-a-valid-key", vec![], &[]).await;
        // 应返回错误
        assert!(result.is_err());
    }

    /// 测试通道创建拒绝允许列表中的无效公钥
    ///
    /// # 验证内容
    /// - 即使密钥有效，允许列表中的无效公钥也会导致创建失败
    /// - 确保所有配置参数在初始化时都经过验证
    #[tokio::test]
    async fn new_rejects_invalid_allowed_pubkey() {
        // 生成有效的密钥
        let keys = Keys::generate();
        // 尝试使用有效密钥但包含无效允许列表项创建通道
        let result = NostrChannel::new(
            &keys.secret_key().to_secret_hex(),
            vec![],
            &["bad-pubkey".to_string()],
        )
        .await;
        // 应返回错误
        assert!(result.is_err());
    }

    /// 测试通道创建为无效中继地址保留具体配置上下文。
    #[tokio::test]
    async fn new_rejects_invalid_relay_url_with_context() {
        let keys = Keys::generate();

        let error = NostrChannel::new(
            &keys.secret_key().to_secret_hex(),
            vec!["not-a-relay".to_string()],
            &[],
        )
        .await
        .err()
        .expect("invalid relay URL should fail");

        assert!(error.to_string().contains("添加中继服务器失败: not-a-relay"));
    }

    /// 测试无中继时健康检查返回 false
    ///
    /// # 验证内容
    /// - 当没有配置任何中继时，`health_check` 应返回 `false`
    /// - 用于判断通道是否可用于消息发送
    #[tokio::test]
    async fn health_check_false_with_no_relays() {
        // 生成测试密钥
        let keys = Keys::generate();
        // 创建无中继的通道
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        // 无中继时应返回 false
        assert!(!ch.health_check().await);
    }

    /// 测试默认协议为 NIP-17
    ///
    /// # 验证内容
    /// - 新创建的通道中，发送方协议映射默认为空
    /// - 未显式设置的发送方不会出现在协议映射中
    /// - 空映射表示使用默认协议（NIP-17）
    #[tokio::test]
    async fn default_protocol_is_nip17() {
        // 生成测试密钥
        let keys = Keys::generate();
        // 创建通道
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        // 读取协议映射
        let map = ch.sender_protocols.read().await;
        // 生成一个未设置协议的公钥
        let pk = Keys::generate().public_key();
        // 该公钥不应存在于映射中（表示使用默认协议）
        assert_eq!(map.get(&pk), None);
    }

    /// 测试发送方协议的动态更新
    ///
    /// # 验证内容
    /// - `sender_protocols` 映射支持动态添加和更新
    /// - 协议可以在 NIP-04 和 NIP-17 之间切换
    /// - 读写锁正常工作，支持并发安全访问
    #[tokio::test]
    async fn sender_protocol_tracks_updates() {
        // 生成测试密钥
        let keys = Keys::generate();
        // 创建通道
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        // 生成目标发送方的公钥
        let pk = Keys::generate().public_key();

        // 第一次更新：设置协议为 NIP-04
        {
            let mut map = ch.sender_protocols.write().await;
            map.insert(pk, NostrProtocol::Nip04);
        }
        // 验证：协议应为 NIP-04
        {
            let map = ch.sender_protocols.read().await;
            assert_eq!(map.get(&pk), Some(&NostrProtocol::Nip04));
        }

        // 第二次更新：将协议切换为 NIP-17
        {
            let mut map = ch.sender_protocols.write().await;
            map.insert(pk, NostrProtocol::Nip17);
        }
        // 验证：协议应更新为 NIP-17
        {
            let map = ch.sender_protocols.read().await;
            assert_eq!(map.get(&pk), Some(&NostrProtocol::Nip17));
        }
    }

    /// 测试发送消息时在任何网络调用前拒绝无效收件人公钥。
    #[tokio::test]
    async fn send_rejects_invalid_recipient_pubkey() {
        let keys = Keys::generate();
        let ch = NostrChannel::new(&keys.secret_key().to_secret_hex(), vec![], &[]).await.unwrap();
        let message = SendMessage::new("hello", "not-a-valid-pubkey");

        let err = ch.send(&message).await.expect_err("invalid recipient should fail");

        assert!(err.to_string().contains("无效的接收者 Nostr 公钥"));
    }
}
