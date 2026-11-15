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

#[test]
fn shell_output_decodes_powershell_utf16_diagnostics_at_every_split() {
    let message = "Windows PowerShell 内部错误：加载托管运行时失败 8009001d。🙂\r\n";
    for little_endian in [true, false] {
        for with_bom in [true, false] {
            let mut bytes = Vec::new();
            if with_bom {
                bytes.extend_from_slice(if little_endian { &[0xff, 0xfe] } else { &[0xfe, 0xff] });
            }
            for unit in message.encode_utf16() {
                bytes.extend_from_slice(&if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            for split in 0..=bytes.len() {
                let mut output = OutputBuffer::default();
                let mut decoder = OutputDecoder::new(true);
                decoder.push(&bytes[..split], false, &mut output);
                decoder.push(&bytes[split..], true, &mut output);
                assert_eq!(output.read(0).unwrap().output, message, "split {split}");
            }
            let mut output = OutputBuffer::default();
            let mut decoder = OutputDecoder::new(true);
            for byte in &bytes {
                decoder.push(&[*byte], false, &mut output);
            }
            decoder.push(&[], true, &mut output);
            assert_eq!(output.read(0).unwrap().output, message);
        }
    }
}

#[test]
fn shell_output_replaces_incomplete_utf16_at_eof() {
    let mut output = OutputBuffer::default();
    let mut decoder = OutputDecoder::new(true);
    decoder.push(&[0xff, 0xfe, b'A', 0, 0x3d, 0xd8], false, &mut output);
    assert_eq!(output.read(0).unwrap().output, "A");
    decoder.push(&[0], true, &mut output);
    assert_eq!(output.read(0).unwrap().output, "A��");
}

#[test]
fn shell_output_preserves_utf8_chinese_and_emoji_at_every_split() {
    let bytes = "中文输出🙂\n".as_bytes();
    for split in 0..=bytes.len() {
        let mut output = OutputBuffer::default();
        let mut decoder = OutputDecoder::new(true);
        decoder.push(&bytes[..split], false, &mut output);
        decoder.push(&bytes[split..], true, &mut output);
        assert_eq!(output.read(0).unwrap().output, "中文输出🙂\n");
    }
}
