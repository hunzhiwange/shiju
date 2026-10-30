use super::split_markdown;

#[test]
fn markdown_delivery_unicode_respects_byte_budget() {
    let input = "你好🙂e\u{301}".repeat(100);
    let chunks = split_markdown(&input, 19).unwrap();
    assert_eq!(chunks.concat(), input);
    assert!(chunks.iter().all(|chunk| !chunk.is_empty() && chunk.len() <= 19));
    assert!(split_markdown("🙂", 3).is_err());
    assert!(split_markdown("", 0).is_err());
    assert!(split_markdown("", 1).unwrap().is_empty());
    assert_eq!(split_markdown("你好", 6).unwrap(), ["你好"]);
    assert_eq!(split_markdown(" a \n", 4).unwrap(), [" a \n"]);
}

#[test]
fn markdown_delivery_fences_preserve_language_and_content() {
    for fence in ["```", "````", "~~~", "~~~~"] {
        let opening = format!("{fence}rust\n");
        let lines = "let a = 1;\n".repeat(30);
        let source = format!("{opening}{lines}{fence}");
        let chunks = split_markdown(&source, 70).unwrap();
        assert!(chunks.len() > 1);
        let mut restored = String::new();
        for chunk in chunks {
            assert!(chunk.len() <= 70);
            restored.push_str(chunk.strip_prefix(&opening).unwrap().strip_suffix(fence).unwrap());
        }
        assert_eq!(restored, lines);
    }
}

#[test]
fn markdown_delivery_short_fences_and_unclosed_source() {
    let source = "````rust\n```\ninside\n````";
    assert_eq!(split_markdown(source, source.len()).unwrap(), [source]);
    assert_eq!(split_markdown("```rust\nhello", 100).unwrap(), ["```rust\nhello\n```"]);
    assert_eq!(
        split_markdown("  ~~~rs\r\na\r\n  ~~~\r\n", 100).unwrap(),
        ["  ~~~rs\r\na\r\n  ~~~\r\n"]
    );
    assert!(split_markdown("```rs\n12345678901234567890\n```", 24).is_err());
}

#[test]
fn markdown_delivery_complete_code_blocks_stay_together() {
    let block = "```rs\nx\n```\n";
    let source = format!("preface\n\n{block}tail");
    let chunks = split_markdown(&source, block.len()).unwrap();
    assert_eq!(chunks.concat(), source);
    assert!(chunks.iter().any(|chunk| chunk == block));
}

#[test]
fn markdown_delivery_paragraphs_and_lines_are_preferred() {
    assert_eq!(split_markdown("one\n\ntwo\n\nthree", 10).unwrap(), ["one\n\ntwo\n\n", "three"]);
    assert_eq!(split_markdown("1234\n5678\n90", 7).unwrap(), ["1234\n", "5678\n90"]);
}

#[test]
fn markdown_delivery_indented_crlf_and_mismatched_fences_preserve_code() {
    let opening = "  ````rust\r\n";
    let code = "```\r\n~~~\r\n中文🙂\r\n".repeat(8);
    let source = format!("{opening}{code}  ````");
    let chunks = split_markdown(&source, 90).unwrap();
    assert!(chunks.len() > 1);
    let restored = chunks
        .iter()
        .map(|chunk| {
            assert!(chunk.len() <= 90);
            chunk.strip_prefix(opening).unwrap().strip_suffix("  ````").unwrap()
        })
        .collect::<String>();
    assert_eq!(restored, code);
}

#[test]
fn markdown_delivery_tables_repeat_header_without_duplicate_rows() {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let header = "| 名称 | 值 |\n| :--- | ---: |\n";
    let rows = (0..12).map(|i| format!("| 项{i} | `a\\|b` |\n")).collect::<Vec<_>>();
    let table = format!("{header}{}", rows.concat());
    assert_eq!(split_markdown(&table, table.len()).unwrap(), [table.clone()]);
    let chunks = split_markdown(&table, 95).unwrap();
    assert!(chunks.len() > 1);
    let mut restored = String::new();
    for chunk in &chunks {
        assert!(chunk.len() <= 95);
        restored.push_str(chunk.strip_prefix(header).expect("each table chunk repeats its header"));
        assert_eq!(
            Parser::new_ext(chunk, Options::ENABLE_TABLES)
                .filter(|event| matches!(event, Event::Start(Tag::Table(_))))
                .count(),
            1
        );
    }
    assert_eq!(restored, rows.concat());
    let mixed = format!("before\n\n{table}\nafter\n\n{header}| 尾 | |\n");
    let chunks = split_markdown(&mixed, 100).unwrap();
    assert_eq!(chunks.iter().map(|chunk| chunk.matches("| 项").count()).sum::<usize>(), 12);
    assert_eq!(chunks.iter().map(|chunk| chunk.matches("| 尾 | |").count()).sum::<usize>(), 1);
}

#[test]
fn markdown_delivery_tables_reject_oversized_atomic_row() {
    let source = format!("preface\n\n| a | b |\n| --- | --- |\n| {} | x |", "长".repeat(30));
    assert!(split_markdown(&source, 70).is_err());
    assert!(split_markdown("| very_long_header |\n| --- |\n| x |", 15).is_err());
}

#[test]
fn markdown_delivery_table_budget_matrix_preserves_rows() {
    let source = "| a | b |\n| --- | --- |\n| 甲 | |\n| 乙 | a\\|b |\n| 丙 | `x\\|y` |\n";
    for budget in [1, 4, 15, 30, 45, 60, 100, 3000] {
        if let Ok(chunks) = split_markdown(source, budget) {
            assert!(chunks.iter().all(|chunk| !chunk.is_empty() && chunk.len() <= budget));
            for row in ["| 甲 | |", "| 乙 | a\\|b |", "| 丙 | `x\\|y` |"] {
                assert_eq!(chunks.iter().map(|chunk| chunk.matches(row).count()).sum::<usize>(), 1);
            }
        }
    }
}

#[test]
fn markdown_delivery_tables_preserve_source_line_boundaries() {
    for newline in ["\n", "\r\n"] {
        for indentation in ["", "  "] {
            let header = format!("{indentation}a | b{newline}{indentation}--- | ---{newline}");
            let row = format!("{indentation}中文 | `a\\|b`{newline}");
            let source = format!("before{newline}{newline}{header}{}{newline}after", row.repeat(5));
            assert_eq!(split_markdown(&source, source.len()).unwrap().concat(), source);
            let chunks = split_markdown(&source, header.len() + row.len()).unwrap();
            assert!(chunks.iter().all(|chunk| chunk.len() <= header.len() + row.len()));
            assert_eq!(chunks.iter().map(|chunk| chunk.matches(&row).count()).sum::<usize>(), 5);
        }
    }
}
