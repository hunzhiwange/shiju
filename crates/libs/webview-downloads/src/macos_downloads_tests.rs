use super::is_attachment;

#[test]
fn content_disposition_recognizes_attachment_with_optional_parameters() {
    assert!(is_attachment("attachment"));
    assert!(is_attachment(" Attachment ; filename=\"report.pdf\""));
    assert!(!is_attachment("inline; filename=attachment"));
    assert!(!is_attachment("attachment-preview"));
}
