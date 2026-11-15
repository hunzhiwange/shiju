//! 验证十六进制颜色的格式检查、RGB 转换与终端样式生成。
//! 无效前缀、缺失值和非法数字必须返回失败而非生成错误颜色。

use super::color::{hex_to_ansi_bold, hex_to_rgb, is_valid_hex};

#[test]
fn validates_and_converts_hex_colors() {
    assert!(is_valid_hex(Some("#00aAFF")));
    assert!(!is_valid_hex(Some("00aAFF")));
    assert!(!is_valid_hex(None));
    assert_eq!(hex_to_rgb("#ff0000"), Some((255, 0, 0)));
    assert_eq!(hex_to_rgb("#gg0000"), None);
    assert!(hex_to_ansi_bold(Some("#0000ff")).expect("ansi").contains("0;0;255"));
}
