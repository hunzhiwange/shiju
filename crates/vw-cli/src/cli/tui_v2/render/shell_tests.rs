use super::shell::truncate_shell_output;

#[test]
fn shell_output_truncation_respects_line_and_byte_limits() {
    let output = truncate_shell_output("a\nb\nc", 2, 100);
    assert_eq!(output.visible_lines, vec!["a", "b"]);
    assert_eq!(output.omitted_lines, 1);

    let output = truncate_shell_output("界界界", 10, 4);
    assert_eq!(output.visible_lines, vec!["界"]);
    assert!(output.omitted_bytes > 0);
}
