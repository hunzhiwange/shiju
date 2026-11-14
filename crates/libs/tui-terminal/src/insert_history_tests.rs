use super::*;

#[test]
fn writes_bold_then_regular_spans() {
    use ratatui::style::Stylize;

    let spans = ["A".bold(), "B".into()];

    let mut actual: Vec<u8> = Vec::new();
    write_spans(&mut actual, spans.iter()).unwrap();

    let mut expected: Vec<u8> = Vec::new();
    queue!(
        expected,
        SetAttribute(crossterm::style::Attribute::Bold),
        Print("A"),
        SetAttribute(crossterm::style::Attribute::NormalIntensity),
        Print("B"),
        SetForegroundColor(CColor::Reset),
        SetBackgroundColor(CColor::Reset),
        SetAttribute(crossterm::style::Attribute::Reset),
    )
    .unwrap();

    assert_eq!(String::from_utf8(actual).unwrap(), String::from_utf8(expected).unwrap());
}

#[test]
fn writes_semantic_web_link_without_changing_visible_text() {
    let destination = "https://example.com/long/path";
    let line = tui_render::terminal_hyperlinks::annotate_web_urls_in_line(Line::from(destination));
    let mut actual = Vec::new();

    write_history_line(&mut actual, &line, /*wrap_width*/ 80).expect("write history line");

    let output = String::from_utf8(actual).expect("UTF-8 terminal output");
    assert!(output.contains("\x1b]8;;https://example.com/long/path\x07"));
    assert_eq!(line.line.spans[0].content, destination);
}
