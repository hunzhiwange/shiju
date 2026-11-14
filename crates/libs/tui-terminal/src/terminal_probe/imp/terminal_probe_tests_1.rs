use super::*;
use pretty_assertions::assert_eq;
use windows_sys::Win32::System::Console::COMMON_LVB_REVERSE_VIDEO;

fn color_table() -> [u32; 16] {
    [
        0x00000000, 0x00000080, 0x00008000, 0x00008080, 0x00800000, 0x00800080, 0x00808000,
        0x00c0c0c0, 0x00808080, 0x000000ff, 0x0000ff00, 0x0000ffff, 0x00ff0000, 0x00ff00ff,
        0x00ffff00, 0x00ffffff,
    ]
}

#[test]
fn decodes_console_color_attribute_indices() {
    assert_eq!(
        decode_console_default_colors(/*attributes*/ 0x21, &color_table()),
        DefaultColors { fg: (128, 0, 0), bg: (0, 128, 0) }
    );
}

#[test]
fn decodes_console_color_intensity_indices() {
    assert_eq!(
        decode_console_default_colors(/*attributes*/ 0xe9, &color_table()),
        DefaultColors { fg: (255, 0, 0), bg: (0, 255, 255) }
    );
}

#[test]
fn decodes_console_color_ref_byte_order() {
    let mut colors = color_table();
    colors[3] = 0x00112233;
    colors[4] = 0x00aabbcc;

    assert_eq!(
        decode_console_default_colors(/*attributes*/ 0x43, &colors),
        DefaultColors { fg: (0x33, 0x22, 0x11), bg: (0xcc, 0xbb, 0xaa) }
    );
}

#[test]
fn ignores_reverse_video_when_decoding_default_colors() {
    assert_eq!(
        decode_console_default_colors(
            /*attributes*/ COMMON_LVB_REVERSE_VIDEO | 0x21,
            &color_table(),
        ),
        DefaultColors { fg: (128, 0, 0), bg: (0, 128, 0) }
    );
}
