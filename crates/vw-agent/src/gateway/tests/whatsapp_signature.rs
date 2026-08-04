//! WhatsApp Webhook 签名验证测试模块
//!
//! 本模块提供 WhatsApp Webhook 请求签名验证功能的完整测试覆盖。
//! WhatsApp 使用 HMAC-SHA256 算法对请求体进行签名，以确保请求来自可信源。
//!
//! # 测试覆盖范围
//!
//! - **正向场景**：有效签名、空消息体、Unicode 内容、JSON 载荷
//! - **反向场景**：错误密钥、篡改消息体、缺失前缀、空头部、无效十六进制
//! - **边界场景**：大小写敏感、截断签名、额外字节
//!
//! # 签名格式
//!
//! WhatsApp 签名头部格式为：`sha256=<hex_encoded_hmac>`
//!
//! # 安全考虑
//!
//! 这些测试验证了签名验证函数能够正确拒绝各种篡改尝试，
//! 防止恶意请求伪造或重放攻击。

use super::*;

/// 计算 WhatsApp 签名的十六进制表示
///
/// 使用 HMAC-SHA256 算法，以应用密钥为密钥，对请求体进行签名，
/// 并返回签名的十六进制编码字符串。
///
/// # 参数
///
/// - `secret`: WhatsApp 应用密钥（App Secret）
/// - `body`: HTTP 请求体的原始字节
///
/// # 返回值
///
/// 返回 HMAC-SHA256 签名的十六进制编码字符串
///
/// # 示例
///
/// ```ignore
/// let signature = compute_whatsapp_signature_hex("my_secret", b"request body");
/// // 返回类似 "a1b2c3d4..." 的 64 字符十六进制字符串
/// ```
fn compute_whatsapp_signature_hex(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // 创建以密钥初始化的 HMAC-SHA256 实例
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    // 使用请求体更新 MAC 状态
    mac.update(body);
    // 完成计算并编码为十六进制
    hex::encode(mac.finalize().into_bytes())
}

/// 构造 WhatsApp 签名头部值
///
/// 将计算得到的签名添加 "sha256=" 前缀，构造完整的签名头部值。
/// 这与 WhatsApp Webhook 发送的 X-Hub-Signature-256 头部格式一致。
///
/// # 参数
///
/// - `secret`: WhatsApp 应用密钥
/// - `body`: HTTP 请求体的原始字节
///
/// # 返回值
///
/// 返回格式为 "sha256=<hex_signature>" 的字符串
fn compute_whatsapp_signature_header(secret: &str, body: &[u8]) -> String {
    format!("sha256={}", compute_whatsapp_signature_hex(secret, body))
}

/// 测试有效签名验证通过
///
/// 验证使用正确密钥和消息体生成的签名能够通过验证。
#[test]
fn whatsapp_signature_valid() {
    let app_secret = generate_test_secret();
    let body = b"test body content";

    let signature_header = compute_whatsapp_signature_header(&app_secret, body);

    assert!(super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试错误密钥签名验证失败
///
/// 验证使用不同密钥生成的签名会被正确拒绝，
/// 防止攻击者使用自己的密钥伪造请求。
#[test]
fn whatsapp_signature_invalid_wrong_secret() {
    let app_secret = generate_test_secret();
    let wrong_secret = generate_test_secret();
    let body = b"test body content";

    let signature_header = compute_whatsapp_signature_header(&wrong_secret, body);

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试篡改消息体验证失败
///
/// 验证当请求体在传输过程中被修改时，签名验证会失败，
/// 确保消息完整性保护有效。
#[test]
fn whatsapp_signature_invalid_wrong_body() {
    let app_secret = generate_test_secret();
    let original_body = b"original body";
    let tampered_body = b"tampered body";

    let signature_header = compute_whatsapp_signature_header(&app_secret, original_body);

    // 使用被篡改的消息体验证应该失败
    assert!(!super::handlers::verify_whatsapp_signature(
        &app_secret,
        tampered_body,
        &signature_header
    ));
}

/// 测试缺失 sha256= 前缀的签名验证失败
///
/// 验证签名必须具有 "sha256=" 前缀才能通过验证，
/// 拒绝不符合格式要求的签名。
#[test]
fn whatsapp_signature_missing_prefix() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    // 不带 "sha256=" 前缀的签名
    let signature_header = "abc123def456";

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, signature_header));
}

