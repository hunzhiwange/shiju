//! Desktop event routing and native screenshot contract for domain MCP tabs.
use super::view;
use crate::app::Message;
use crate::app::message::settings::{McpMessage, SettingsMessage};
use crate::app::state::McpSettingsTab;
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_mcp() {
    use crate::app::App;
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_extensions::text as tr;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_mcp"),
        || App::new().0,
        |scene, app| {
            app.mcp_settings.active_tab = McpSettingsTab::Servers;
            app.mcp_settings.save_error = Some("fixture MCP error".into());
            let locale = app.general_settings.desktop_locale;
            // Match the agents page's real viewport, scrolled to the legacy controls.
            let page = iced::widget::scrollable(iced::widget::container(view(app)))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .anchor_bottom()
                .height(iced::Length::Fill);
            let tab = scene.check(
                app,
                page.into(),
                &["全局服务器", "运行时 MCP", "高级 JSON"].map(|label| tr(locale, label)),
                2,
            );
            assert!(matches!(
                tab,
                SettingsMessage::Mcp(McpMessage::TabSelected(McpSettingsTab::Json))
            ));
            let _ = app.update(Message::Settings(tab));
            assert_eq!(app.mcp_settings.active_tab, McpSettingsTab::Json);
            let labels = scene.labels(view(app));
            assert!(!labels.contains(tr(locale, "当前没有全局 MCP 服务器。")));
            assert!(labels.contains("fixture MCP error"));
        },
    )
    .await;
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn instruction_copy_feedback_expires_per_button_and_ignores_older_timers() {
    use crate::app::App;
    use crate::app::message::settings::mcp::extensions::message;
    use desktop_settings_mcp::extensions::{InstructionCopy, Message as Event};
    use gateway_desktop_settings::{McpExtension, McpInstallInstructions};

    let mut app = App::new().0;
    let extension = McpExtension::ChromeDevTools;
    let command = InstructionCopy::Command(0);
    let configuration = InstructionCopy::Configuration;
    let _ = app.update(message(Event::Instructions(
        extension,
        Ok(McpInstallInstructions {
            version: "fixture".into(),
            shell: "fixture".into(),
            commands: vec!["command".into()],
            configuration: "{}".into(),
        }),
    )));

    let mut copy = |item| {
        let _ = app.update(message(Event::CopyInstruction(extension, item)));
        let request = app.mcp_settings.extensions.copy_sequence;
        let _ = app.update(message(Event::InstructionCopied(extension, item, request, Ok(()))));
        request
    };
    let first = copy(command);
    let config_request = copy(configuration);
    let last = copy(command);

    let _ = app.update(message(Event::ClearCopyFeedback(extension, command, first)));
    let feedback = &app.mcp_settings.extensions.cards[&extension].copy_feedback;
    assert!(feedback[&command].copied);
    assert!(feedback[&configuration].copied);

    let _ = app.update(message(Event::ClearCopyFeedback(extension, configuration, config_request)));
    let feedback = &app.mcp_settings.extensions.cards[&extension].copy_feedback;
    assert!(feedback[&command].copied);
    assert!(!feedback.contains_key(&configuration));

    let _ = app.update(message(Event::ClearCopyFeedback(extension, command, last)));
    assert!(app.mcp_settings.extensions.cards[&extension].copy_feedback.is_empty());
}
