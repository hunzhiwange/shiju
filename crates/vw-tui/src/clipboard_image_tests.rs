use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{encode_rgba_as_png, is_image_paste_key};

#[test]
fn image_paste_key_accepts_control_or_alt_v_only() {
    assert!(is_image_paste_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)));
    assert!(is_image_paste_key(KeyEvent::new(KeyCode::Char('V'), KeyModifiers::ALT)));
    assert!(!is_image_paste_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)));
    assert!(!is_image_paste_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
}

#[test]
fn rgba_clipboard_image_encodes_as_png() {
    let encoded = encode_rgba_as_png(1, 1, &[0x11, 0x22, 0x33, 0xff]).expect("encode png");
    assert!(encoded.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn rgba_clipboard_image_rejects_invalid_buffer_length() {
    assert_eq!(
        encode_rgba_as_png(2, 2, &[0; 4]).expect_err("invalid buffer"),
        "剪贴板图片 RGBA 数据长度无效"
    );
}
