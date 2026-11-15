    use super::*;
    use pretty_assertions::assert_eq;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    // 验证 `one_cell_width_falls_back_without_panic_for_wrapped_two_column_rows` 场景的触发条件与预期结果。
    #[test]
    fn one_cell_width_falls_back_without_panic_for_wrapped_two_column_rows() {
        let row = GenericDisplayRow {
            name: "1. Very long option label".to_string(),
            description: Some("Very long description".to_string()),
            wrap_indent: Some(4),
            ..Default::default()
        };

        let two_col = wrap_two_column_row(&row, /*desc_col*/ 0, /*width*/ 1);
        assert_eq!(two_col.len(), 0);
    }

    // 验证 `popup_name_truncation_counts_halfwidth_sound_marks` 场景的触发条件与预期结果。
    #[test]
    fn popup_name_truncation_counts_halfwidth_sound_marks() {
        for (name, desc_col, match_index, expected) in
            [("abｶﾞc", 6, None, "abｶﾞ…"), ("aｶﾞc", 4, Some(1), "a…")]
        {
            let row = GenericDisplayRow {
                name: name.to_string(),
                description: Some("description".to_string()),
                match_indices: match_index.map(|index| vec![index]),
                ..Default::default()
            };
            let text =
                build_full_line(&row, desc_col, SelectionDescriptionLayout::Columns).to_string();

            assert!(text.starts_with(expected), "unexpected row: {text:?}");
        }
    }

    // 验证 `fuzzy_matched_emoji_graphemes_keep_description_alignment` 场景的触发条件与预期结果。
    #[test]
    fn fuzzy_matched_emoji_graphemes_keep_description_alignment() {
        let rows = [
            GenericDisplayRow {
                name: "👍🏻".to_string(),
                match_indices: Some(vec![1]),
                description: Some("description".to_string()),
                ..Default::default()
            },
            GenericDisplayRow {
                name: "👨‍👩‍👧‍👦".to_string(),
                match_indices: Some(vec![2]),
                description: Some("description".to_string()),
                ..Default::default()
            },
        ];
        let area = Rect::new(0, 0, /*width*/ 20, /*height*/ 2);
        let mut buf = Buffer::empty(area);

        for (row_index, row) in rows.iter().enumerate() {
            let line =
                build_full_line(row, /*desc_col*/ 4, SelectionDescriptionLayout::Columns);
            let name = line.spans.first().expect("fuzzy-matched name span");
            assert_eq!(name.content.as_ref(), row.name);
            assert!(name.style.add_modifier.contains(Modifier::BOLD));

            let row_area = Rect::new(area.x, area.y + row_index as u16, area.width, 1);
            ratatui::widgets::Widget::render(
                ratatui::widgets::Paragraph::new(line),
                row_area,
                &mut buf,
            );
            assert_eq!(buf[(4, row_index as u16)].symbol(), "d");
        }

        insta::assert_snapshot!("popup_fuzzy_matched_emoji_graphemes", format!("{buf:?}"));
    }

    // 验证 `wrapped_two_column_rows_count_halfwidth_sound_marks` 场景的触发条件与预期结果。
    #[test]
    fn wrapped_two_column_rows_count_halfwidth_sound_marks() {
        let rows = vec![GenericDisplayRow {
            name: "abｶﾞc".to_string(),
            description: Some("abﾊﾟc".to_string()),
            wrap_indent: Some(0),
            ..Default::default()
        }];
        let area = Rect::new(0, 0, 9, 2);
        let mut buf = Buffer::empty(area);

        let rendered_lines = render_rows(
            area,
            &mut buf,
            &rows,
            &ScrollState::default(),
            /*max_results*/ 1,
            "no rows",
        );
        let rendered = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(rendered_lines, 2);
        insta::assert_snapshot!(rendered, @r"
        abｶﾞ  ab
        c     ﾊﾟ c
        ");
    }

    // 验证 `wrapped_two_column_rows_preserve_hard_line_breaks` 场景的触发条件与预期结果。
    #[test]
    fn wrapped_two_column_rows_preserve_hard_line_breaks() {
        let row = GenericDisplayRow {
            name: "first\nsecond".to_string(),
            description: Some("alpha\nbeta".to_string()),
            wrap_indent: Some(0),
            ..Default::default()
        };

        let rendered = wrap_two_column_row(&row, /*desc_col*/ 8, /*width*/ 24)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!(rendered, @r"
        first   alpha
        second  beta
        ");
    }

    // 验证 `selected_rows_use_the_shared_accent_style` 场景的触发条件与预期结果。
    #[test]
    fn selected_rows_use_the_shared_accent_style() {
        let rows = vec![GenericDisplayRow { name: "selected".to_string(), ..Default::default() }];
        let state = ScrollState { selected_idx: Some(0), ..Default::default() };
        let area = Rect::new(0, 0, 16, 1);
        let mut buf = Buffer::empty(area);

        render_rows(area, &mut buf, &rows, &state, /*max_results*/ 1, "no rows");

        let style = buf[(0, 0)].style();
        let expected = accent_style();
        assert_eq!(style.fg, expected.fg);
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }
