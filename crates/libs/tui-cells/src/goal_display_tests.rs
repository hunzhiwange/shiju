use super::*;

    #[test]
    // 覆盖秒、小时和天的边界，确认状态行使用紧凑单位。
    fn elapsed_time_uses_compact_units() {
        assert_eq!(format_goal_elapsed_seconds(59), "59s");
        assert_eq!(format_goal_elapsed_seconds(90 * 60), "1h 30m");
        assert_eq!(format_goal_elapsed_seconds(24 * 60 * 60), "1d 0h 0m");
    }
