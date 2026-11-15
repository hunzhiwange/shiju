use super::contracts::{message_footer_padding, processing_duration_label};
use iced::Padding;
use session_ui_types::ProcessingTiming;

#[test]
fn user_message_footer_padding_has_right_offset() {
    let padding = message_footer_padding(true);
    assert_eq!(padding, Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 });
}

#[test]
fn non_user_message_footer_padding_is_zero() {
    let padding = message_footer_padding(false);
    assert_eq!(padding, Padding::ZERO);
}

#[test]
fn processing_duration_formats_seconds_minutes_and_hours() {
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 1_000,
                completed_ms: Some(60_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 59s")
    );
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 1_000,
                completed_ms: Some(61_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 1m 0s")
    );
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 1_000,
                completed_ms: Some(1_518_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 25m 17s")
    );
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 1_000,
                completed_ms: Some(3_601_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 1h 0m 0s")
    );
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 1_000,
                completed_ms: Some(3_724_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 1h 2m 3s")
    );
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&ProcessingTiming {
                started_ms: 2_000,
                completed_ms: Some(1_000),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            }),
            None
        )
        .as_deref(),
        Some("已处理 0s")
    );
}

#[test]
fn processing_label_shows_zero_for_active_open_cycle() {
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: None,
        processing_elapsed_ms: Some(0),
        llm_elapsed_ms: Some(0),
    };

    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&timing),
            Some(0)
        )
        .as_deref(),
        Some("已处理 0s")
    );
}

#[test]
fn historical_processing_label_includes_pauses_outside_llm_activity() {
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: Some(61_000),
        processing_elapsed_ms: Some(60_000),
        llm_elapsed_ms: Some(2_750),
    };

    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&timing),
            None
        )
        .as_deref(),
        Some("已处理 1m 0s")
    );
}
