use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[test]
fn terminal_req_03_plain_shortcuts_are_accepted() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        assert!(is_image_paste_key(KeyEvent::new(KeyCode::Char('v'), modifiers)));
        assert!(is_image_paste_key(KeyEvent::new(KeyCode::Char('V'), modifiers)));
    }
}

#[test]
fn terminal_req_03_combined_modifiers_do_not_read_images() {
    for modifiers in [
        KeyModifiers::CONTROL | KeyModifiers::ALT,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
        KeyModifiers::CONTROL | KeyModifiers::SUPER,
    ] {
        assert!(!is_image_paste_key(KeyEvent::new(KeyCode::Char('v'), modifiers)));
    }
}

#[test]
fn terminal_req_03_repeat_and_release_are_ignored() {
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        let event = KeyEvent::new_with_kind(KeyCode::Char('v'), KeyModifiers::CONTROL, kind);
        assert!(!is_image_paste_key(event));
    }
}

#[test]
fn terminal_req_03_unicode_is_not_an_image_shortcut() {
    for value in ['文', '🙂', 'ｖ', 'ṽ'] {
        assert!(!is_image_paste_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::ALT)));
    }
}

#[test]
fn terminal_req_03_zero_dimensions_are_rejected() {
    for (width, height) in [(0, 0), (0, 1), (1, 0)] {
        assert!(encode_rgba_as_png(width, height, &[]).unwrap_err().contains("尺寸不能为零"));
    }
}

#[test]
fn terminal_req_03_extreme_dimensions_are_rejected_before_encoding() {
    assert!(encode_rgba_as_png(u32::MAX, u32::MAX, &[]).is_err());
}

#[test]
fn terminal_req_03_large_compressible_pixels_are_rejected() {
    let pixels = vec![0; MAX_IMAGE_BYTES + 4];
    assert!(
        encode_rgba_as_png((pixels.len() / 4) as u32, 1, &pixels)
            .unwrap_err()
            .contains("原始数据超过 5 MiB")
    );
}

#[test]
fn terminal_req_03_png_writer_enforces_limit_before_growth() {
    let mut bytes = vec![0; MAX_IMAGE_BYTES - 1];
    let mut writer = BoundedPngWriter(&mut bytes);
    writer.write_all(&[1]).unwrap();
    assert!(writer.write_all(&[2]).is_err());
    assert_eq!(bytes.len(), MAX_IMAGE_BYTES);
}

#[test]
fn terminal_req_03_valid_rgba_roundtrips() {
    let rgba = [20, 30, 40, 255];
    let png = encode_rgba_as_png(1, 1, &rgba).unwrap();
    assert_eq!(image::load_from_memory(&png).unwrap().to_rgba8().as_raw(), &rgba);
}
