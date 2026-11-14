use super::*;
use pretty_assertions::assert_eq;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

fn tty_with_buffered_input(input: &[u8]) -> Tty {
    let (reader, mut writer) = UnixStream::pair().expect("create terminal input pair");
    writer.write_all(input).expect("buffer terminal input");

    let reader: OwnedFd = reader.into();
    let writer: OwnedFd = writer.into();
    Tty::new(reader.into(), writer.into()).expect("create terminal probe")
}

#[test]
fn terminal_probe_stops_reading_when_its_deadline_expires() {
    let mut tty = tty_with_buffered_input(b"still queued");
    let mut buffer = Vec::new();

    tty.read_available(&mut buffer, Instant::now(), MAX_TERMINAL_PROBE_BYTES)
        .expect("respect terminal probe deadline");

    assert_eq!(buffer, Vec::<u8>::new());
    let mut queued = [0; 12];
    let count = unsafe {
        libc::read(
            tty.reader.as_raw_fd(),
            queued.as_mut_ptr().cast::<libc::c_void>(),
            queued.len(),
        )
    };
    assert_eq!(count, queued.len() as isize);
    assert_eq!(&queued, b"still queued");
}

#[test]
fn terminal_probe_leaves_input_queued_after_reaching_its_byte_budget() {
    for max_bytes in [MAX_TERMINAL_PROBE_BYTES, MAX_STARTUP_PROBE_BYTES] {
        let input = [b'x'; 256];
        let mut tty = tty_with_buffered_input(&input);
        let mut buffer = vec![b'x'; max_bytes - 32];

        tty.read_available(
            &mut buffer,
            Instant::now() + Duration::from_secs(/*secs*/ 1),
            max_bytes,
        )
        .expect("respect terminal probe byte budget");

        assert_eq!(buffer.len(), max_bytes);
        let mut queued = [0; 224];
        let count = unsafe {
            libc::read(
                tty.reader.as_raw_fd(),
                queued.as_mut_ptr().cast::<libc::c_void>(),
                queued.len(),
            )
        };
        assert_eq!(count, queued.len() as isize);
        assert_eq!(queued, [b'x'; 224]);
    }
}

#[test]
fn parses_cursor_position_as_zero_based() {
    assert_eq!(parse_cursor_position(b"\x1B[20;10R"), Some(Position { x: 9, y: 19 }));
    assert_eq!(parse_cursor_position(b"\x1B[I\x1B[20;10R"), Some(Position { x: 9, y: 19 }));
}

#[test]
fn parses_keyboard_enhancement_flags_and_pda_fallback() {
    assert_eq!(
        parse_keyboard_enhancement_support(b"\x1B[?7u"),
        KeyboardProbeState::Supported
    );
    assert_eq!(
        parse_keyboard_enhancement_support(b"\x1B[?64;1;2c"),
        KeyboardProbeState::UnsupportedFallback
    );
    assert_eq!(
        parse_keyboard_enhancement_support(b"\x1B[?64;1;2c\x1B[?7u"),
        KeyboardProbeState::SupportedAndFallback
    );
    assert_eq!(
        parse_keyboard_enhancement_support(b"\x1B[?7u\x1B[?64;1;2c"),
        KeyboardProbeState::SupportedAndFallback
    );
    assert_eq!(parse_keyboard_enhancement_support(b""), KeyboardProbeState::Pending);
}

#[test]
fn startup_probe_parses_batched_terminal_responses() {
    let mut probe = StartupProbe {
        cursor_position: None,
        default_colors: None,
        keyboard_enhancement_supported: None,
    };
    let mut saw_supported_keyboard = false;
    update_startup_probe(
        &mut probe,
        &mut saw_supported_keyboard,
        b"\x1B[20;10R\x1B]11;rgb:1111/1111/1111\x07\x1B[?64;1;2c\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B[?7u",
        StartupKeyboardEnhancementProbe::Query,
    );

    assert_eq!(
        probe,
        StartupProbe {
            cursor_position: Some(Position { x: 9, y: 19 }),
            default_colors: Some(DefaultColors { fg: (238, 238, 238), bg: (17, 17, 17) }),
            keyboard_enhancement_supported: Some(true),
        }
    );
    assert!(startup_probe_complete(&probe, StartupKeyboardEnhancementProbe::Query));
}

#[test]
fn startup_probe_finds_responses_after_large_typeahead() {
    let mut input = vec![b'x'; 2_048];
    input.extend_from_slice(
        b"\x1B[20;10R\x1B]11;rgb:1111/1111/1111\x07\x1B[?64;1;2c\x1B]10;rgb:eeee/eeee/eeee\x1B\\\x1B[?7u",
    );
    let mut tty = tty_with_buffered_input(&input);
    let mut buffer = Vec::new();

    let probe = read_startup_probe(
        &mut tty,
        Duration::from_secs(/*secs*/ 1),
        StartupKeyboardEnhancementProbe::Query,
        &mut buffer,
    )
    .expect("read terminal responses after buffered typeahead");

    assert_eq!(
        probe,
        StartupProbe {
            cursor_position: Some(Position { x: 9, y: 19 }),
            default_colors: Some(DefaultColors { fg: (238, 238, 238), bg: (17, 17, 17) }),
            keyboard_enhancement_supported: Some(true),
        }
    );
    let mut expected_replay = vec![b'x'; 2_048];
    expected_replay.extend_from_slice(b"\x1B[20;10R\x1B[?64;1;2c\x1B[?7u");
    assert_eq!(startup_replay_input(&buffer), expected_replay);
}

#[test]
fn startup_probe_finds_valid_responses_after_many_incomplete_sequences() {
    let mut input = Vec::new();
    for _ in 0..2_048 {
        input.extend_from_slice(b"\x1b[?");
    }
    input.extend_from_slice(b"\x1b[20;10R\x1b[?64;1;2c\x1b[?7u");

    assert_eq!(parse_cursor_position(&input), Some(Position { x: 9, y: 19 }));
    assert_eq!(
        parse_keyboard_enhancement_support(&input),
        KeyboardProbeState::SupportedAndFallback
    );
}
