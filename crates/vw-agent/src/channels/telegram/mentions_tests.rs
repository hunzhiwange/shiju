use super::TelegramChannel;

#[test]
fn mention_spans_require_username_boundaries() {
    let spans = TelegramChannel::find_bot_mention_spans("hi @VibeBot and no@vibebot", "vibebot");

    assert_eq!(spans, vec![(3, 11)]);
    assert!(TelegramChannel::contains_bot_mention("hi @vibebot", "@VibeBot"));
}

#[test]
fn telegram_username_char_accepts_only_ascii_username_chars() {
    assert!(TelegramChannel::is_telegram_username_char('a'));
    assert!(TelegramChannel::is_telegram_username_char('7'));
    assert!(TelegramChannel::is_telegram_username_char('_'));
    assert!(!TelegramChannel::is_telegram_username_char('-'));
    assert!(!TelegramChannel::is_telegram_username_char('中'));
}

#[test]
fn mention_spans_ignore_bare_at_signs() {
    assert!(TelegramChannel::find_bot_mention_spans("@ @! @中", "bot").is_empty());
    assert!(!TelegramChannel::contains_bot_mention("@ @! @中", "bot"));
}

#[test]
fn mention_spans_allow_non_username_character_before_at() {
    let spans = TelegramChannel::find_bot_mention_spans("中@bot (@BOT)", "bot");

    assert_eq!(spans, vec![(3, 7), (9, 13)]);
}

#[test]
fn normalize_incoming_content_removes_mentions_and_collapses_space() {
    assert_eq!(
        TelegramChannel::normalize_incoming_content(" @bot   run  this ", "bot").as_deref(),
        Some("run this")
    );
    assert_eq!(TelegramChannel::normalize_incoming_content("@bot", "bot"), None);
}

#[test]
fn mention_spans_ignore_empty_username_and_suffix_matches() {
    assert!(
        TelegramChannel::find_bot_mention_spans("hi @bot", "").is_empty(),
        "empty bot username should never match"
    );
    assert!(
        TelegramChannel::find_bot_mention_spans("hi @bot_suffix", "bot").is_empty(),
        "a longer username should not match the bot mention"
    );
    assert!(!TelegramChannel::contains_bot_mention("hi @bot_suffix", "bot"));
}

#[test]
fn mention_spans_match_multiple_mentions_case_insensitively() {
    let spans = TelegramChannel::find_bot_mention_spans("@Bot, ping @bot and @BOT", "@bot");

    assert_eq!(spans, vec![(0, 4), (11, 15), (20, 24)]);
}

#[test]
fn normalize_incoming_content_without_mentions_still_collapses_whitespace() {
    assert_eq!(
        TelegramChannel::normalize_incoming_content("  keep \n  spacing\t tidy  ", "bot")
            .as_deref(),
        Some("keep spacing tidy")
    );
}

#[test]
fn normalize_incoming_content_removes_multiple_mentions_and_returns_none_for_blank_text() {
    assert_eq!(
        TelegramChannel::normalize_incoming_content("@bot hi @BOT there", "bot").as_deref(),
        Some("hi there")
    );
    assert_eq!(TelegramChannel::normalize_incoming_content(" \n\t ", "bot"), None);
}
