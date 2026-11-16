use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_16_lsp_state_switches_without_translating_server_content() {
    let mut status = None;
    let mut localized = None;
    let raw = "服务/%s/%n";
    crate::editor::set_lsp_status(&mut status, &mut localized, "LSP 失败: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "LSP 失败: "),
        (DesktopLocale::TraditionalChinese, "LSP 失敗: "),
        (DesktopLocale::English, "LSP failed: "),
    ] {
        assert_eq!(
            crate::editor::lsp_status(locale, status.as_deref(), localized.as_ref()).unwrap(),
            format!("{prefix}{raw}")
        );
    }
    status = Some(raw.into());
    assert_eq!(
        crate::editor::lsp_status(DesktopLocale::English, status.as_deref(), localized.as_ref())
            .as_deref(),
        Some(raw)
    );
}

#[test]
fn desktop_gettext_area_14_hub_title_updates_after_locale_change() {
    let mut title = String::new();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "专家·技能·连接器"),
        (DesktopLocale::TraditionalChinese, "專家·技能·連接器"),
        (DesktopLocale::English, "Experts · Skills · Connectors"),
    ] {
        crate::apps::refresh_hub_titles(
            crate::I18n::new(locale),
            [("expert_hub", &mut title)],
            &[("expert_hub", "专家·技能·连接器")],
        );
        assert_eq!(title, expected);
    }
}

#[test]
fn desktop_gettext_area_18_generated_session_titles_preserve_renames() {
    let feedback = crate::input::Feedback::new("规划 · %s", vec!["id/%s/%n".into()]);
    let raw = feedback.fallback.clone();
    assert_eq!(
        crate::other::session_title(
            crate::I18n::new(DesktopLocale::English),
            &raw,
            None,
            Some(&feedback)
        ),
        "Planning · id/%s/%n"
    );
    assert_eq!(
        crate::other::session_title(
            crate::I18n::new(DesktopLocale::TraditionalChinese),
            &raw,
            None,
            Some(&feedback)
        ),
        "規劃 · id/%s/%n"
    );
    assert_eq!(
        crate::other::session_title(
            crate::I18n::new(DesktopLocale::TraditionalChinese),
            "用户/%s/%n",
            None,
            Some(&feedback)
        ),
        "用户/%s/%n"
    );
}

#[path = "host_module_39_locale_tests.rs"]
mod locale;
