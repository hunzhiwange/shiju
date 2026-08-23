use super::*;

fn parsed_message(raw: &[u8]) -> mail_parser::Message<'_> {
    MessageParser::default().parse(raw).unwrap()
}

fn email_config() -> EmailConfig {
    EmailConfig {
        smtp_host: "localhost".to_string(),
        smtp_port: 25,
        smtp_tls: false,
        username: "user@example.com".to_string(),
        password: "password".to_string(),
        from_address: "bot@example.com".to_string(),
        ..Default::default()
    }
}

#[test]
fn extract_sender_reads_first_from_address() {
    let message =
        parsed_message(b"From: Alice <alice@example.com>\r\nSubject: Hello\r\n\r\nPlain body");

    assert_eq!(EmailChannel::extract_sender(&message), "alice@example.com");
}

#[test]
fn extract_sender_defaults_unknown_without_from() {
    let message = parsed_message(b"Subject: Hello\r\n\r\nPlain body");

    assert_eq!(EmailChannel::extract_sender(&message), "unknown");
}

#[test]
fn extract_text_prefers_plain_body() {
    let message = parsed_message(b"Content-Type: text/plain; charset=utf-8\r\n\r\nPlain body");

    assert_eq!(EmailChannel::extract_text(&message), "Plain body");
}

#[test]
fn extract_text_uses_html_body() {
    let message = parsed_message(
        b"Content-Type: text/html; charset=utf-8\r\n\r\n<p>Hello <strong>HTML</strong></p>",
    );

    assert_eq!(EmailChannel::extract_text(&message), "Hello HTML");
}

#[test]
fn extract_text_reads_text_attachment() {
    let message = parsed_message(
        b"Content-Type: multipart/mixed; boundary=\"frontier\"\r\n\r\n--frontier\r\nContent-Type: application/octet-stream\r\n\r\nbinary\r\n--frontier\r\nContent-Type: text/plain; name=\"note.txt\"\r\nContent-Disposition: attachment; filename=\"note.txt\"\r\n\r\nAttached text\r\n--frontier--",
    );

    assert_eq!(EmailChannel::extract_text(&message), "[Attachment: note.txt]\nAttached text");
}

#[test]
fn extract_text_falls_back_when_attachment_is_not_utf8() {
    let message = parsed_message(
        b"Content-Type: multipart/mixed; boundary=\"frontier\"\r\n\r\n--frontier\r\nContent-Type: text/plain; name=\"bad.txt\"\r\nContent-Disposition: attachment; filename=\"bad.txt\"\r\n\r\n\xff\xfe\xfd\r\n--frontier--",
    );

    assert_eq!(EmailChannel::extract_text(&message), "(no readable content)");
}

#[test]
fn extract_text_defaults_without_readable_parts() {
    let message = parsed_message(b"Content-Type: application/octet-stream\r\n\r\nbinary");

    assert_eq!(EmailChannel::extract_text(&message), "(no readable content)");
}

#[test]
fn parsed_email_from_body_formats_content_and_fallback_timestamp() {
    let email = EmailChannel::parsed_email_from_body(
        7,
        b"From: Alice <alice@example.com>\r\nSubject: Parsed\r\nMessage-ID: <parsed@example.com>\r\n\r\nBody text",
        1234,
    )
    .unwrap();

    assert_eq!(email._uid, 7);
    assert_eq!(email.msg_id, "parsed@example.com");
    assert_eq!(email.sender, "alice@example.com");
    assert_eq!(email.content, "Subject: Parsed\n\nBody text");
    assert_eq!(email.timestamp, 1234);
}

#[test]
fn parsed_email_from_body_uses_generated_id_when_message_id_missing() {
    let email = EmailChannel::parsed_email_from_body(
        9,
        b"From: Alice <alice@example.com>\r\nSubject: No Id\r\n\r\nBody text",
        4321,
    )
    .unwrap();

    assert!(email.msg_id.starts_with("gen-"));
    assert_eq!(email.timestamp, 4321);
}

