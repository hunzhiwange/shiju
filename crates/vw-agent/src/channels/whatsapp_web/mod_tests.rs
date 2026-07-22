use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};

#[test]
#[cfg(not(feature = "whatsapp-web"))]
fn whatsapp_web_channel_is_disabled_without_feature() {
    let channel = WhatsAppWebChannel::new(
        "session.db".to_string(),
        Some("15551234567".to_string()),
        Some("PAIR".to_string()),
        vec!["*".to_string()],
    );

    assert_eq!(channel.name(), "whatsapp");
}

#[tokio::test]
#[cfg(not(feature = "whatsapp-web"))]
async fn whatsapp_web_stub_methods_report_feature_requirement() {
    let channel = WhatsAppWebChannel::new("session.db".to_string(), None, None, vec![]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let send_err =
        channel.send(&SendMessage::new("hello", "+1234567890")).await.unwrap_err().to_string();
    let listen_err = channel.listen(tx).await.unwrap_err().to_string();
    let start_err = channel.start_typing("+1234567890").await.unwrap_err().to_string();
    let stop_err = channel.stop_typing("+1234567890").await.unwrap_err().to_string();

    assert!(send_err.contains("requires the 'whatsapp-web' feature"));
    assert!(listen_err.contains("requires the 'whatsapp-web' feature"));
    assert!(start_err.contains("requires the 'whatsapp-web' feature"));
    assert!(stop_err.contains("requires the 'whatsapp-web' feature"));
    assert!(!channel.health_check().await);
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn attachment_kind_parses_case_insensitively_and_maps_media_types() {
    assert!(matches!(WaAttachmentKind::from_marker("image"), Some(WaAttachmentKind::Image)));
    assert!(matches!(WaAttachmentKind::from_marker("DOCUMENT"), Some(WaAttachmentKind::Document)));
    assert!(matches!(WaAttachmentKind::from_marker("Video"), Some(WaAttachmentKind::Video)));
    assert!(matches!(WaAttachmentKind::from_marker("AuDiO"), Some(WaAttachmentKind::Audio)));
    assert!(WaAttachmentKind::from_marker("sticker").is_none());

    assert!(matches!(WaAttachmentKind::Image.media_type(), wa_rs_core::download::MediaType::Image));
    assert!(matches!(
        WaAttachmentKind::Document.media_type(),
        wa_rs_core::download::MediaType::Document
    ));
    assert!(matches!(WaAttachmentKind::Video.media_type(), wa_rs_core::download::MediaType::Video));
    assert!(matches!(WaAttachmentKind::Audio.media_type(), wa_rs_core::download::MediaType::Audio));
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn parse_attachment_markers_extracts_valid_entries_and_preserves_invalid_text() {
    let (cleaned, attachments) = parse_wa_attachment_markers(
        " hello [image:/tmp/a.png] keep [BOGUS:/tmp/file] [DOCUMENT:   ] [AUDIO:/tmp/b.ogg] ",
    );

    assert_eq!(cleaned, "hello  keep [BOGUS:/tmp/file] [DOCUMENT:   ]");
    assert_eq!(attachments.len(), 2);
    assert!(matches!(attachments[0].kind, WaAttachmentKind::Image));
    assert_eq!(attachments[0].target, "/tmp/a.png");
    assert!(matches!(attachments[1].kind, WaAttachmentKind::Audio));
    assert_eq!(attachments[1].target, "/tmp/b.ogg");
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn parse_attachment_markers_handles_plain_text_and_unclosed_markers() {
    let (plain_cleaned, plain_attachments) = parse_wa_attachment_markers("just text");
    assert_eq!(plain_cleaned, "just text");
    assert!(plain_attachments.is_empty());

    let (broken_cleaned, broken_attachments) =
        parse_wa_attachment_markers("prefix [IMAGE:/tmp/a.png suffix");
    assert_eq!(broken_cleaned, "prefix [IMAGE:/tmp/a.png suffix");
    assert!(broken_attachments.is_empty());
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn parse_attachment_markers_handles_adjacent_markers_and_colons_in_target() {
    let (cleaned, attachments) =
        parse_wa_attachment_markers("[IMAGE:C:\\media\\a.png][VIDEO:https://example.test/v.mp4]");

    assert!(cleaned.is_empty());
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].target, "C:\\media\\a.png");
    assert_eq!(attachments[1].target, "https://example.test/v.mp4");
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn mime_from_path_recognizes_known_extensions_and_falls_back() {
    use std::path::Path;

    assert_eq!(mime_from_path(Path::new("photo.PNG")), "image/png");
    assert_eq!(mime_from_path(Path::new("video.mov")), "video/quicktime");
    assert_eq!(mime_from_path(Path::new("audio.opus")), "audio/ogg");
    assert_eq!(
        mime_from_path(Path::new("report.xlsx")),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert_eq!(mime_from_path(Path::new("archive.unknown")), "application/octet-stream");
    assert_eq!(mime_from_path(Path::new("no-extension")), "application/octet-stream");
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn mime_from_path_covers_supported_image_audio_video_and_document_extensions() {
    use std::path::Path;

    let cases = [
        ("photo.jpg", "image/jpeg"),
        ("photo.jpeg", "image/jpeg"),
        ("animation.gif", "image/gif"),
        ("photo.webp", "image/webp"),
        ("video.mp4", "video/mp4"),
        ("audio.mp3", "audio/mpeg"),
        ("report.pdf", "application/pdf"),
        ("report.doc", "application/msword"),
        ("report.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        ("sheet.xls", "application/vnd.ms-excel"),
    ];

    for (path, expected) in cases {
        assert_eq!(mime_from_path(Path::new(path)), expected, "path: {path}");
    }
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn constructor_and_helpers_store_state_and_normalize_recipients() {
    let channel = WhatsAppWebChannel::new(
        "session.db".to_string(),
        Some("15551234567".to_string()),
        Some("PAIR".to_string()),
        vec!["+1234567890".to_string(), "*".to_string()],
    );

    assert_eq!(channel.name(), "whatsapp");
    assert_eq!(channel.session_path, "session.db");
    assert_eq!(channel.pair_phone.as_deref(), Some("15551234567"));
    assert_eq!(channel.pair_code.as_deref(), Some("PAIR"));
    assert!(channel.is_number_allowed("+1234567890"));
    assert!(channel.is_number_allowed("+9999999999"));
    assert_eq!(channel.normalize_phone("  15551234567  "), "+15551234567");
    assert_eq!(channel.normalize_phone(" +15551234567@s.whatsapp.net "), "+15551234567");
    assert!(WhatsAppWebChannel::is_jid("12345@s.whatsapp.net"));
    assert!(!WhatsAppWebChannel::is_jid("  +12345  "));
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn recipient_to_jid_accepts_phone_numbers_and_full_jids() {
    let channel = WhatsAppWebChannel::new("session.db".to_string(), None, None, vec![]);

    let numeric = channel.recipient_to_jid(" +1 (555) 123-4567 ").unwrap();
    let jid = channel.recipient_to_jid("12345@s.whatsapp.net").unwrap();
    let empty_err = channel.recipient_to_jid("   ").unwrap_err().to_string();
    let invalid_err = channel.recipient_to_jid("not-a-number").unwrap_err().to_string();

    assert_eq!(numeric.to_string(), "15551234567@s.whatsapp.net");
    assert_eq!(jid.to_string(), "12345@s.whatsapp.net");
    assert!(empty_err.contains("Recipient cannot be empty"));
    assert!(invalid_err.contains("does not contain a valid phone number"));
}

#[cfg(feature = "whatsapp-web")]
#[test]
fn render_pairing_qr_rejects_empty_payload_and_renders_valid_code() {
    let err = WhatsAppWebChannel::render_pairing_qr("   ").unwrap_err().to_string();
    let rendered = WhatsAppWebChannel::render_pairing_qr("pair-me").unwrap();

    assert!(err.contains("QR payload is empty"));
    assert!(!rendered.trim().is_empty());
    assert!(rendered.lines().count() > 1);
}

#[cfg(feature = "whatsapp-web")]
#[tokio::test]
async fn send_and_typing_methods_require_connected_client() {
    let channel = WhatsAppWebChannel::new(
        "session.db".to_string(),
        None,
        None,
        vec!["+1234567890".to_string()],
    );

    let send_err =
        channel.send(&SendMessage::new("hello", "+1234567890")).await.unwrap_err().to_string();
    let start_err = channel.start_typing("+1234567890").await.unwrap_err().to_string();
    let stop_err = channel.stop_typing("+1234567890").await.unwrap_err().to_string();

    assert!(send_err.contains("client not connected"));
    assert!(start_err.contains("client not connected"));
    assert!(stop_err.contains("client not connected"));
}

#[cfg(feature = "whatsapp-web")]
#[tokio::test]
async fn health_check_tracks_whether_bot_handle_is_present() {
    let channel = WhatsAppWebChannel::new("session.db".to_string(), None, None, vec![]);
    assert!(!channel.health_check().await);

    let handle = tokio::spawn(async {});
    *channel.bot_handle.lock() = Some(handle);

    assert!(channel.health_check().await);

    if let Some(handle) = channel.bot_handle.lock().take() {
        handle.abort();
    }
}
