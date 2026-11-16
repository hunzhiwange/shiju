use crate::components::message::Message;
use desktop_release::InstallerKind;
use desktop_update::app::installer_update::PreparedDesktopUpdate;
use iced::advanced::{renderer::Headless, widget::Tree};
impl desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(_: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        Self::None
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
) -> iced::Element<'a, Message> {
    desktop_ui_controls::selectable_text::markdown_selection::selectable_titled_text(
        title,
        body,
        theme.map(|theme| theme.palette().text),
        i18n,
    )
}
fn fixture(kind: InstallerKind) -> (tempfile::TempDir, PreparedDesktopUpdate) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().canonicalize().unwrap();
    let extension = match kind {
        InstallerKind::Dmg => "dmg",
        InstallerKind::Msi => "msi",
        InstallerKind::Rpm => "rpm",
        InstallerKind::Deb => "deb",
    };
    let name = format!("installer.{extension}");
    let path = directory.join(&name);
    std::fs::write(&path, b"verified").unwrap();
    let ready: PreparedDesktopUpdate = serde_json::from_value(serde_json::json!({
        "path": path, "directory": directory, "version": "1.2.3",
        "asset": {"name": name, "target": "", "kind": kind, "url": "",
            "size": 8, "sha256": "1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb"}
    }))
    .unwrap();
    (root, ready)
}

#[tokio::test(flavor = "multi_thread")]
async fn app_update_ready_installer_draws_all_loading_frames_in_both_themes() {
    let mut panel = desktop_update::panel::PanelState::default();
    panel.cli_install_modal_use_app_update_action = true;
    panel.cli_install_modal_show_install_action = true;
    let mut state =
        desktop_update::app::desktop_update::DesktopUpdate::from_config(&serde_json::json!({}));
    // Draw every loading frame and the ready/retry controls in both themes.
    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .unwrap();
    for (kind, theme) in [
        (InstallerKind::Dmg, iced::Theme::Dark),
        (InstallerKind::Dmg, iced::Theme::Light),
        (InstallerKind::Rpm, iced::Theme::Dark),
        (InstallerKind::Rpm, iced::Theme::Light),
        (InstallerKind::Deb, iced::Theme::Dark),
        (InstallerKind::Deb, iced::Theme::Light),
    ] {
        let (_root, ready) = fixture(kind);
        let mut effects = Vec::new();
        state.busy = true;
        let _ = desktop_update::panel::update(
            &mut panel,
            &mut state,
            &mut effects,
            "0.6.0",
            config_ui::DesktopLocale::SimplifiedChinese,
            desktop_update::panel::PanelMessage::AppUpdatePrepared(Ok(ready)),
        );
        for frame in 0..=10 {
            state.busy = frame < 10;
            panel.cli_install_modal_is_checking_update = state.busy;

            let mut element = crate::components::install_cli_modal::view_inner(
                desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
                &panel.cli_install_modal_title,
                &panel.cli_install_modal_message,
                Some(&panel.cli_install_modal_current_version),
                Some(&panel.cli_install_modal_server_version),
                panel.cli_install_modal_is_checking_update,
                panel.cli_install_modal_show_install_action,
                panel.cli_install_modal_use_app_update_action,
                false,
                panel.app_update_progress.as_ref(),
                None,
                Some(&state),
                None,
                frame,
                help,
            );
            let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 900.0));
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
            );
            iced::advanced::Renderer::reset(&mut renderer, viewport);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                iced::advanced::Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels =
                renderer.screenshot(iced::Size::new(960, 900), 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
