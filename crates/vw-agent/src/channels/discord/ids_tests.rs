use super::*;

#[test]
fn base64_decode_accepts_unpadded_discord_token_prefix() {
    assert_eq!(base64_decode("MTIzNDU2"), Some("123456".to_string()));
    assert_eq!(bot_user_id_from_token("MTIzNDU2.timestamp.signature"), Some("123456".to_string()));
}

#[test]
fn base64_decode_rejects_invalid_or_non_utf8_input() {
    assert_eq!(base64_decode("not valid!"), None);
    assert_eq!(base64_decode("//8="), None);
}

#[test]
fn base64_decode_handles_padding_variants_and_empty_chunks() {
    assert_eq!(base64_decode("TWE="), Some("Ma".to_string()));
    assert_eq!(base64_decode("TWE"), Some("Ma".to_string()));
    assert_eq!(base64_decode("TQ"), Some("M".to_string()));
    assert_eq!(base64_decode(""), Some(String::new()));
    assert_eq!(base64_decode("A"), Some(String::new()));
}

#[test]
fn base64_decode_handles_binary_byte_boundaries() {
    assert_eq!(base64_decode("AA=="), Some("\0".to_string()));
    assert_eq!(base64_decode("AAA="), Some("\0\0".to_string()));
    assert_eq!(base64_decode("AAAA"), Some("\0\0\0".to_string()));
}

#[test]
fn bot_user_id_from_token_uses_first_segment_only() {
    assert_eq!(bot_user_id_from_token("TWE=.ignored.ignored"), Some("Ma".to_string()));
    assert_eq!(bot_user_id_from_token("bad!.ignored"), None);
}
