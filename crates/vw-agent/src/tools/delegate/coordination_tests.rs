use super::text_preview;

#[test]
fn text_preview_maps_blank_input_to_explicit_placeholder() {
    assert_eq!(text_preview(" \n\t ", 8), "[empty]");
}

#[test]
fn text_preview_truncates_on_unicode_character_boundary() {
    assert_eq!(text_preview(" 甲乙丙丁 ", 3), "甲乙丙...");
}

#[test]
fn text_preview_preserves_trimmed_value_at_limit() {
    assert_eq!(text_preview(" 甲乙丙 ", 3), "甲乙丙");
}
