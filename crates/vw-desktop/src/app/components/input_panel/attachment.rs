//! 桌面附件消息映射。
#[cfg(not(target_arch = "wasm32"))]
use crate::app::skill_hub::ExpertHubSection;
use crate::app::{App, Message, message};
use desktop_chat_input::input_panel::attachment as shared;
use desktop_chat_input::input_panel::attachment::menu_row;
use desktop_chat_input::popovers::FileMenuSection;
use desktop_chat_tools::state::SessionToolSelectorTab;
use desktop_ui_controls::composer_icons::ComposerIcon;
use desktop_ui_controls::overlays::{AboveOverlay, SideOverlay};
use desktop_ui_controls::system_settings_common::settings_divider;
use iced::widget::{Space, column, container, mouse_area, text_editor};
use iced::{Element, Length};
pub(crate) use shared::{AttachmentDisplayItem, parse_attachment_markers};

fn map_message(message: shared::Message) -> Message {
    match message {
        shared::Message::Pick => Message::Project(message::ProjectMessage::AttachmentFilesPick),
        shared::Message::Open(path) => Message::Preview(message::PreviewMessage::Open(path)),
        shared::Message::Remove(path) => {
            Message::Project(message::ProjectMessage::RemoveAttachedFile(path))
        }
    }
}
pub fn attachment_button(app: &App, mobile: bool) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let max_images = app.multimodal_settings.max_images as usize;
    let max_image_size = app.multimodal_settings.max_image_size_mb as usize;
    if mobile {
        return shared::attachment_button(
            i18n,
            &app.session_state.files,
            max_images,
            max_image_size,
            super::styles::round_icon_button_style,
        )
        .map(map_message);
    }

    let expanded = app.popover_visibility.file;
    let trigger = shared::attachment_trigger(
        i18n,
        &app.session_state.files,
        max_images,
        max_image_size,
        super::styles::round_icon_button_style,
        Some(expanded),
        Message::View(message::ViewMessage::ToggleFilePopover),
    );
    let close = Message::View(message::ViewMessage::CloseFilePopover);
    let menu = if expanded {
        attachment_menu(app, i18n, &close)
    } else {
        Space::new().into()
    };
    AboveOverlay::new(trigger, menu)
        .show(expanded)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(7.0)
        .on_close(close)
        .into()
}

