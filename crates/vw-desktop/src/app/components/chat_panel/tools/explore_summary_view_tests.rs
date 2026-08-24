//! 验证探索工具摘要的去重、运行态、展开控制与数字动画布局。
//! 重复键保留最后一项；只有用户显式展开后，运行态内容才进入详情区域。

use super::explore_summary_view::{
    EXPANDED_LIST_TOP_SPACING, SummarySegmentKind, explore_summary_expanded,
    explore_summary_is_running, latest_explore_items, right_aligned_slot_char,
    split_summary_segments, summary_animation_key, tool_explore_summary_view,
};
use super::types::{EXPLORE_GROUP_TOOL_IDX, ExploreItem};
use crate::app::components::chat_panel::tool_text_support::tool_text_key;
use crate::app::state::ExploreSummaryAnimationState;
use crate::app::{App, Message};
use iced::widget::text_editor;
use std::collections::HashSet;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn right_aligned_slot_char_pads_left_slots() {
    let chars = ['1', '2'];

    assert_eq!(right_aligned_slot_char(&chars, 0, 3), "");
    assert_eq!(right_aligned_slot_char(&chars, 1, 3), "1");
}

#[test]
fn summary_animation_key_packs_message_and_group() {
    assert_ne!(summary_animation_key(1, 2), summary_animation_key(1, 3));
}

#[test]
fn split_summary_segments_keeps_plain_text_and_numbers() {
    let segments = split_summary_segments("Read 12 files");

    assert_eq!(
        segments,
        vec![
            (SummarySegmentKind::Text, "Read "),
            (SummarySegmentKind::Number, "12"),
            (SummarySegmentKind::Text, " files"),
        ]
    );
    assert!(split_summary_segments("").is_empty());
}

#[test]
fn explore_summary_expanded_ignores_running_until_user_expands() {
    let key = 42;
    let mut expanded = HashSet::new();

    assert!(!explore_summary_expanded(true, key, &expanded));

    expanded.insert(key);
    assert!(explore_summary_expanded(true, key, &expanded));
}

#[test]
fn explore_summary_is_running_respects_following_block_closure() {
    assert!(explore_summary_is_running(true, false, false));
    assert!(explore_summary_is_running(false, true, false));
    assert!(!explore_summary_is_running(true, true, true));
    assert!(!explore_summary_is_running(false, false, false));
}

#[test]
fn latest_explore_items_keeps_last_item_for_same_dedupe_key() {
    let first = ExploreItem {
        tool_idx: 1,
        raw: r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"completed"}"#
            .to_string(),
    };
    let second = ExploreItem {
        tool_idx: 2,
        raw: r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"completed"}"#
            .to_string(),
    };
    let third = ExploreItem {
        tool_idx: 3,
        raw: r#"tool grep
{"input":"{\"pattern\":\"App\"}","status":"completed"}"#
            .to_string(),
    };
    let items = vec![first, second, third];

    let latest = latest_explore_items(&items);

    assert_eq!(latest.len(), 2);
    assert_eq!(latest[0].tool_idx, 2);
    assert_eq!(latest[1].tool_idx, 3);
}

#[test]
fn tool_explore_summary_view_returns_none_for_empty_items() {
    let app = test_app();

    assert!(tool_explore_summary_view(&app, 0, 0, &[], false, false).is_none());
}

