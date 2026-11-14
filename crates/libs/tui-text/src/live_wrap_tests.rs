    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn rows_do_not_exceed_width_ascii() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("hello whirl this is a test");
        let rows = rb.rows().to_vec();
        assert_eq!(
            rows,
            vec![
                Row { text: "hello whir".to_string(), explicit_break: false },
                Row { text: "l this is ".to_string(), explicit_break: false }
            ]
        );
    }

    #[test]
    fn rows_do_not_exceed_width_emoji_cjk() {
        // 😀 is width 2; 你/好 are width 2.
        let mut rb = RowBuilder::new(/*target_width*/ 6);
        rb.push_fragment("😀😀 你好");
        let rows = rb.rows().to_vec();
        // 宽度 6 时首行能容纳两个表情和一个空格；首个中日韩字符需两列会溢出，因此留在缓冲区。
        // At width 6, we expect the first row to fit exactly two emojis and a space
        // (2 + 2 + 1 = 5) plus one more column for the first CJK char (2 would overflow),
        // so only the two emojis and the space fit; the rest remains buffered.
        assert_eq!(rows, vec![Row { text: "😀😀 ".to_string(), explicit_break: false }]);
    }

    #[test]
    fn halfwidth_sound_marks_stay_with_their_grapheme_when_wrapping() {
        assert_eq!(take_prefix_by_width("abｶﾞc", /*max_cols*/ 3), ("ab".to_string(), "ｶﾞc", 2));
        assert_eq!(take_prefix_by_width("abｶﾞc", /*max_cols*/ 4), ("abｶﾞ".to_string(), "c", 4));

        let mut row_builder = RowBuilder::new(/*target_width*/ 1);
        row_builder.push_fragment("ｶﾞx");
        assert_eq!(row_builder.rows()[0].text, "ｶﾞ");
    }

    #[test]
    fn fragmentation_invariance_long_token() {
        let s = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 26 chars
        let mut rb_all = RowBuilder::new(/*target_width*/ 7);
        rb_all.push_fragment(s);
        let all_rows = rb_all.rows().to_vec();

        let mut rb_chunks = RowBuilder::new(/*target_width*/ 7);
        for i in (0..s.len()).step_by(3) {
            let end = (i + 3).min(s.len());
            rb_chunks.push_fragment(&s[i..end]);
        }
        let chunk_rows = rb_chunks.rows().to_vec();

        assert_eq!(all_rows, chunk_rows);
    }

    #[test]
    fn newline_splits_rows() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("hello\nworld");
        let rows = rb.display_rows();
        assert!(rows.iter().any(|r| r.explicit_break));
        assert_eq!(rows[0].text, "hello");
        // 第二行应以 `world` 开头。
        // Second row should begin with 'world'
        assert!(rows.iter().any(|r| r.text.starts_with("world")));
    }

    #[test]
    fn rewrap_on_width_change() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("abcdefghijK");
        assert!(!rb.rows().is_empty());
        rb.set_width(/*width*/ 5);
        for r in rb.rows() {
            assert!(r.width() <= 5);
        }
    }