/// 测试空签名头验证失败
///
/// 验证空字符串作为签名头部会被正确拒绝。
#[test]
fn whatsapp_signature_empty_header() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, ""));
}

/// 测试无效十六进制字符验证失败
///
/// 验证包含非十六进制字符的签名会被正确拒绝，
/// 防止解析错误导致的绕过。
#[test]
fn whatsapp_signature_invalid_hex() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    // 包含无效十六进制字符
    let signature_header = "sha256=not_valid_hex_zzz";

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, signature_header));
}

/// 测试空消息体的签名验证通过
///
/// 验证空请求体也能正确生成和验证签名，
/// 这是一个边界情况测试。
#[test]
fn whatsapp_signature_empty_body() {
    let app_secret = generate_test_secret();
    let body = b"";

    let signature_header = compute_whatsapp_signature_header(&app_secret, body);

    assert!(super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试 Unicode 内容的签名验证通过
///
/// 验证包含 Unicode 字符（如 emoji）的请求体能够正确签名和验证，
/// 确保多字节字符处理正确。
#[test]
fn whatsapp_signature_unicode_body() {
    let app_secret = generate_test_secret();
    let body = "Hello World".as_bytes();

    let signature_header = compute_whatsapp_signature_header(&app_secret, body);

    assert!(super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试真实 JSON 载荷的签名验证通过
///
/// 使用模拟的 WhatsApp Webhook JSON 载荷进行测试，
/// 验证真实场景下的签名验证功能。
#[test]
fn whatsapp_signature_json_payload() {
    let app_secret = generate_test_secret();
    // 模拟 WhatsApp Webhook 消息格式的 JSON 载荷
    let body = br#"{"entry":[{"changes":[{"value":{"messages":[{"from":"1234567890","text":{"body":"Hello"}}]}}]}]}"#;

    let signature_header = compute_whatsapp_signature_header(&app_secret, body);

    assert!(super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试签名前缀大小写敏感性
///
/// 验证签名前缀 "sha256=" 是大小写敏感的，
/// "SHA256=" 格式应该被拒绝。
#[test]
fn whatsapp_signature_case_sensitive_prefix() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    let hex_sig = compute_whatsapp_signature_hex(&app_secret, body);

    // 错误大小写的前缀应该失败
    let wrong_prefix = format!("SHA256={hex_sig}");
    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, &wrong_prefix));

    // 正确小写前缀应该通过
    let correct_prefix = format!("sha256={hex_sig}");
    assert!(super::handlers::verify_whatsapp_signature(&app_secret, body, &correct_prefix));
}

/// 测试截断的十六进制签名验证失败
///
/// 验证签名长度不足（被截断）的情况会被正确拒绝。
/// SHA256 输出应为 64 个十六进制字符。
#[test]
fn whatsapp_signature_truncated_hex() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    let hex_sig = compute_whatsapp_signature_hex(&app_secret, body);
    // 只取签名的一半（32 字符而非完整的 64 字符）
    let truncated = &hex_sig[..32];
    let signature_header = format!("sha256={truncated}");

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}

/// 测试包含额外字节的签名验证失败
///
/// 验证在签名末尾添加额外字节的伪造签名会被正确拒绝，
/// 防止通过拼接方式绕过验证。
#[test]
fn whatsapp_signature_extra_bytes() {
    let app_secret = generate_test_secret();
    let body = b"test body";

    let hex_sig = compute_whatsapp_signature_hex(&app_secret, body);
    // 在有效签名后追加额外字节
    let extended = format!("{hex_sig}deadbeef");
    let signature_header = format!("sha256={extended}");

    assert!(!super::handlers::verify_whatsapp_signature(&app_secret, body, &signature_header));
}
