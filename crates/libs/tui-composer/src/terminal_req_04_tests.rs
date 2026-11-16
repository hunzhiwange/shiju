use super::*;

fn submitted(composer: &mut ChatComposer, queue: bool) -> String {
    match composer.submit_or_queue(queue) {
        V3Action::Submit { text, .. } if !queue => text,
        V3Action::Queue { text, .. } if queue => text,
        action => panic!("unexpected action: {action:?}"),
    }
}

#[test]
fn terminal_req_04_threshold_counts_characters() {
    let mut composer = ChatComposer::new();
    let text = "中".repeat(1000);
    composer.handle_paste(text.clone());
    assert_eq!(composer.text(), text);
    assert_eq!(submitted(&mut composer, false), text);
}

#[test]
fn terminal_req_04_above_threshold_folds_unicode() {
    let mut composer = ChatComposer::new();
    let text = "😀".repeat(1001);
    composer.handle_paste(text.clone());
    assert_eq!(composer.text(), "[Pasted text: 1001 chars]");
    assert_eq!(submitted(&mut composer, false), text);
}

#[test]
fn terminal_req_04_insertion_preserves_middle_position() {
    let mut composer = ChatComposer::new();
    composer.replace_text("before after");
    composer.textarea.set_cursor(7);
    let text = "x".repeat(1001);
    composer.handle_paste(text.clone());
    assert_eq!(submitted(&mut composer, false), format!("before {text}after"));
}

#[test]
fn terminal_req_04_multiple_pastes_queue_in_order() {
    let mut composer = ChatComposer::new();
    let first = "a".repeat(1001);
    let second = "b".repeat(1002);
    composer.handle_paste(first.clone());
    composer.textarea.insert_str(" / ");
    composer.handle_paste(second.clone());
    assert_eq!(submitted(&mut composer, true), format!("{first} / {second}"));
}

#[test]
fn terminal_req_04_whitespace_is_preserved() {
    let mut composer = ChatComposer::new();
    let text = format!(" \n{}\r\n\t ", "x".repeat(1001));
    composer.handle_paste(text.clone());
    assert_eq!(submitted(&mut composer, false), text);
}

#[test]
fn terminal_req_04_backspace_deletes_whole_paste() {
    let mut composer = ChatComposer::new();
    composer.replace_text("keep");
    composer.handle_paste("x".repeat(1001));
    composer.textarea.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(submitted(&mut composer, false), "keep");
}

#[test]
fn terminal_req_04_kill_and_yank_restore_original() {
    let mut composer = ChatComposer::new();
    let text = "x".repeat(1001);
    composer.handle_paste(text.clone());
    composer.textarea.kill_to_beginning_of_line();
    assert!(composer.text().is_empty());
    composer.textarea.yank();
    assert_eq!(submitted(&mut composer, false), text);
}

#[test]
fn terminal_req_04_cancel_discards_payload() {
    let mut composer = ChatComposer::new();
    composer.handle_paste("x".repeat(1001));
    assert!(matches!(
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        V3Action::Cancel
    ));
    assert!(matches!(composer.submit_or_queue(false), V3Action::None));
    composer.replace_text("next");
    assert_eq!(submitted(&mut composer, false), "next");
}

#[test]
fn terminal_req_04_literal_placeholder_is_not_expanded() {
    let mut composer = ChatComposer::new();
    composer.handle_paste("x".repeat(1001));
    composer.replace_text("[Pasted text: 1001 chars]");
    assert_eq!(submitted(&mut composer, false), "[Pasted text: 1001 chars]");
}

#[test]
fn terminal_req_04_empty_paste_does_not_submit() {
    let mut composer = ChatComposer::new();
    composer.handle_paste(String::new());
    assert!(matches!(composer.submit_or_queue(false), V3Action::None));
}
