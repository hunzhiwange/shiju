use iced::Color;
use iced::advanced::Widget;

use crate::app::DiffTheme;

use super::utils::{
    FileStatus, Lang, get_diff_colors, get_word_diff_ranges, highlight_segments, lang_for_file,
    line_content_parts, render_line_content,
};

#[test]
fn file_status_derives_expected_traits() {
    assert_eq!(FileStatus::Modified, FileStatus::Modified);
    assert_ne!(FileStatus::Added, FileStatus::Deleted);
    assert_eq!(format!("{:?}", FileStatus::Renamed), "Renamed");
    let copied = FileStatus::Untracked;
    let cloned = copied;
    assert_eq!(copied, cloned);
    assert_eq!(FileStatus::Unknown, FileStatus::Unknown);
}

#[test]
fn lang_for_file_recognizes_supported_extensions() {
    let cases = [
        ("main.rs", Lang::Rust),
        ("app.ts", Lang::Ts),
        ("view.tsx", Lang::Ts),
        ("index.js", Lang::Js),
        ("index.jsx", Lang::Js),
        ("data.json", Lang::Json),
        ("Cargo.toml", Lang::Toml),
        ("ci.yaml", Lang::Yaml),
        ("ci.yml", Lang::Yaml),
        ("script.py", Lang::Python),
        ("server.go", Lang::Go),
        ("native.c", Lang::C),
        ("native.cpp", Lang::Cpp),
        ("native.cc", Lang::Cpp),
        ("native.cxx", Lang::Cpp),
        ("native.hpp", Lang::Cpp),
        ("index.html", Lang::Html),
        ("index.htm", Lang::Html),
        ("style.css", Lang::Css),
        ("query.sql", Lang::Sql),
        ("run.sh", Lang::Bash),
        ("run.bash", Lang::Bash),
        ("README", Lang::Other),
    ];

    for (name, expected) in cases {
        assert!(matches!(
            (lang_for_file(name), expected),
            (Lang::Rust, Lang::Rust)
                | (Lang::Ts, Lang::Ts)
                | (Lang::Js, Lang::Js)
                | (Lang::Json, Lang::Json)
                | (Lang::Toml, Lang::Toml)
                | (Lang::Yaml, Lang::Yaml)
                | (Lang::Python, Lang::Python)
                | (Lang::Go, Lang::Go)
                | (Lang::C, Lang::C)
                | (Lang::Cpp, Lang::Cpp)
                | (Lang::Html, Lang::Html)
                | (Lang::Css, Lang::Css)
                | (Lang::Sql, Lang::Sql)
                | (Lang::Bash, Lang::Bash)
                | (Lang::Other, Lang::Other)
        ));
    }
}

#[test]
fn lang_for_file_requires_a_lowercase_terminal_extension() {
    for name in ["MAIN.RS", "component.TSX", "archive.rs.bak", "README.rs.txt", ""] {
        assert!(matches!(lang_for_file(name), Lang::Other));
    }

    assert!(matches!(lang_for_file("nested/src/lib.rs"), Lang::Rust));
}

#[test]
fn highlight_segments_splits_words_symbols_and_keywords() {
    assert!(highlight_segments("", Lang::Rust, DiffTheme::GitHub).is_empty());

    let segments = highlight_segments("if value_else == 1", Lang::Rust, DiffTheme::GitHub);
    assert_eq!(segments[0], (0, 2, true));
    assert_eq!(segments[1], (2, 3, false));
    assert_eq!(segments[2], (3, 13, false));
    assert_eq!(segments.last().copied(), Some((17, 18, false)));

    let keyword_segments =
        highlight_segments("let var else fn function class export", Lang::Ts, DiffTheme::Monokai);
    assert!(keyword_segments.iter().filter(|(_, _, is_kw)| *is_kw).count() >= 7);
}

#[test]
fn highlight_segments_preserves_utf8_boundaries_and_exact_keyword_matches() {
    let line = "fn café_2()";
    let chunks: Vec<_> = highlight_segments(line, Lang::Rust, DiffTheme::GitHub)
        .into_iter()
        .map(|(start, end, is_keyword)| (&line[start..end], is_keyword))
        .collect();

    assert_eq!(
        chunks,
        vec![
            ("fn", true),
            (" ", false),
            ("caf", false),
            ("é", false),
            ("_2", false),
            ("()", false),
        ]
    );
    assert_eq!(highlight_segments("+-=", Lang::Other, DiffTheme::Monokai), vec![(0, 3, false)]);
}

