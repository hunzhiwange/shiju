use super::is_word_byte;
#[test]
fn word_byte_accepts_ascii_identifier_bytes_only() {
    for byte in [b'a', b'Z', b'0', b'_'] {
        assert!(is_word_byte(byte));
    }
    for byte in [b'-', b' ', b'.', 0xFF] {
        assert!(!is_word_byte(byte));
    }
}