#[test]
fn parsed_email_from_body_uses_message_date_timestamp() {
    let email = EmailChannel::parsed_email_from_body(
        10,
        b"From: Alice <alice@example.com>\r\nSubject: Dated\r\nMessage-ID: <dated@example.com>\r\nDate: Thu, 1 Jan 1970 00:00:01 +0000\r\n\r\nBody text",
        4321,
    )
    .unwrap();

    assert_eq!(email.timestamp, 1);
}

#[test]
fn parsed_email_from_body_defaults_subject_when_missing() {
    let email = EmailChannel::parsed_email_from_body(
        11,
        b"From: Alice <alice@example.com>\r\nMessage-ID: <subjectless@example.com>\r\n\r\nBody text",
        4321,
    )
    .unwrap();

    assert_eq!(email.content, "Subject: (no subject)\n\nBody text");
}

#[test]
fn parsed_email_from_body_strips_html_body() {
    let email = EmailChannel::parsed_email_from_body(
        12,
        b"From: Alice <alice@example.com>\r\nSubject: Html\r\nMessage-ID: <html@example.com>\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>Hello <strong>HTML</strong></p>",
        777,
    )
    .unwrap();

    assert_eq!(email.content, "Subject: Html\n\nHello HTML");
}

#[test]
fn parsed_email_from_body_reads_text_attachment_body() {
    let email = EmailChannel::parsed_email_from_body(
        13,
        b"From: Alice <alice@example.com>\r\nSubject: Attachment\r\nMessage-ID: <attachment@example.com>\r\nContent-Type: multipart/mixed; boundary=\"frontier\"\r\n\r\n--frontier\r\nContent-Type: application/octet-stream\r\n\r\nbinary\r\n--frontier\r\nContent-Type: text/plain; name=\"note.txt\"\r\nContent-Disposition: attachment; filename=\"note.txt\"\r\n\r\nAttached text\r\n--frontier--",
        888,
    )
    .unwrap();

    assert_eq!(email.content, "Subject: Attachment\n\n[Attachment: note.txt]\nAttached text");
}

#[test]
fn parsed_email_from_body_uses_zero_for_invalid_date_components() {
    let email = EmailChannel::parsed_email_from_body(
        14,
        b"From: Alice <alice@example.com>\r\nSubject: Invalid Date\r\nMessage-ID: <invalid-date@example.com>\r\nDate: Thu, 32 Jan 1970 00:00:01 +0000\r\n\r\nBody text",
        4321,
    )
    .unwrap();

    assert_eq!(email.timestamp, 0);
}

#[test]
fn parsed_email_from_body_returns_none_for_invalid_body() {
    assert!(EmailChannel::parsed_email_from_body(1, b"", 42).is_none());
}

#[test]
fn outgoing_subject_and_body_prefers_explicit_subject() {
    let message = SendMessage::with_subject("body", "bob@example.com", "Explicit");

    assert_eq!(EmailChannel::outgoing_subject_and_body(&message), ("Explicit", "body"));
}

#[test]
fn outgoing_subject_and_body_parses_legacy_subject_line() {
    let message = SendMessage::new("Subject: Legacy\nbody", "bob@example.com");

    assert_eq!(EmailChannel::outgoing_subject_and_body(&message), ("Legacy", "body"));
}

#[test]
fn outgoing_subject_and_body_defaults_when_legacy_subject_has_no_body() {
    let message = SendMessage::new("Subject: Legacy", "bob@example.com");

    assert_eq!(
        EmailChannel::outgoing_subject_and_body(&message),
        ("VibeWindow Message", "Subject: Legacy")
    );
}

#[test]
fn outgoing_subject_and_body_defaults_for_plain_content() {
    let message = SendMessage::new("body", "bob@example.com");

    assert_eq!(EmailChannel::outgoing_subject_and_body(&message), ("VibeWindow Message", "body"));
}