fn attachment_menu(
    app: &App,
    i18n: crate::app::i18n::I18n,
    close: &Message,
) -> Element<'static, Message> {
    let select = |section| Message::View(message::ViewMessage::SelectFileMenuSection(section));
    let section = app.popover_visibility.file_section;
    let mut categories = column![].spacing(3);
    let show_tools =
        app.current_session_runtime_ref().is_none_or(|runtime| runtime.acp_agent.is_none());

    for (item, icon, title) in [
        (FileMenuSection::Files, ComposerIcon::Attachment, "添加文件"),
        (FileMenuSection::Experts, ComposerIcon::Expert, "专家"),
        (FileMenuSection::Agent, ComposerIcon::Agent, "智能体"),
        (FileMenuSection::Tools, ComposerIcon::Tools, "工具"),
        (FileMenuSection::Skills, ComposerIcon::Skill, "技能"),
    ] {
        if item == FileMenuSection::Tools && !show_tools {
            continue;
        }
        let event = select(Some(item));
        let selected = section == Some(item);
        let trigger = menu_row(i18n, icon, title, selected, true, event.clone());
        let trigger = mouse_area(trigger).on_enter(event);
        if selected {
            categories = categories.push(
                SideOverlay::new(trigger, section_menu(app, i18n, item, close))
                    .show(true)
                    .gap(9.0),
            );
        } else {
            categories = categories.push(trigger);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let categories = categories.push(
        mouse_area(menu_row(
            i18n,
            ComposerIcon::Connector,
            "管理连接器",
            false,
            false,
            close_then(
                close,
                Message::SkillHub(message::SkillHubMessage::SelectSection(
                    ExpertHubSection::Connectors,
                )),
            ),
        ))
        .on_enter(select(None)),
    );

    container(categories)
        .padding(7)
        .width(Length::Fixed(184.0))
        .style(super::styles::popover_style)
        .into()
}

fn close_then(close: &Message, action: Message) -> Message {
    Message::Batch(vec![close.clone(), action])
}

fn section_menu(
    app: &App,
    i18n: crate::app::i18n::I18n,
    section: FileMenuSection,
    close: &Message,
) -> Element<'static, Message> {
    let action = |icon, title, message| {
        menu_row(i18n, icon, title, false, false, close_then(close, message))
    };

    if section == FileMenuSection::Files {
        let mut items = column![action(
            ComposerIcon::Upload,
            "本地文件",
            Message::Project(message::ProjectMessage::AttachmentFilesPick),
        )]
        .spacing(3);
        if app.navigation_tabs.project_path.is_some() {
            let input = app
                .current_session_runtime_ref()
                .map_or(&app.session_state.input_editor, |runtime| &runtime.input_editor);
            let draft = input.text();
            let prefix =
                if draft.is_empty() || draft.ends_with(char::is_whitespace) { "@" } else { " @" };
            let mention = Message::Batch(vec![
                Message::Chat(message::ChatMessage::InputEditorAction(text_editor::Action::Move(
                    text_editor::Motion::DocumentEnd,
                ))),
                Message::Chat(message::ChatMessage::InputEditorAction(text_editor::Action::Edit(
                    text_editor::Edit::Paste(std::sync::Arc::new(prefix.to_owned())),
                ))),
            ]);
            items = items.push(action(ComposerIcon::Attachment, "引用项目文件", mention));
        }
        return container(items)
            .padding(7)
            .width(Length::Fixed(212.0))
            .style(super::styles::popover_style)
            .into();
    }

    let (tab, placeholder) = match section {
        FileMenuSection::Experts => (SessionToolSelectorTab::Skills, "搜索专家"),
        FileMenuSection::Agent => (SessionToolSelectorTab::Agent, "搜索智能体"),
        FileMenuSection::Tools => (SessionToolSelectorTab::Tools, "搜索工具"),
        FileMenuSection::Skills | FileMenuSection::Files => {
            (SessionToolSelectorTab::Skills, "搜索技能")
        }
    };
    let picker = crate::app::components::chat_panel::tool_selector::session_tool_selector_picker(
        app,
        tab,
        i18n.__(placeholder),
        section == FileMenuSection::Experts,
    );
    let mut items = column![picker].spacing(6);
    match section {
        #[cfg(not(target_arch = "wasm32"))]
        FileMenuSection::Experts => {
            items = items.push(settings_divider()).push(action(
                ComposerIcon::Expert,
                "浏览专家中心",
                Message::SkillHub(message::SkillHubMessage::SelectSection(
                    ExpertHubSection::Experts,
                )),
            ));
        }
        FileMenuSection::Agent => {
            items = items.push(settings_divider()).push(action(
                ComposerIcon::Agent,
                "配置智能体",
                Message::View(message::ViewMessage::OpenSystemSettingsTab(
                    crate::app::components::system_settings::SystemTab::Agents,
                )),
            ));
        }
        FileMenuSection::Skills => {
            items = items.push(settings_divider());
            #[cfg(not(target_arch = "wasm32"))]
            {
                items = items
                    .push(action(
                        ComposerIcon::Upload,
                        "从文件夹导入",
                        Message::Settings(message::SettingsMessage::SkillsImportFolder),
                    ))
                    .push(action(
                        ComposerIcon::Upload,
                        "从 ZIP 导入",
                        Message::Settings(message::SettingsMessage::SkillsImportZip),
                    ));
            }
            items = items.push(action(
                ComposerIcon::Skill,
                "技能管理",
                Message::View(message::ViewMessage::OpenSystemSettingsTarget(
                    desktop_settings_shell::search::SystemSettingsSearchTarget::AgentSkills(
                        crate::app::state::SkillsSettingsTab::Skills,
                    ),
                )),
            ));
        }
        #[cfg(target_arch = "wasm32")]
        FileMenuSection::Experts => {}
        FileMenuSection::Files | FileMenuSection::Tools => {}
    }
    container(items)
        .padding(8)
        .width(Length::Fixed(326.0))
        .style(super::styles::popover_style)
        .into()
}

pub fn attachment_strip(app: &App) -> Element<'_, Message> {
    shared::attachment_strip(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &app.session_state.files,
    )
    .map(map_message)
}
pub(crate) fn attachment_preview_strip(
    i18n: crate::app::i18n::I18n,
    items: Vec<AttachmentDisplayItem>,
) -> Element<'static, Message> {
    shared::attachment_preview_strip(i18n, items).map(map_message)
}
