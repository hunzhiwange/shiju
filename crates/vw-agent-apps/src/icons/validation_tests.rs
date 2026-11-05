use super::validation::is_valid_custom_icon;
use base64::{Engine, engine::general_purpose::STANDARD};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&vec![128; (width * height * 4) as usize]).unwrap();
    }
    bytes
}
fn uri(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}
#[test]
fn cc_base_icon_png_accepts_complete_and_max_dimension() {
    for size in [1, 96, 256] {
        assert!(is_valid_custom_icon(&uri(&png(size, size))));
    }
}
#[test]
fn cc_base_icon_png_rejects_truncation() {
    let bytes = png(16, 16);
    for end in [33, bytes.len() / 2, bytes.len() - 12] {
        assert!(!is_valid_custom_icon(&uri(&bytes[..end])));
    }
}
#[test]
fn cc_base_icon_png_rejects_crc_corruption() {
    let mut bytes = png(16, 16);
    bytes[29] ^= 1;
    assert!(!is_valid_custom_icon(&uri(&bytes)));
}
#[test]
fn cc_base_icon_png_rejects_svg_and_invalid_encoding() {
    for input in ["data:image/svg+xml;base64,PHN2Zy8+", "data:image/png;base64,!!!", "", "builtin"]
    {
        assert!(!is_valid_custom_icon(input));
    }
    assert!(!is_valid_custom_icon(&uri(b"<svg/>")));
}
#[test]
fn cc_base_icon_png_rejects_overlong_uri() {
    let mut input = uri(&png(1, 1));
    input.extend(std::iter::repeat_n('A', 65537 - input.len()));
    assert!(!is_valid_custom_icon(&input));
}
#[test]
fn cc_base_icon_png_rejects_oversized_dimensions() {
    for (width, height) in [(257, 1), (1, 257)] {
        assert!(!is_valid_custom_icon(&uri(&png(width, height))));
    }
}
