//! Chat input domain tests migrated from the desktop host.
mod attachment_tests;
mod model_popover_tests;
mod queue_panel_tests;
mod reasoning_selector_tests;
mod task_mode_tests;
mod todo_panel_tests;
mod agent_sessions_tests;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_gettext_chat_visible_copy() {
    use crate::input_panel::voice_controls::{Input, recording_labels};
    use desktop_voice_ui::message::voice_draft::Phase;
    use config_ui::DesktopLocale;
    for (locale, transcribing, preparing, cancel) in [
        (DesktopLocale::SimplifiedChinese, "正在转写…", "正在准备麦克风…", "取消听写"),
        (DesktopLocale::English, "Transcribing…", "Preparing microphone…", "Cancel dictation"),
        (DesktopLocale::TraditionalChinese, "正在轉寫…", "正在準備麥克風…", "取消聽寫"),
    ] {
        let i18n = desktop_i18n::I18n::new(locale);
        let input = Input { phase: Phase::Transcribing, pending: false, ready: false };
        assert_eq!(recording_labels(i18n, input), (Some(transcribing), cancel));
        assert_eq!(recording_labels(i18n, Input { phase: Phase::Recording, ..input }), (Some(preparing), cancel));
        assert_eq!(recording_labels(i18n, Input { phase: Phase::Recording, ready: true, ..input }), (None, cancel));
        let hint = crate::input_panel::voice_controls::dictation_tooltip(i18n, "听写");
        assert_eq!(hint, i18n.__("听写"));
        let diagnostic = "服务原文 /tmp/中文.rs %n/%s";
        let failure = format!("麦克风授权失败：{diagnostic}");
        let rendered = desktop_voice_ui::feedback::render(i18n, &failure);
        assert!(rendered.ends_with(diagnostic));
        assert_eq!(desktop_voice_ui::feedback::render(i18n, diagnostic), diagnostic);
        assert_eq!(desktop_i18n::input::feedback(diagnostic).render(i18n).unwrap(), diagnostic);
        for raw in ["用户消息 /tmp/中文.rs", "转写正文", "终端输出: Completed", r#"{"status":"Running"}"#] {
            assert_eq!(desktop_i18n::chat::trajectory_label(i18n, raw), raw);
        }
    }
}