#[test]
fn outgoing_subject_and_body_trims_legacy_body_after_subject() {
    let message = SendMessage::new("Subject: Legacy\n\n  body with space  \n", "bob@example.com");

    assert_eq!(EmailChannel::outgoing_subject_and_body(&message), ("Legacy", "body with space"));
}

#[test]
fn build_outgoing_email_uses_subject_and_body() {
    let channel = EmailChannel::new(email_config());
    let message = SendMessage::with_subject("body", "bob@example.com", "Explicit");
    let email = channel.build_outgoing_email(&message).unwrap();
    let formatted = String::from_utf8(email.formatted()).unwrap();

    assert!(formatted.contains("Subject: Explicit"));
    assert!(formatted.trim_end().ends_with("\r\n\r\nbody"));
}

#[test]
fn build_outgoing_email_uses_default_subject_for_plain_message() {
    let channel = EmailChannel::new(email_config());
    let message = SendMessage::new("body", "bob@example.com");
    let email = channel.build_outgoing_email(&message).unwrap();
    let formatted = String::from_utf8(email.formatted()).unwrap();

    assert!(formatted.contains("Subject: VibeWindow Message"));
    assert!(formatted.trim_end().ends_with("\r\n\r\nbody"));
}

#[test]
fn build_outgoing_email_parses_legacy_subject_line() {
    let channel = EmailChannel::new(email_config());
    let message = SendMessage::new("Subject: Legacy\n\nBody", "bob@example.com");
    let email = channel.build_outgoing_email(&message).unwrap();
    let formatted = String::from_utf8(email.formatted()).unwrap();

    assert!(formatted.contains("Subject: Legacy"));
    assert!(formatted.trim_end().ends_with("\r\n\r\nBody"));
}

#[test]
fn build_outgoing_email_rejects_invalid_sender_address() {
    let channel = EmailChannel::new(EmailConfig {
        from_address: "invalid address".to_string(),
        ..email_config()
    });
    let message = SendMessage::new("body", "bob@example.com");

    assert!(channel.build_outgoing_email(&message).is_err());
}

#[test]
fn build_outgoing_email_rejects_invalid_recipient_address() {
    let channel = EmailChannel::new(email_config());
    let message = SendMessage::new("body", "invalid address");

    assert!(channel.build_outgoing_email(&message).is_err());
}

#[test]
fn create_smtp_transport_builds_plain_transport() {
    let channel = EmailChannel::new(email_config());

    assert!(channel.create_smtp_transport().is_ok());
}

#[test]
fn create_smtp_transport_builds_tls_transport() {
    let channel = EmailChannel::new(EmailConfig {
        smtp_host: "smtp.example.com".to_string(),
        smtp_tls: true,
        ..email_config()
    });

    assert!(channel.create_smtp_transport().is_ok());
}

#[test]
fn create_smtp_transport_rejects_invalid_relay_host() {
    let channel = EmailChannel::new(EmailConfig {
        smtp_host: "bad host name".to_string(),
        smtp_tls: true,
        ..email_config()
    });

    assert!(channel.create_smtp_transport().is_err());
}

#[tokio::test]
async fn send_rejects_invalid_recipient_before_smtp_connection() {
    let channel = EmailChannel::new(email_config());
    let message = SendMessage::new("body", "invalid address");

    let error = channel.send(&message).await.expect_err("invalid recipient should be rejected");

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn send_rejects_invalid_sender_before_smtp_connection() {
    let channel = EmailChannel::new(EmailConfig {
        from_address: "invalid address".to_string(),
        ..email_config()
    });
    let message = SendMessage::new("body", "bob@example.com");

    let error = channel.send(&message).await.expect_err("invalid sender should be rejected");

    assert!(!error.to_string().is_empty());
}

#[test]
fn outgoing_subject_and_body_preserves_empty_explicit_subject() {
    let message = SendMessage::with_subject("body", "bob@example.com", "");

    assert_eq!(EmailChannel::outgoing_subject_and_body(&message), ("", "body"));
}
// 本文件验证邮件发送者、正文、附件与时间戳的解析回退规则。
