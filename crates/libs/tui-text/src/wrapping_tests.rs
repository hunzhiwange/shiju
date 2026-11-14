    use super::*;
    use itertools::Itertools as _;
    use pretty_assertions::assert_eq;
    use ratatui::style::Color;
    use ratatui::style::Stylize;
    use std::string::ToString;

    fn concat_line(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
    }

    #[test]
    fn trivial_unstyled_no_indents_wide_width() {
        let line = Line::from("hello");
        let out = word_wrap_line(&line, /*width_or_options*/ 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "hello");
    }

    #[test]
    fn simple_unstyled_wrap_narrow_width() {
        let line = Line::from("hello world");
        let out = word_wrap_line(&line, /*width_or_options*/ 5);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn simple_styled_wrap_preserves_styles() {
        let line = Line::from(vec!["hello ".red(), "world".into()]);
        let out = word_wrap_line(&line, /*width_or_options*/ 6);
        assert_eq!(out.len(), 2);
        // 第一行应保留红色样式。
        // First line should carry the red style
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        // 第二行不带样式。
        // Second line is unstyled
        assert_eq!(concat_line(&out[1]), "world");
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[1].spans[0].style.fg, None);
    }

    #[test]
    fn with_initial_and_subsequent_indents() {
        let opts = RtOptions::new(/*width*/ 8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));
        let line = Line::from("hello world foo");
        let out = word_wrap_line(&line, opts);
        // 期望三行均带正确前缀。
        // Expect three lines with proper prefixes
        assert!(concat_line(&out[0]).starts_with("- "));
        assert!(concat_line(&out[1]).starts_with("  "));
        assert!(concat_line(&out[2]).starts_with("  "));
        // 同时正文应按预期大致分段。
        // And content roughly segmented
        assert_eq!(concat_line(&out[0]), "- hello");
        assert_eq!(concat_line(&out[1]), "  world");
        assert_eq!(concat_line(&out[2]), "  foo");
    }

    #[test]
    fn empty_initial_indent_subsequent_spaces() {
        let opts = RtOptions::new(/*width*/ 8)
            .initial_indent(Line::from(""))
            .subsequent_indent(Line::from("    "));
        let line = Line::from("hello world foobar");
        let out = word_wrap_line(&line, opts);
        assert!(concat_line(&out[0]).starts_with("hello"));
        for l in &out[1..] {
            assert!(concat_line(l).starts_with("    "));
        }
    }

    #[test]
    fn empty_input_yields_single_empty_line() {
        let line = Line::from("");
        let out = word_wrap_line(&line, /*width_or_options*/ 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "");
    }

    #[test]
    fn leading_spaces_preserved_on_first_line() {
        let line = Line::from("   hello");
        let out = word_wrap_line(&line, /*width_or_options*/ 8);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "   hello");
    }

    #[test]
    fn multiple_spaces_between_words_dont_start_next_line_with_spaces() {
        let line = Line::from("hello   world");
        let out = word_wrap_line(&line, /*width_or_options*/ 8);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn break_words_false_allows_overflow_for_long_word() {
        let opts = RtOptions::new(/*width*/ 5).break_words(/*break_words*/ false);
        let line = Line::from("supercalifragilistic");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "supercalifragilistic");
    }

    #[test]
    fn hyphen_splitter_breaks_at_hyphen() {
        let line = Line::from("hello-world");
        let out = word_wrap_line(&line, /*width_or_options*/ 7);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello-");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn indent_consumes_width_leaving_one_char_space() {
        let opts = RtOptions::new(/*width*/ 4)
            .initial_indent(Line::from(">>>>"))
            .subsequent_indent(Line::from("--"));
        let line = Line::from("hello");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 3);
        assert_eq!(concat_line(&out[0]), ">>>>h");
        assert_eq!(concat_line(&out[1]), "--el");
        assert_eq!(concat_line(&out[2]), "--lo");
    }

    #[test]
    fn wide_unicode_wraps_by_display_width() {
        let line = Line::from("😀😀😀");
        let out = word_wrap_line(&line, /*width_or_options*/ 4);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "😀😀");
        assert_eq!(concat_line(&out[1]), "😀");
    }

    #[test]
    fn styled_split_within_span_preserves_style() {
        use ratatui::style::Stylize;
        let line = Line::from(vec!["abcd".red()]);
        let out = word_wrap_line(&line, /*width_or_options*/ 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(out[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(concat_line(&out[0]), "ab");
        assert_eq!(concat_line(&out[1]), "cd");
    }

    #[test]
    fn wrap_lines_applies_initial_indent_only_once() {
        let opts = RtOptions::new(/*width*/ 8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));

        let lines = vec![Line::from("hello world"), Line::from("foo bar baz")];
        let out = word_wrap_lines(lines, opts);

        // 首行前缀为 `- `，后续包装片段为两个空格；第二条输入行的首片段不得再使用 `- `。
        // Expect: first line prefixed with "- ", subsequent wrapped pieces with "  "
        // and for the second input line, there should be no "- " prefix on its first piece
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert!(rendered[0].starts_with("- "));
        for r in rendered.iter().skip(1) {
            assert!(r.starts_with("  "));
        }
    }

    #[test]
    fn wrap_lines_without_indents_is_concat_of_single_wraps() {
        let lines = vec![Line::from("hello"), Line::from("world!")];
        let out = word_wrap_lines(lines, /*width_or_options*/ 10);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello", "world!"]);
    }

    #[test]
    fn wrap_lines_accepts_borrowed_iterators() {
        let lines = [Line::from("hello world"), Line::from("foo bar baz")];
        let out = word_wrap_lines(lines, /*width_or_options*/ 10);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello", "world", "foo bar", "baz"]);
    }

    #[test]
    fn wrap_lines_accepts_str_slices() {
        let lines = ["hello world", "goodnight moon"];
        let out = word_wrap_lines(lines, /*width_or_options*/ 12);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello world", "goodnight", "moon"]);
    }

    #[test]
    fn line_height_counts_double_width_emoji() {
        let line = "😀😀😀".into(); // each emoji ~ width 2
        assert_eq!(word_wrap_line(&line, /*width_or_options*/ 4).len(), 2);
        assert_eq!(word_wrap_line(&line, /*width_or_options*/ 2).len(), 3);
        assert_eq!(word_wrap_line(&line, /*width_or_options*/ 6).len(), 1);
    }

    #[test]
    fn word_wrap_does_not_split_words_simple_english() {
        let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira’s herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
        let line = Line::from(sample);
        let lines = [line];
        // 使用小宽度强制覆盖空格处换行。
        // Force small width to exercise wrapping at spaces.
        let wrapped = word_wrap_lines(&lines, /*width_or_options*/ 40);
        let joined: String = wrapped.iter().map(ToString::to_string).join("\n");
        assert_eq!(
            joined,
            r#"Years passed, and Willowmere thrived in
peace and friendship. Mira’s herb garden
flourished with both ordinary and
enchanted plants, and travelers spoke of
the kindness of the woman who tended
them."#
        );
    }

    #[test]
    fn ascii_space_separator_with_no_hyphenation_keeps_url_intact() {
        let line = Line::from(
            "http://example.com/long-url-with-dashes-wider-than-terminal-window/blah-blah-blah-text/more-gibberish-text",
        );
        let opts = RtOptions::new(/*width*/ 24)
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .word_splitter(textwrap::WordSplitter::NoHyphenation)
            .break_words(/*break_words*/ false);

        let out = word_wrap_line(&line, opts);

        assert_eq!(out.len(), 1);
        assert_eq!(
            concat_line(&out[0]),
            "http://example.com/long-url-with-dashes-wider-than-terminal-window/blah-blah-blah-text/more-gibberish-text"
        );
    }

    #[test]
    fn text_contains_url_like_matches_expected_tokens() {
        let positives = [
            "https://example.com/a/b",
            "ftp://host/path",
            "www.example.com/path?x=1",
            "example.test/path#frag",
            "localhost:3000/api",
            "127.0.0.1:8080/health",
            "(https://example.com/wrapped-in-parens)",
        ];

        for text in positives {
            assert!(text_contains_url_like(text), "expected URL-like match for {text:?}");
        }
    }

    #[test]
    fn text_contains_url_like_rejects_non_urls() {
        let negatives = [
            "src/main.rs",
            "foo/bar",
            "key:value",
            "just-some-text-with-dashes",
            "hello.world", // no path/query/fragment and no www
        ];

        for text in negatives {
            assert!(!text_contains_url_like(text), "did not expect URL-like match for {text:?}");
        }
    }

    #[test]
    fn line_contains_url_like_checks_across_spans() {
        let line = Line::from(vec![
            "see ".into(),
            "https://example.com/a/very/long/path".cyan(),
            " for details".into(),
        ]);

        assert!(line_contains_url_like(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_detects_prose_plus_url() {
        let line = Line::from("see https://example.com/path for details");
        assert!(line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_ignores_pipe_prefix() {
        let line = Line::from(vec!["  │ ".into(), "https://example.com/path".into()]);
        assert!(!line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_ignores_ordered_list_marker() {
        let line = Line::from("1. https://example.com/path");
        assert!(!line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn text_contains_url_like_accepts_custom_scheme_with_separator() {
        assert!(text_contains_url_like("myapp://open/some/path"));
    }

    #[test]
    fn text_contains_url_like_rejects_invalid_ports() {
        assert!(!text_contains_url_like("localhost:99999/path"));
        assert!(!text_contains_url_like("example.com:abc/path"));
    }

    #[test]
    fn adaptive_wrap_line_keeps_long_url_like_token_intact() {
        let line = Line::from("example.test/a-very-long-path-with-many-segments-and-query?x=1&y=2");
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 20));
        assert_eq!(out.len(), 1);
        assert_eq!(
            concat_line(&out[0]),
            "example.test/a-very-long-path-with-many-segments-and-query?x=1&y=2"
        );
    }

    #[test]
    fn adaptive_wrap_line_preserves_default_behavior_for_non_url_tokens() {
        let line = Line::from("a_very_long_token_without_spaces_to_force_wrapping");
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 20));
        assert!(out.len() > 1, "expected non-url token to wrap with default options");
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_keeps_regular_words_intact() {
        let line = Line::from(
            "see https://example.com/path and keep strikethrough intact while wrapping prose",
        );
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 36));
        let joined = out.iter().map(concat_line).join("\n");

        assert_eq!(
            joined,
            "see https://example.com/path and\nkeep strikethrough intact while\nwrapping prose"
        );
    }

    #[test]
    fn adaptive_wrap_line_keeps_url_split_across_styled_spans_intact() {
        let line = Line::from(vec![
            "see ".red(),
            "https://exa".cyan(),
            "mple.com/path".magenta(),
            " now".green(),
        ]);
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 10));

        assert_eq!(
            out,
            vec![
                Line::from("see".red()),
                Line::from(vec!["https://exa".cyan(), "mple.com/path".magenta()]),
                Line::from("now".green()),
            ]
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_wraps_long_non_url_token() {
        let long_non_url = "a_very_long_token_without_spaces_to_force_wrapping";
        let line = Line::from(format!("see https://ex.com {long_non_url}"));
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 24));

        assert!(
            out.iter().any(|line| concat_line(line).contains("https://ex.com")),
            "expected URL token to remain present, got: {out:?}"
        );
        assert!(
            !out.iter().any(|line| concat_line(line).contains(long_non_url)),
            "expected long non-url token to wrap on mixed lines, got: {out:?}"
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word() {
        let line = Line::from("      abcdefgh https://x.co");
        let out = adaptive_wrap_line(
            &line,
            RtOptions::new(/*width*/ 10).subsequent_indent("      ".into()),
        );
        let rendered = out.iter().map(concat_line).collect_vec();

        assert_eq!(rendered[..2], ["      abcd".to_string(), "      efgh".to_string()]);
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width() {
        let line = Line::from("abcdefghijklmnopqrst https://x.co");
        let out = adaptive_wrap_line(
            &line,
            RtOptions::new(/*width*/ 10).subsequent_indent("    ".into()),
        );
        let rendered = out.iter().map(concat_line).collect_vec();

        assert_eq!(
            rendered[..3],
            ["abcdefghij".to_string(), "    klmnop".to_string(), "    qrst".to_string(),]
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks() {
        let line = Line::from("ｶﾞﾊﾟtail https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 4));
        let rendered = out.iter().map(concat_line).collect_vec();

        assert_eq!(rendered, ["ｶﾞﾊﾟ", "tail", "https://x.co"]);
    }

    #[test]
    fn adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme() {
        let line = Line::from("ｶﾞ https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(/*width*/ 1));
        let rendered = out.iter().map(concat_line).collect_vec();

        assert_eq!(rendered, ["ｶﾞ", "https://x.co"]);
    }

    #[test]
    fn map_owned_wrapped_line_to_range_recovers_on_non_prefix_mismatch() {
        // 先匹配源字符，再引入非罚字符不匹配；函数应恢复并返回已映射前缀范围。
        // Match source chars first, then introduce a non-penalty mismatch.
        // The function should recover and return the mapped prefix range.
        let range = map_owned_wrapped_line_to_range("hello world", /*cursor*/ 0, "helloX", "");
        assert_eq!(range, 0..5);
    }

    #[test]
    fn borrowed_slice_range_rejects_slices_outside_source_text() {
        let text = "test message";
        let external = String::from("test");

        assert_eq!(borrowed_slice_range(text, &external), None);

        let fallback = map_owned_wrapped_line_to_range(text, /*cursor*/ 0, &external, "");
        assert_eq!(fallback, 0..4);
    }

    #[test]
    fn map_owned_wrapped_line_to_range_indent_coincides_with_source() {
        // 合成缩进前缀与源文本以同一字符开头时，映射器不得把缩进字符误当源匹配；
        // 否则会过早设置匹配状态并返回过短范围。
        // When the synthetic indent prefix starts with a character that also
        // appears at the current source position, the mapper must not confuse
        // the indent char for a source match.  Here the indent is "- " and the
        // source text also starts with "-", so a naive char-by-char match would
        // consume the source "-" for the indent "-", set saw_source_char too
        // early, then break on the space — returning 0..1 instead of the full
        // first word.
        let text = "- item one and some more words";
        // 模拟 `subsequent_indent = "- "` 时 textwrap 为第一条续行产生的输出。
        // Simulate what textwrap would produce for the first continuation line
        // when subsequent_indent = "- ": it prepends "- " to the source slice.
        let range = map_owned_wrapped_line_to_range(text, /*cursor*/ 0, "- - item one", "- ");
        // 映射器应跳过合成的 `- ` 前缀，把 `- item one` 映射回源字节 `0..10`。
        // The mapper should skip the synthetic "- " prefix and map "- item one"
        // back to source bytes 0..10.
        assert_eq!(range, 0..10);
    }

    #[test]
    fn wrap_ranges_indent_prefix_coincides_with_source_char() {
        // 端到端验证：源文本与缩进前缀以同一字符开头时，`wrap_ranges` 仍可重建完整源文本。
        // End-to-end: source text starts with the same character as the indent
        // prefix.  wrap_ranges must still reconstruct the full source.
        let text = "- first item is long enough to wrap around";
        let opts = || textwrap::Options::new(16).initial_indent("- ").subsequent_indent("- ");
        let ranges = wrap_ranges(text, opts());
        assert!(!ranges.is_empty());

        let mut rebuilt = String::new();
        let mut cursor = 0usize;
        for range in ranges {
            let start = range.start.max(cursor).min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                rebuilt.push_str(&text[start..end]);
            }
            cursor = cursor.max(end);
        }
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn wrap_ranges_count_halfwidth_sound_marks_without_changing_byte_offsets() {
        for (text, width, expected) in [
            ("abｶﾞc", 4, &["abｶﾞ", "c"][..]),
            ("abｶﾞc", 3, &["ab", "ｶﾞc"][..]),
            ("ﾞab", 2, &["ﾞa", "b"][..]),
            ("a ﾞb", 2, &["a", "ﾞb"][..]),
            ("a ﾟb", 2, &["a", "ﾟb"][..]),
            ("ｶﾞﾞx", 3, &["ｶﾞﾞ", "x"][..]),
            ("界ﾞx", 3, &["界ﾞ", "x"][..]),
            ("ｶﾞﾞab", 2, &["ｶﾞﾞ", "ab"][..]),
            ("界ﾞab", 2, &["界ﾞ", "ab"][..]),
            ("abｶﾞﾞcd", 2, &["ab", "ｶﾞﾞ", "cd"][..]),
            ("ab界ﾞcd", 2, &["ab", "界ﾞ", "cd"][..]),
        ] {
            let ranges = wrap_ranges_trim(text, Options::new(width));
            let wrapped = ranges.iter().map(|range| &text[range.clone()]).collect_vec();
            assert_eq!(wrapped, expected);
        }

        for (text, emoji, sound_mark) in [("a👨‍👩 ﾞ", "👨‍👩", "ﾞ"), ("a👍🏻 ﾟ", "👍🏻", "ﾟ")]
        {
            for options in [
                Options::new(/*width*/ 2),
                Options::new(/*width*/ 2).word_separator(WordSeparator::AsciiSpace),
            ] {
                let ranges = wrap_ranges_trim(text, options);
                let wrapped = ranges.iter().map(|range| &text[range.clone()]).collect_vec();

                assert_eq!(wrapped, ["a", emoji, sound_mark]);
            }
        }

        for grapheme in ["ｶﾞﾞ", "界ﾞ"] {
            for width in [1, 2] {
                let ranges = wrap_ranges(grapheme, Options::new(width));
                assert_eq!(ranges, std::iter::once(0..grapheme.len() + 1).collect_vec());
            }
        }
    }

    #[test]
    fn wrap_ranges_preserve_crlf_source_boundaries_without_splitting_graphemes() {
        for prefix in ["ｶﾞ", "ﾊﾟ"] {
            let text = format!("{prefix}\r\nnext");

            for word_separator in [WordSeparator::UnicodeBreakProperties, WordSeparator::AsciiSpace]
            {
                for line_ending in [textwrap::LineEnding::LF, textwrap::LineEnding::CRLF] {
                    let options = Options::new(/*width*/ 4)
                        .line_ending(line_ending)
                        .word_separator(word_separator)
                        .wrap_algorithm(textwrap::WrapAlgorithm::FirstFit);
                    let first_end =
                        prefix.len() + usize::from(line_ending == textwrap::LineEnding::LF);
                    let second_start = prefix.len() + "\r\n".len();

                    let ranges = wrap_ranges(&text, options.clone());
                    assert_eq!(ranges, [0..first_end + 1, second_start..text.len() + 1]);
                    for range in &ranges {
                        assert!(text.get(range.start..range.end - 1).is_some());
                    }

                    let trimmed = wrap_ranges_trim(&text, options);
                    assert_eq!(trimmed, [0..first_end, second_start..text.len()]);
                }
            }
        }
    }

    #[test]
    fn map_owned_wrapped_line_to_range_repro_overconsumes_repeated_prefix_patterns() {
        let text = "- - foo";
        let opts = textwrap::Options::new(3)
            .initial_indent("- ")
            .subsequent_indent("- ")
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .break_words(false);
        let wrapped = textwrap::wrap(text, opts);
        let Some(line) = wrapped.first() else {
            panic!("expected at least one wrapped line");
        };

        let mapped = map_owned_wrapped_line_to_range(text, /*cursor*/ 0, line.as_ref(), "- ");
        let expected_len = line.as_ref().strip_prefix("- ").unwrap_or(line.as_ref()).len();
        let mapped_len = mapped.end.saturating_sub(mapped.start);
        assert!(
            mapped_len <= expected_len,
            "overconsumed source: text={text:?} line={line:?} mapped={mapped:?} expected_len={expected_len}"
        );
    }

    #[test]
    fn wrap_ranges_recovers_with_non_space_indents() {
        let text = "The quick brown fox jumps over the lazy dog";
        let wrapped = textwrap::wrap(
            text,
            textwrap::Options::new(12).initial_indent("* ").subsequent_indent("  "),
        );
        assert!(
            wrapped.iter().any(|line| matches!(line, std::borrow::Cow::Owned(_))),
            "expected textwrap to produce owned lines with synthetic indent prefixes"
        );

        let ranges = wrap_ranges(
            text,
            textwrap::Options::new(12).initial_indent("* ").subsequent_indent("  "),
        );
        assert!(!ranges.is_empty());

        // `wrap_ranges` 返回面向光标、可能重叠一字节的范围；按游标推进重建以验证完整源覆盖。
        // wrap_ranges returns cursor-oriented ranges that may overlap by one byte;
        // rebuild with cursor progression to validate full source coverage.
        let mut rebuilt = String::new();
        let mut cursor = 0usize;
        for range in ranges {
            let start = range.start.max(cursor).min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                rebuilt.push_str(&text[start..end]);
            }
            cursor = cursor.max(end);
        }

        assert_eq!(rebuilt, text);
    }

    #[test]
    fn wrap_ranges_trim_handles_owned_lines_with_penalty_char() {
        fn split_every_char(word: &str) -> Vec<usize> {
            word.char_indices().skip(1).map(|(idx, _)| idx).collect()
        }

        let text = "a_very_long_token_without_spaces";
        let opts = Options::new(8)
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .word_splitter(textwrap::WordSplitter::Custom(split_every_char))
            .break_words(false);

        let ranges = wrap_ranges_trim(text, opts);
        let rebuilt = ranges.iter().map(|range| &text[range.clone()]).collect::<String>();

        assert_eq!(rebuilt, text);
        assert!(ranges.len() > 1, "expected wrapped ranges, got: {ranges:?}");
    }
