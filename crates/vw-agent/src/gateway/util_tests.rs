// 验证网关辅助函数对地址、请求数据与边界输入的规范化。
use super::*;
use axum::http::{HeaderMap, HeaderValue};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn channel_message() -> crate::app::agent::channels::traits::ChannelMessage {
    crate::app::agent::channels::traits::ChannelMessage {
        id: "msg-123".to_string(),
        sender: "user-456".to_string(),
        reply_target: "room-789".to_string(),
        content: "hello".to_string(),
        channel: "test".to_string(),
        timestamp: 1_700_000_000,
        thread_ts: Some("thread-1".to_string()),
    }
}

#[test]
fn webhook_memory_key_uses_expected_prefix_and_unique_uuid() {
    let first = webhook_memory_key();
    let second = webhook_memory_key();

    assert!(first.starts_with("webhook_msg_"));
    assert!(second.starts_with("webhook_msg_"));
    assert_ne!(first, second);

    let uuid = first.strip_prefix("webhook_msg_").expect("prefix should be present");
    assert!(uuid::Uuid::parse_str(uuid).is_ok());
}

#[test]
fn channel_memory_keys_include_channel_prefix_sender_and_message_id() {
    let msg = channel_message();

    assert_eq!(whatsapp_memory_key(&msg), "whatsapp_user-456_msg-123");
    assert_eq!(linq_memory_key(&msg), "linq_user-456_msg-123");
    assert_eq!(wati_memory_key(&msg), "wati_user-456_msg-123");
    assert_eq!(nextcloud_talk_memory_key(&msg), "nextcloud_talk_user-456_msg-123");
    assert_eq!(qq_memory_key(&msg), "qq_user-456_msg-123");
}

#[test]
fn hash_webhook_secret_returns_stable_sha256_hex() {
    assert_eq!(
        hash_webhook_secret("secret"),
        concat!("2bb80d537b1da3e38bd30361aa855686bde0eacd", "7162fef6a25fe97bf527a25b")
    );
    assert_eq!(hash_webhook_secret("").len(), 64);
}

#[test]
fn parse_client_ip_handles_forwarded_values() {
    assert_eq!(parse_client_ip("203.0.113.7"), Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))));
    assert_eq!(parse_client_ip("203.0.113.7:443"), Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))));
    assert_eq!(
        parse_client_ip("  \"2001:db8::1\"  "),
        Some(IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)))
    );
    assert_eq!(
        parse_client_ip("[2001:db8::2]:8443"),
        Some(IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 2)))
    );
    assert_eq!(
        parse_client_ip("[2001:db8::3]"),
        Some(IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 3)))
    );
    assert_eq!(parse_client_ip("   "), None);
    assert_eq!(parse_client_ip("not an ip"), None);
}

#[test]
fn forwarded_client_ip_prefers_x_forwarded_for_first_hop() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.7, 10.0.0.1"));

    assert_eq!(forwarded_client_ip(&headers), Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))));
}

#[test]
fn forwarded_client_ip_skips_invalid_x_forwarded_for_entries() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("unknown, 198.51.100.9:9000"));

    assert_eq!(forwarded_client_ip(&headers), Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9))));
}

#[test]
fn forwarded_client_ip_falls_back_to_x_real_ip() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("unknown"));
    headers.insert("x-real-ip", HeaderValue::from_static("192.0.2.10"));

    assert_eq!(forwarded_client_ip(&headers), Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))));
}

#[test]
fn forwarded_client_ip_returns_none_without_valid_forwarded_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-real-ip", HeaderValue::from_static("unknown"));

    assert_eq!(forwarded_client_ip(&headers), None);
    assert_eq!(forwarded_client_ip(&HeaderMap::new()), None);
}

#[test]
fn client_key_from_request_uses_forwarded_ip_when_trusted() {
    let peer_addr = SocketAddr::from(([10, 0, 0, 1], 30_000));
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.8"));

    assert_eq!(client_key_from_request(Some(peer_addr), &headers, true), "203.0.113.8");
}

#[test]
fn client_key_from_request_uses_peer_addr_when_forwarded_headers_are_untrusted() {
    let peer_addr = SocketAddr::from(([10, 0, 0, 1], 30_000));
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.8"));

    assert_eq!(client_key_from_request(Some(peer_addr), &headers, false), "10.0.0.1");
}

#[test]
fn client_key_from_request_falls_back_to_peer_or_unknown() {
    let peer_addr = SocketAddr::from(([10, 0, 0, 2], 30_000));
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("unknown"));

    assert_eq!(client_key_from_request(Some(peer_addr), &headers, true), "10.0.0.2");
    assert_eq!(client_key_from_request(None, &headers, true), "unknown");
}

#[test]
fn normalize_max_keys_preserves_configured_value_or_uses_nonzero_fallback() {
    assert_eq!(normalize_max_keys(42, 10), 42);
    assert_eq!(normalize_max_keys(0, 10), 10);
    assert_eq!(normalize_max_keys(0, 0), 1);
}
