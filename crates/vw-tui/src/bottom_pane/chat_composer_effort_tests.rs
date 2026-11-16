//! 验证输入编辑器提升推理档位时的点火动画、状态栏过渡与页脚优先级。

use super::tests::new_test_composer;
use super::tests::snapshot_composer_state_with_width;
use super::*;
use pretty_assertions::assert_eq;

#[test]
// 首次建基线、重复档位和降档都不重播动画，只有后续升档触发过渡。
fn effort_composer_baseline_repeat_and_lowering_do_not_replay() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_status_line(Some(Line::from("gpt-5.4 high · main")));
    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_none());
    assert!(!composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    ));

    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Max),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_some());
    assert!(composer.effort_animation_style.is_some());
    assert!(composer.effort_status_line_transition.is_some());
    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_some());
    assert!(composer.effort_status_line_transition.is_some());
    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Medium),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_none());
    assert!(composer.effort_status_line_transition.is_none());
}

#[test]
// 任务运行且草稿排队时没有可见旧状态行，因此升档只点火而不排队页脚过渡。
fn effort_transition_does_not_queue_a_missing_outgoing_status_line() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_status_line(Some(Line::from("gpt-5.4 high · main")));
    composer.set_task_running(/*running*/ true);
    composer.set_text_content("queued draft".to_string(), Vec::new(), Vec::new());
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::High),
        /*animations_enabled*/ true,
    );

    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_some());
    assert!(composer.effort_status_line_transition.is_none());
}

#[test]
// 计划模式提示占用页脚时，升档不能缓存一条实际不可见的旧状态行。
fn effort_transition_does_not_queue_while_plan_mode_nudge_is_visible() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_status_line(Some(Line::from("gpt-5.4 high · main")));
    composer.set_plan_mode_nudge_visible(/*visible*/ true);
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::High),
        /*animations_enabled*/ true,
    );

    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_some());
    assert!(composer.effort_status_line_transition.is_none());
}

#[test]
// 恢复基线和关闭动画均不得启动过渡，随后启用动画的真实升档仍可触发。
fn effort_composer_restored_baseline_and_reduced_motion_do_not_start() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_status_line(Some(Line::from("gpt-5.4 high · main")));
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::High),
        /*animations_enabled*/ false,
    );
    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ false,
    ));
    assert_eq!(composer.effort_tier, Some(EffortTier::Ultra));
    assert!(composer.effort_ignition.is_none());
    assert_eq!(composer.effort_animation_style, None);
    assert!(composer.effort_status_line_transition.is_none());

    assert!(composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Max),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_some());
    assert!(composer.effort_status_line_transition.is_some());
    composer.set_active_reasoning_effort_baseline(Some(&ReasoningEffort::Max));
    assert_eq!(composer.effort_tier, Some(EffortTier::Max));
    assert!(composer.effort_ignition.is_none());
    assert!(composer.effort_status_line_transition.is_none());
    assert!(!composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Max),
        /*animations_enabled*/ true,
    ));
    assert!(composer.effort_ignition.is_none());
    assert!(composer.effort_status_line_transition.is_none());
}

#[test]
// 临时反馈消息的显示优先级高于推理档位状态栏过渡。
fn effort_transition_never_replaces_a_footer_flash() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_status_line(Some(Line::from("gpt-5.4 high · main")));
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::High),
        /*animations_enabled*/ true,
    );
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    );
    composer.set_status_line(Some(Line::from("gpt-5.4 ultra · main")));
    composer
        .footer
        .show_flash(Line::from("saved"), Duration::from_secs(/*secs*/ 1));

    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 60, /*height*/ 6,
    );
    let mut buf = Buffer::empty(area);
    composer.render(area, &mut buf);
    let footer = (0..area.width)
        .map(|column| buf[(column, area.bottom() - 1)].symbol())
        .collect::<String>();

    assert!(footer.contains("saved"));
    assert!(!footer.contains("U L T R A"));
}

#[test]
// 过渡首帧保留完整旧状态行，并遵循窄页脚下隐藏模式提示的既有布局。
fn effort_transition_keeps_the_full_footer_row() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_status_line_enabled(/*enabled*/ true);
    composer.set_collaboration_modes_enabled(/*enabled*/ true);
    composer.set_collaboration_mode_indicator(Some(CollaborationModeIndicator::Plan));
    composer.set_status_line(Some(Line::from("gpt-5.4 high · feature-branch")));
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::High),
        /*animations_enabled*/ true,
    );
    composer.set_active_reasoning_effort(
        Some(&ReasoningEffort::Ultra),
        /*animations_enabled*/ true,
    );

    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 36, /*height*/ 6,
    );
    let mut buf = Buffer::empty(area);
    composer.render(area, &mut buf);
    let footer = (0..area.width)
        .map(|column| buf[(column, area.bottom() - 1)].symbol())
        .collect::<String>();

    assert!(footer.contains("gpt-5.4 high · feature-branch"));
    assert!(!footer.contains("Plan mode"));
    insta::assert_snapshot!("effort_transition_keeps_the_full_footer_row", footer);
}

#[test]
// Ultra 档位除颜色强调外，还会把输入提示符升级为双箭头字形。
fn ultra_accent_upgrades_prompt_glyph() {
    snapshot_composer_state_with_width(
        "ultra_accent_upgrades_prompt_glyph",
        /*width*/ 60,
        /*enhanced_keys_supported*/ false,
        |composer| {
            composer.set_active_reasoning_effort(
                Some(&ReasoningEffort::Ultra),
                /*animations_enabled*/ true,
            );
        },
    );
}
