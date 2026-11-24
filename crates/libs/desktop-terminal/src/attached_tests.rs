use super::*;

#[test]
fn cli_workbench_attach_resumes_a_split_utf8_sequence_and_rejects_stale_output() {
    let mut terminal = AttachedTerminal::new("/tmp".into(), &Theme::Light, true);
    let input = "中文".as_bytes();
    assert!(terminal.apply_output(0, 0, 2, &input[..2], false, true));
    assert!(!terminal.apply_output(0, 0, 2, &input[..2], false, true));
    assert!(terminal.apply_output(2, 2, 6, &input[2..], false, true));
    assert_eq!(terminal.cursor(), 6);
    assert!(terminal.terminal.text().iter().any(|line| line.contains("中文")));
    assert!(terminal.take_input().0.is_empty());
}

#[test]
fn cli_workbench_attach_resets_after_a_replay_gap_and_drops_historical_replies() {
    let mut terminal = AttachedTerminal::new("/tmp".into(), &Theme::Dark, false);
    assert!(terminal.apply_output(0, 0, 6, b"old\x1b[6", false, true));
    assert!(!terminal.apply_output(6, 20, 25, b"new\x1b[", false, true));
    assert!(terminal.apply_output(6, 20, 29, b"new\x1b[5n\r\n", true, true));
    assert!(terminal.take_input().0.is_empty());
    assert!(!terminal.terminal.text().iter().any(|line| line.contains("old")));
    assert!(terminal.terminal.text().iter().any(|line| line.contains("new")));
    assert!(terminal.apply_output(29, 29, 33, b"\x1b[5n", false, true));
    assert!(!terminal.take_input().0.is_empty());
}

#[test]
fn cli_workbench_attach_keeps_user_input_and_reports_measured_size() {
    use iced_term::BackendCommand;

    let mut terminal = AttachedTerminal::new("/tmp".into(), &Theme::Light, true);
    let id = terminal.terminal.id;
    terminal.handle(Event::BackendCall(id + 1, BackendCommand::Write(b"wrong tab".to_vec())));
    terminal.handle(Event::BackendCall(
        id,
        BackendCommand::Resize(Some(iced::Size::new(600.0, 320.0)), None),
    ));
    terminal.handle(Event::BackendCall(id, BackendCommand::Write(b"user input".to_vec())));
    assert!(terminal.apply_output(0, 0, 3, b"out", false, true));
    let (bytes, size, overflow) = terminal.take_input();
    assert_eq!(bytes.as_slice(), b"user input");
    let (rows, cols) = size.unwrap();
    assert!(rows > 0 && cols >= 2);
    assert!(!overflow);
    assert_eq!(terminal.take_input(), (Vec::new(), None, false));
}
