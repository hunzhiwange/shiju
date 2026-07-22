use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

#[test]
fn parse_positive_u32_trims_input_and_accepts_boundaries() {
    assert_eq!(parse_positive_u32(" 1 ", "field"), Ok(1));
    assert_eq!(parse_positive_u32(&u32::MAX.to_string(), "field"), Ok(u32::MAX));
}

#[test]
fn parse_positive_u32_rejects_empty_and_whitespace_only_input() {
    assert_eq!(
        parse_positive_u32("", "goal_loop.interval_minutes"),
        Err("goal_loop.interval_minutes 不能为空".to_string())
    );
    assert_eq!(
        parse_positive_u32(" \t\n", "goal_loop.step_timeout_secs"),
        Err("goal_loop.step_timeout_secs 不能为空".to_string())
    );
}

#[test]
fn parse_positive_u32_rejects_zero_invalid_and_out_of_range_input() {
    assert_eq!(parse_positive_u32("0", "field"), Err("field 必须大于 0".to_string()));

    for input in ["-1", "1.5", "abc", "4294967296"] {
        assert_eq!(
            parse_positive_u32(input, "field"),
            Err("field 必须是正整数".to_string()),
            "unexpected result for {input:?}"
        );
    }
}

#[test]
fn persist_goal_loop_settings_reports_fields_in_validation_order() {
    let mut app = app();

    app.goal_loop_settings.interval_minutes_input = " ".to_string();
    app.goal_loop_settings.step_timeout_secs_input = "0".to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = "invalid".to_string();
    assert_eq!(
        persist_goal_loop_settings(&mut app).err().as_deref(),
        Some("goal_loop.interval_minutes 不能为空")
    );

    app.goal_loop_settings.interval_minutes_input = "10".to_string();
    assert_eq!(
        persist_goal_loop_settings(&mut app).err().as_deref(),
        Some("goal_loop.step_timeout_secs 必须大于 0")
    );

    app.goal_loop_settings.step_timeout_secs_input = "120".to_string();
    assert_eq!(
        persist_goal_loop_settings(&mut app).err().as_deref(),
        Some("goal_loop.max_steps_per_cycle 必须是正整数")
    );
}

#[test]
fn persist_goal_loop_settings_accepts_valid_trimmed_values() {
    let mut app = app();
    app.goal_loop_settings.enabled = true;
    app.goal_loop_settings.interval_minutes_input = " 15 ".to_string();
    app.goal_loop_settings.step_timeout_secs_input = " 30 ".to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = " 5 ".to_string();
    app.goal_loop_settings.channel_input = " slack ".to_string();
    app.goal_loop_settings.target_input = " ops ".to_string();

    assert!(persist_goal_loop_settings(&mut app).is_ok());

    app.goal_loop_settings.channel_input = " \t".to_string();
    app.goal_loop_settings.target_input = "\n".to_string();
    assert!(persist_goal_loop_settings(&mut app).is_ok());
}

#[test]
fn goal_loop_update_ignores_unrelated_settings_message() {
    let mut app = app();
    app.goal_loop_settings.enabled = true;
    app.goal_loop_settings.interval_minutes_input = "17".to_string();
    app.goal_loop_settings.step_timeout_secs_input = "31".to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = "6".to_string();
    app.goal_loop_settings.channel_input = "channel".to_string();
    app.goal_loop_settings.target_input = "target".to_string();
    app.goal_loop_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.goal_loop_settings.enabled);
    assert_eq!(app.goal_loop_settings.interval_minutes_input, "17");
    assert_eq!(app.goal_loop_settings.step_timeout_secs_input, "31");
    assert_eq!(app.goal_loop_settings.max_steps_per_cycle_input, "6");
    assert_eq!(app.goal_loop_settings.channel_input, "channel");
    assert_eq!(app.goal_loop_settings.target_input, "target");
    assert_eq!(app.goal_loop_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn goal_loop_numeric_updates_keep_input_and_report_the_failing_field() {
    let mut app = app();

    let cases = [
        (
            GoalLoopMessage::IntervalMinutesChanged("0".to_string()),
            "goal_loop.interval_minutes 必须大于 0",
        ),
        (
            GoalLoopMessage::StepTimeoutSecsChanged("invalid".to_string()),
            "goal_loop.step_timeout_secs 必须是正整数",
        ),
        (
            GoalLoopMessage::MaxStepsPerCycleChanged(" ".to_string()),
            "goal_loop.max_steps_per_cycle 不能为空",
        ),
    ];

    for (message, expected_error) in cases {
        let _ = update(&mut app, SettingsMessage::GoalLoop(message));
        assert_eq!(app.goal_loop_settings.save_error.as_deref(), Some(expected_error));

        app.goal_loop_settings.interval_minutes_input = "10".to_string();
        app.goal_loop_settings.step_timeout_secs_input = "120".to_string();
        app.goal_loop_settings.max_steps_per_cycle_input = "3".to_string();
    }
}

#[test]
fn goal_loop_updates_all_fields_and_clears_previous_error() {
    let mut app = app();
    app.goal_loop_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::GoalLoop(GoalLoopMessage::EnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::IntervalMinutesChanged(" 15 ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::StepTimeoutSecsChanged(" 30 ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::MaxStepsPerCycleChanged(" 5 ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::ChannelChanged(" slack ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::TargetChanged(" ops ".to_string())),
    );

    assert!(app.goal_loop_settings.enabled);
    assert_eq!(app.goal_loop_settings.interval_minutes_input, " 15 ");
    assert_eq!(app.goal_loop_settings.step_timeout_secs_input, " 30 ");
    assert_eq!(app.goal_loop_settings.max_steps_per_cycle_input, " 5 ");
    assert_eq!(app.goal_loop_settings.channel_input, " slack ");
    assert_eq!(app.goal_loop_settings.target_input, " ops ");
    assert!(app.goal_loop_settings.save_error.is_none());
}

#[test]
fn goal_loop_validation_rejection_skips_persistence_but_valid_edit_schedules_it() {
    let mut app = app();

    let rejected = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::IntervalMinutesChanged("0".to_string())),
    );
    assert!(!has_scheduled_action(rejected));
    assert_eq!(
        app.goal_loop_settings.save_error.as_deref(),
        Some("goal_loop.interval_minutes 必须大于 0")
    );

    let scheduled = update(
        &mut app,
        SettingsMessage::GoalLoop(GoalLoopMessage::IntervalMinutesChanged("15".to_string())),
    );
    assert!(has_scheduled_action(scheduled));
    assert!(app.goal_loop_settings.save_error.is_none());
}
