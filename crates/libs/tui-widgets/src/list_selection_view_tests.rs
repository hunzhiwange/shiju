    use super::*;
    use insta::assert_snapshot;
    use ratatui::buffer::Buffer;
    fn render(view: &ListSelectionView, width: u16) -> String {
        let area = Rect::new(0, 0, width, view.desired_height(width));
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn items() -> Vec<V3SelectionItem> {
        vec![
            V3SelectionItem {
                id: "read".into(),
                label: "Read only".into(),
                description: Some("Inspect files".into()),
                is_current: true,
                ..Default::default()
            },
            V3SelectionItem {
                id: "write".into(),
                label: "Full access".into(),
                description: Some("Modify files".into()),
                ..Default::default()
            },
            V3SelectionItem {
                id: "locked".into(),
                label: "Unavailable".into(),
                disabled: true,
                disabled_reason: Some("not configured".into()),
                ..Default::default()
            },
        ]
    }
    // 验证 `v3_selection_surfaces_reuse_copied_codex_list_view` 场景的触发条件与预期结果。
    #[test]
    fn v3_selection_surfaces_reuse_copied_codex_list_view() {
        let mut view = ListSelectionView::from_v3("permissions", "Permissions", items());
        assert_eq!(view.handle_key_event(KeyEvent::from(KeyCode::Down)), V3Action::None);
        assert_eq!(
            view.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select { surface: "permissions".into(), item_id: "write".into() }
        );
        assert_snapshot!(render(&view, 48), @r###"
Permissions
  1. Read only (current)     Inspect files
› 2. Full access             Modify files
     Unavailable (disabled)  disabled: not
     configured
"###);
    }
    // 验证 `filtering_skips_disabled_rows_and_empty_state_is_rendered` 场景的触发条件与预期结果。
    #[test]
    fn filtering_skips_disabled_rows_and_empty_state_is_rendered() {
        let mut view = ListSelectionView::new(SelectionViewParams {
            surface: "search".into(),
            title: Some("Search".into()),
            is_searchable: true,
            items: items().into_iter().map(SelectionItem::from_v3).collect(),
            ..Default::default()
        });
        view.handle_key_event(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert_eq!(view.selected_index(), None);
        assert!(render(&view, 40).contains("no matches"));
    }
