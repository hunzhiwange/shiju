    use super::*;
    use crate::line_truncation::line_width;
    use pretty_assertions::assert_eq;
    use ratatui::text::Line;

    #[test]
    fn display_width_matches_ratatui_halfwidth_sound_marks_without_overflow() {
        assert_eq!(display_width("ｶﾞﾊﾟ"), 4);
        assert_eq!(display_width("ｶﾞﾞ"), 3);
        assert_eq!(display_width("界ﾞ"), 3);
        assert_eq!(char_width('\u{FF9E}'), 1);
        assert_eq!(char_width('\u{FF9F}'), 1);

        let text = "a".repeat(65_536);
        assert_eq!(display_width(&text), 65_536);
        assert_eq!(line_width(&Line::from(text)), 65_536);
    }

    #[test]
    fn usable_content_width_returns_none_when_reserved_exhausts_width() {
        assert_eq!(usable_content_width(/*total_width*/ 0, /*reserved_cols*/ 0), None);
        assert_eq!(usable_content_width(/*total_width*/ 2, /*reserved_cols*/ 2), None);
        assert_eq!(usable_content_width(/*total_width*/ 3, /*reserved_cols*/ 4), None);
        assert_eq!(usable_content_width(/*total_width*/ 5, /*reserved_cols*/ 4), Some(1));
    }

    #[test]
    fn usable_content_width_u16_matches_usize_variant() {
        assert_eq!(usable_content_width_u16(/*total_width*/ 2, /*reserved_cols*/ 2), None);
        assert_eq!(usable_content_width_u16(/*total_width*/ 5, /*reserved_cols*/ 4), Some(1));
    }