#[test]
fn word_diff_ranges_trim_changed_words_but_keep_whitespace_only_changes() {
    let (old_ranges, new_ranges) = get_word_diff_ranges("hello world", "hello rust");
    assert_eq!(old_ranges, vec![6..11]);
    assert_eq!(new_ranges, vec![6..10]);

    let (old_ranges, new_ranges) = get_word_diff_ranges("same", "same");
    assert!(old_ranges.is_empty());
    assert!(new_ranges.is_empty());

    let (old_ranges, new_ranges) = get_word_diff_ranges("a \n b", "a \n  b");
    assert!(old_ranges.is_empty() || old_ranges.iter().all(|range| range.start < range.end));
    assert!(new_ranges.is_empty() || new_ranges.iter().all(|range| range.start < range.end));
}

#[test]
fn word_diff_ranges_cover_whole_text_insertions_and_deletions() {
    let (old_ranges, new_ranges) = get_word_diff_ranges("", "rust");
    assert!(old_ranges.is_empty());
    assert_eq!(new_ranges, vec![0..4]);

    let (old_ranges, new_ranges) = get_word_diff_ranges("rust", "");
    assert_eq!(old_ranges, vec![0..4]);
    assert!(new_ranges.is_empty());
}

#[test]
fn render_line_content_builds_rows_for_plain_syntax_and_highlighted_ranges() {
    let base = Color::from_rgb8(1, 2, 3);
    let highlight = Color::from_rgb8(4, 5, 6);

    let _ = render_line_content(
        "let value = 1",
        Lang::Rust,
        DiffTheme::GitHub,
        true,
        &[0..3, 10..11],
        base,
        highlight,
    );
    let _ =
        render_line_content("plain", Lang::Other, DiffTheme::Monokai, false, &[], base, highlight);
}

#[test]
fn render_line_content_accepts_empty_content_and_ranges_beyond_content() {
    let base = Color::from_rgb8(1, 2, 3);
    let highlight = Color::from_rgb8(4, 5, 6);

    let _ = render_line_content("", Lang::Rust, DiffTheme::GitHub, true, &[0..0], base, highlight);
    let _ = render_line_content(
        "plain",
        Lang::Other,
        DiffTheme::Monokai,
        false,
        &[10..20],
        base,
        highlight,
    );
}

#[test]
fn render_line_content_splits_disjoint_highlight_ranges_at_exact_boundaries() {
    let row = render_line_content(
        "abcdef",
        Lang::Other,
        DiffTheme::GitHub,
        false,
        &[1..3, 4..6],
        Color::TRANSPARENT,
        Color::BLACK,
    );

    assert_eq!(Widget::children(&row).len(), 4);
}

#[test]
fn line_content_parts_marks_highlighted_ranges_across_syntax_segments() {
    let parts = line_content_parts("let value = 1", Lang::Rust, DiffTheme::GitHub, true, &[1..9]);
    let spans = parts
        .iter()
        .map(|part| (part.start, part.end, part.color.is_some(), part.is_highlighted))
        .collect::<Vec<_>>();

    assert_eq!(
        spans,
        vec![
            (0, 1, true, false),
            (1, 3, true, true),
            (3, 4, false, true),
            (4, 9, false, true),
            (9, 12, false, false),
            (12, 13, false, false),
        ]
    );
}

#[test]
fn line_content_parts_handles_empty_ranges_and_ranges_outside_content() {
    let plain = line_content_parts("abc", Lang::Other, DiffTheme::Monokai, false, &[]);
    assert_eq!(plain.len(), 1);
    assert_eq!((plain[0].start, plain[0].end, plain[0].is_highlighted), (0, 3, false));

    let outside = line_content_parts("abc", Lang::Other, DiffTheme::Monokai, false, &[10..12]);
    assert_eq!(outside.len(), 1);
    assert_eq!((outside[0].start, outside[0].end, outside[0].is_highlighted), (0, 3, false));
}

#[test]
fn diff_colors_match_builtin_theme_palettes() {
    assert_eq!(
        get_diff_colors(DiffTheme::GitHub),
        (
            Color::TRANSPARENT,
            Color::from_rgb8(0x24, 0x29, 0x2E),
            Color::from_rgb8(0xEA, 0xF7, 0xEE),
            Color::from_rgb8(0xC7, 0xE9, 0xD1),
            Color::from_rgb8(0xFE, 0xEF, 0xEE),
            Color::from_rgb8(0xF7, 0xC9, 0xC5),
            Color::from_rgb8(0xEE, 0xF3, 0xF8),
            Color::from_rgb8(0x57, 0x60, 0x6A),
        )
    );
    assert_eq!(
        get_diff_colors(DiffTheme::Monokai),
        (
            Color::TRANSPARENT,
            Color::from_rgb8(0xE6, 0xED, 0xF3),
            Color::from_rgb8(0x13, 0x2A, 0x1B),
            Color::from_rgb8(0x1E, 0x4B, 0x2C),
            Color::from_rgb8(0x31, 0x1A, 0x18),
            Color::from_rgb8(0x60, 0x30, 0x2B),
            Color::from_rgb8(0x1B, 0x20, 0x27),
            Color::from_rgb8(0xC9, 0xD1, 0xD9),
        )
    );
}