#[test]
fn tool_explore_summary_view_builds_collapsed_summary_without_expanded_content() {
    let app = test_app();
    let items = vec![
        ExploreItem {
            tool_idx: 1,
            raw: r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"completed"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 2,
            raw: r#"tool grep
{"input":"{\"pattern\":\"App\"}","status":"completed"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 3,
            raw: r#"tool glob
{"input":"{\"pattern\":\"*.rs\"}","status":"running"}"#
                .to_string(),
        },
    ];

    let collapsed =
        tool_explore_summary_view(&app, 1, 0, &items, false, false).expect("collapsed summary");
    keep_element(collapsed);
}

#[test]
fn tool_explore_summary_view_builds_expanded_content_with_top_gap() {
    assert_eq!(EXPANDED_LIST_TOP_SPACING, 10.0);

    let mut app = test_app();
    let items = vec![
        ExploreItem {
            tool_idx: 1,
            raw: r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"completed"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 2,
            raw: r#"tool grep
{"input":"{\"pattern\":\"App\"}","status":"completed"}"#
                .to_string(),
        },
    ];

    let group_tool_idx = EXPLORE_GROUP_TOOL_IDX;
    let key = ((1_u64) << 32) | group_tool_idx as u64;
    app.chat_explore_expanded.insert(key);
    app.chat_tool_hovered_idx = Some(key);
    let expanded =
        tool_explore_summary_view(&app, 1, 0, &items, false, false).expect("expanded summary");
    keep_element(expanded);
}

#[test]
fn right_aligned_slot_char_handles_empty_and_full_width_slots() {
    assert_eq!(right_aligned_slot_char(&[], 0, 3), "");
    assert_eq!(right_aligned_slot_char(&['a', 'b', 'c'], 0, 3), "a");
    assert_eq!(right_aligned_slot_char(&['a', 'b', 'c'], 1, 3), "b");
    assert_eq!(right_aligned_slot_char(&['a', 'b', 'c'], 2, 3), "c");
}

#[test]
fn summary_animation_key_differs_across_message_or_group() {
    assert_eq!(summary_animation_key(1, 2), summary_animation_key(1, 2));
    assert_ne!(summary_animation_key(1, 2), summary_animation_key(2, 2));
    assert_ne!(summary_animation_key(0, 0), summary_animation_key(0, 1));
    assert_eq!(summary_animation_key(0, 0), 0);
}

#[test]
fn split_summary_segments_handles_various_patterns() {
    assert_eq!(split_summary_segments("12345"), vec![(SummarySegmentKind::Number, "12345")]);
    assert_eq!(
        split_summary_segments("hello world"),
        vec![(SummarySegmentKind::Text, "hello world")]
    );
    assert_eq!(
        split_summary_segments("12 and 34"),
        vec![
            (SummarySegmentKind::Number, "12"),
            (SummarySegmentKind::Text, " and "),
            (SummarySegmentKind::Number, "34"),
        ]
    );
    assert_eq!(
        split_summary_segments("12 files"),
        vec![(SummarySegmentKind::Number, "12"), (SummarySegmentKind::Text, " files"),]
    );
    assert_eq!(
        split_summary_segments("file 12"),
        vec![(SummarySegmentKind::Text, "file "), (SummarySegmentKind::Number, "12"),]
    );
    assert_eq!(split_summary_segments("a"), vec![(SummarySegmentKind::Text, "a")]);
    assert_eq!(split_summary_segments("1"), vec![(SummarySegmentKind::Number, "1")]);
}

#[test]
fn latest_explore_items_handles_all_different_keys() {
    let items = vec![
        ExploreItem {
            tool_idx: 1,
            raw: r#"tool read
{"call_id":"call-a","input":"{}"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 2,
            raw: r#"tool grep
{"call_id":"call-b","input":"{}"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 3,
            raw: r#"tool glob
{"call_id":"call-c","input":"{}"}"#
                .to_string(),
        },
    ];
    let latest = latest_explore_items(&items);
    assert_eq!(latest.len(), 3);
}

#[test]
fn latest_explore_items_keeps_only_last_when_all_same_key() {
    let items = vec![
        ExploreItem {
            tool_idx: 1,
            raw: r#"tool read
{"call_id":"call-x","input":"{}"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 2,
            raw: r#"tool read
{"call_id":"call-x","input":"{}"}"#
                .to_string(),
        },
        ExploreItem {
            tool_idx: 3,
            raw: r#"tool read
{"call_id":"call-x","input":"{}"}"#
                .to_string(),
        },
    ];
    let latest = latest_explore_items(&items);
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].tool_idx, 3);
}

#[test]
fn latest_explore_items_handles_none_keys() {
    let items = vec![
        ExploreItem { tool_idx: 1, raw: "no delimiter".to_string() },
        ExploreItem { tool_idx: 2, raw: "invalid".to_string() },
    ];
    let latest = latest_explore_items(&items);
    assert_eq!(latest.len(), 2);
}

#[test]
fn latest_explore_items_empty_and_single() {
    assert!(latest_explore_items(&[]).is_empty());

    let single = vec![ExploreItem {
        tool_idx: 5,
        raw: r#"tool read
{"call_id":"call-z","input":"{}"}"#
            .to_string(),
    }];
    assert_eq!(latest_explore_items(&single).len(), 1);
    assert_eq!(latest_explore_items(&single)[0].tool_idx, 5);
}

#[test]
fn tool_explore_summary_view_with_only_read_items() {
    let app = test_app();
    let items = vec![ExploreItem {
        tool_idx: 1,
        raw: r#"tool read
{"call_id":"call-r","input":"{}","status":"completed"}"#
            .to_string(),
    }];
    let view = tool_explore_summary_view(&app, 0, 0, &items, false, false);
    assert!(view.is_some());
    keep_element(view.unwrap());
}

#[test]
fn tool_explore_summary_view_with_no_explore_tool_kind() {
    let app = test_app();
    let items = vec![ExploreItem {
        tool_idx: 1,
        raw: r#"tool unknown_tool
{"call_id":"call-u","input":"{}","status":"completed"}"#
            .to_string(),
    }];
    let view = tool_explore_summary_view(&app, 0, 0, &items, false, false);
    assert!(view.is_some());
    keep_element(view.unwrap());
}

#[test]
fn tool_explore_summary_view_with_force_running_no_items() {
    let app = test_app();
    let items = vec![ExploreItem {
        tool_idx: 1,
        raw: r#"tool read
{"call_id":"call-r","input":"{}","status":"completed"}"#
            .to_string(),
    }];
    let view = tool_explore_summary_view(&app, 0, 0, &items, true, false);
    assert!(view.is_some());
    keep_element(view.unwrap());
}

#[test]
fn tool_explore_summary_view_uses_active_number_animation() {
    let mut app = test_app();
    let msg_idx = 4;
    let group_idx = 2;
    app.chat_explore_summary_animations.insert(
        summary_animation_key(msg_idx, group_idx),
        ExploreSummaryAnimationState {
            previous_summary_text: "1 次读取".to_string(),
            current_summary_text: "2 次读取".to_string(),
            changed_at_ms: Some(u64::MAX),
        },
    );
    let items = vec![
        ExploreItem {
            tool_idx: 1,
            raw: "tool read\n{\"call_id\":\"a\",\"input\":\"{}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ExploreItem {
            tool_idx: 2,
            raw: "tool read\n{\"call_id\":\"b\",\"input\":\"{}\",\"status\":\"completed\"}"
                .to_string(),
        },
    ];

    keep_element(
        tool_explore_summary_view(&app, msg_idx, group_idx, &items, false, false)
            .expect("animated summary"),
    );
}

#[test]
fn tool_explore_summary_view_uses_matching_cached_summary_editor() {
    let mut app = test_app();
    app.chat_tool_text_editors.insert(
        tool_text_key(0, EXPLORE_GROUP_TOOL_IDX, 0),
        text_editor::Content::with_text("1 次读取"),
    );
    let items = vec![ExploreItem {
        tool_idx: 1,
        raw: "tool read\n{\"call_id\":\"read\",\"input\":\"{}\",\"status\":\"completed\"}"
            .to_string(),
    }];

    keep_element(
        tool_explore_summary_view(&app, 0, 0, &items, false, false).expect("cached summary"),
    );
}

#[test]
fn tool_explore_summary_view_falls_back_when_animation_state_cannot_animate() {
    let items = vec![ExploreItem {
        tool_idx: 1,
        raw: "tool read\n{\"call_id\":\"read\",\"input\":\"{}\",\"status\":\"completed\"}"
            .to_string(),
    }];

    for state in [
        ExploreSummaryAnimationState {
            previous_summary_text: "0 次读取".to_string(),
            current_summary_text: "2 次读取".to_string(),
            changed_at_ms: Some(u64::MAX),
        },
        ExploreSummaryAnimationState {
            previous_summary_text: "1 次读取".to_string(),
            current_summary_text: "1 次读取".to_string(),
            changed_at_ms: None,
        },
        ExploreSummaryAnimationState {
            previous_summary_text: "1 次读取".to_string(),
            current_summary_text: "1 次读取".to_string(),
            changed_at_ms: Some(0),
        },
        ExploreSummaryAnimationState {
            previous_summary_text: "1 次读取".to_string(),
            current_summary_text: "1 次读取".to_string(),
            changed_at_ms: Some(u64::MAX),
        },
    ] {
        let mut app = test_app();
        app.chat_explore_summary_animations.insert(summary_animation_key(0, 0), state);

        keep_element(
            tool_explore_summary_view(&app, 0, 0, &items, false, false).expect("fallback summary"),
        );
    }
}

#[test]
fn expanded_summary_skips_malformed_compact_items() {
    let mut app = test_app();
    let key = EXPLORE_GROUP_TOOL_IDX as u64;
    app.chat_explore_expanded.insert(key);
    let items = vec![
        ExploreItem { tool_idx: 1, raw: "missing delimiter".to_string() },
        ExploreItem { tool_idx: 2, raw: "tool \n{}".to_string() },
        ExploreItem { tool_idx: 3, raw: "tool read\nnot-json".to_string() },
    ];

    keep_element(
        tool_explore_summary_view(&app, 0, 0, &items, false, false)
            .expect("summary with malformed detail rows"),
    );
}
