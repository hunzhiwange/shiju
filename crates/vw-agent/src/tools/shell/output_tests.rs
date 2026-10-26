use super::output::*;

#[test]
fn shell_output_poll_returns_incremental_chunks() {
    let mut output = OutputBuffer::default();
    output.push(b"first");
    let first = output.read(0).unwrap();
    output.push(b"second");
    assert_eq!(output.read(first.next_cursor).unwrap().output, "second");
    assert_eq!(output.read(0).unwrap().output, "firstsecond");
    assert!(output.read(u64::MAX).is_err());
}

#[test]
fn shell_output_overflow_reports_gap() {
    let mut output = OutputBuffer::default();
    output.push(&vec![b'x'; OUTPUT_LIMIT * 2]);
    let read = output.read(0).unwrap();
    assert!(read.truncated);
    assert!(read.dropped_bytes > 0);
    assert!(read.output.len() <= OUTPUT_LIMIT);
    assert!(output.retained_bytes() <= OUTPUT_LIMIT);
    assert_eq!(output.read(read.next_cursor).unwrap().output, "");
}

#[test]
fn shell_output_preserves_split_utf8() {
    let mut output = OutputBuffer::default();
    let mut decoder = OutputDecoder::default();
    decoder.push(&[0xe4, 0xb8], false, &mut output);
    assert_eq!(output.read(0).unwrap().output, "");
    decoder.push(&[0xad, 0xff, 0xe4], false, &mut output);
    decoder.push(&[], true, &mut output);
    assert_eq!(output.read(0).unwrap().output, "中��");
}
