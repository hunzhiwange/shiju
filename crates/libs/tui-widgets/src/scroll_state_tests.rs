    use super::ScrollState;

    // 验证 `wrap_navigation_and_visibility` 场景的触发条件与预期结果。
    #[test]
    fn wrap_navigation_and_visibility() {
        let mut s = ScrollState::new();
        let len = 10;
        let vis = 5;

        s.clamp_selection(len);
        assert_eq!(s.selected_idx, Some(0));
        s.ensure_visible(len, vis);
        assert_eq!(s.scroll_top, 0);

        s.move_up_wrap(len);
        s.ensure_visible(len, vis);
        assert_eq!(s.selected_idx, Some(len - 1));
        match s.selected_idx {
            Some(sel) => assert!(s.scroll_top <= sel),
            None => panic!("expected Some(selected_idx) after wrap"),
        }

        s.move_down_wrap(len);
        s.ensure_visible(len, vis);
        assert_eq!(s.selected_idx, Some(0));
        assert_eq!(s.scroll_top, 0);
    }

    // 验证 `page_and_jump_navigation_clamps` 场景的触发条件与预期结果。
    #[test]
    fn page_and_jump_navigation_clamps() {
        let mut s = ScrollState::new();
        let len = 10;
        let vis = 4;

        s.clamp_selection(len);
        s.page_down_clamped(len, vis);
        assert_eq!(s.selected_idx, Some(4));
        assert_eq!(s.scroll_top, 1);

        s.page_down_clamped(len, vis);
        assert_eq!(s.selected_idx, Some(8));
        assert_eq!(s.scroll_top, 5);

        s.page_down_clamped(len, vis);
        assert_eq!(s.selected_idx, Some(9));
        assert_eq!(s.scroll_top, 6);

        s.page_up_clamped(len, vis);
        assert_eq!(s.selected_idx, Some(5));
        assert_eq!(s.scroll_top, 5);

        s.jump_top(len, vis);
        assert_eq!(s.selected_idx, Some(0));
        assert_eq!(s.scroll_top, 0);

        s.jump_bottom(len, vis);
        assert_eq!(s.selected_idx, Some(9));
        assert_eq!(s.scroll_top, 6);
    }
