//! 多模态快捷菜单与模型选择。

use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_skill_hub::skill_hub::LoadState;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::settings_muted_text_style;
use iced::widget::{Column, button, column, container, row, stack, svg, text};
use iced::{Alignment, Element, Length, Padding, Theme};
use vw_api_media::{MediaModel, MediaModelSelection, MediaOperation};

use super::quick_message::Message;
use super::quick_messages::{
    DROPDOWN_ICON_BOX_SIZE, QuickInput, dropdown_icon, shortcut_menu, shortcut_menu_items,
    shortcut_trigger,
};
use super::quick_styles as styles;

const PROMPTS: [(&str, Icon, &str); 3] = [
    ("生语音", Icon::ChatTextFill, "请把下面这段文字合成为语音："),
    ("理解视频", Icon::Eye, "请分析我提供的视频："),
    ("识语音", Icon::FileText, "请转写我提供的音频或视频："),
];

#[derive(Clone, Copy)]
pub struct MediaModelPicker<'a> {
    pub operation: MediaOperation,
    pub catalog: MediaModelCatalog<'a>,
}

#[derive(Clone, Copy)]
pub enum MediaModelCatalog<'a> {
    Loading,
    Failed,
    Ready(&'a [MediaModel]),
}

pub(super) fn menu(app: QuickInput<'_>, open: bool) -> Element<'_, Message> {
    let label = app.i18n.__("多模态与应用");
    let label = app.selected_media_model.map_or_else(
        || label.to_owned(),
        |name| format!("{label} · {}", super::model_popover::truncated_model_name(name)),
    );
    let label = container(SingleLineLabel::new(label, 12.0).shrink())
        .padding(Padding { right: DROPDOWN_ICON_BOX_SIZE + 5.0, ..Padding::ZERO });
    let trigger = shortcut_trigger(
        stack![
            label,
            container(dropdown_icon()).align_right(Length::Fill).center_y(Length::Fill),
        ],
        Message::MediaShortcutMenuChanged(!open),
        open,
    );

    if !open {
        return trigger.into();
    }

    let items = match app.media_models {
        Some(picker) => model_list(app.i18n, picker),
        None => operations(app),
    };
    shortcut_menu(trigger, items, Message::MediaShortcutMenuChanged(false))
}

fn operations(app: QuickInput<'_>) -> Element<'_, Message> {
    let can_select_skill = !matches!(app.installed, LoadState::Loading { .. });
    let mut items = vec![
        item(
            app.i18n.__("生图"),
            Icon::Image,
            Some(Message::OpenMediaModelPicker(MediaOperation::ImageGenerate)),
        ),
        item(
            app.i18n.__("生视频"),
            Icon::ArrowRight,
            Some(Message::OpenMediaModelPicker(MediaOperation::VideoGenerate)),
        ),
        item(
            app.i18n.__("识图"),
            Icon::Eye,
            Some(Message::OpenMediaModelPicker(MediaOperation::ImageUnderstand)),
        ),
    ];
    items.extend(PROMPTS.into_iter().map(|(label, icon, prompt)| {
        item(
            app.i18n.__(label),
            icon,
            Some(Message::MediaPromptSelected(app.i18n.__(prompt).into())),
        )
    }));
    items.extend([
        item(
            app.i18n.__("识矩剪辑"),
            Icon::Scissors,
            can_select_skill.then_some(Message::SelectVideoEditorSkill),
        ),
        item(
            app.i18n.__("应用开发"),
            Icon::Code,
            can_select_skill.then_some(Message::SelectAppDevelopmentSkill),
        ),
        item(
            app.i18n.__("应用管理"),
            Icon::Sliders,
            can_select_skill.then_some(Message::SelectAppManagementSkill),
        ),
    ]);
    let count = items.len();
    let items = Column::with_children(items).spacing(3).width(Length::Fill);
    shortcut_menu_items(items, count).into()
}

fn model_list(i18n: I18n, picker: MediaModelPicker<'_>) -> Element<'_, Message> {
    let title = match picker.operation {
        MediaOperation::ImageGenerate => "生图模型",
        MediaOperation::VideoGenerate => "生视频模型",
        MediaOperation::ImageUnderstand => "识图模型",
        _ => return column![].into(),
    };
    let back = button(super::icons::icon_svg(Icon::ChevronLeft, 13.0))
        .padding([6, 8])
        .style(|theme, status| styles::quick_message_button_style(theme, status, false))
        .on_press(Message::BackFromMediaModelPicker);
    let header =
        row![back, text(i18n.__(title)).size(13).font(super::styles::selector_label_font()),]
            .spacing(7)
            .align_y(Alignment::Center);
    let auto = item(i18n.__("自动选择"), Icon::Sliders, Some(Message::MediaModelPicked(None)));
    let status = |label| {
        container(text(i18n.__(label)).size(12).style(settings_muted_text_style)).padding([8, 10])
    };
    let list: Element<'_, Message> = match picker.catalog {
        MediaModelCatalog::Loading => status("正在加载媒体模型…").into(),
        MediaModelCatalog::Failed => status("无法加载媒体模型").into(),
        MediaModelCatalog::Ready([]) => status("没有支持当前操作的媒体模型").into(),
        MediaModelCatalog::Ready(models) => {
            let mut rows = column![].spacing(3).width(Length::Fill);
            for model in models {
                let provider = model
                    .deployment_id
                    .rsplit_once(':')
                    .map_or(model.deployment_id.as_str(), |(id, _)| id);
                let provider_name = if provider == "shiju" { i18n.__("识矩") } else { provider };
                let subtitle = format!("{provider_name} · {}", model.model_id);
                rows = rows.push(
                    button(
                        column![
                            text(&model.display_name)
                                .size(12)
                                .wrapping(text::Wrapping::WordOrGlyph),
                            text(subtitle)
                                .size(11)
                                .wrapping(text::Wrapping::WordOrGlyph)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                    )
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(|theme, status| styles::quick_message_button_style(theme, status, false))
                    .on_press(Message::MediaModelPicked(Some(
                        MediaModelSelection {
                            operation: picker.operation,
                            deployment_id: model.deployment_id.clone(),
                            model_id: model.model_id.clone(),
                        },
                    ))),
                );
            }
            container(shortcut_menu_items(rows, models.len())).max_height(224).into()
        }
    };
    column![header, auto, list].spacing(5).width(Length::Fill).into()
}

fn item(label: &str, icon: Icon, message: Option<Message>) -> Element<'_, Message> {
    let enabled = message.is_some();
    let icon = super::icons::icon_svg(icon, 14.0).style(move |theme: &Theme, _| svg::Style {
        color: Some(theme.palette().text.scale_alpha(if enabled { 0.72 } else { 0.38 })),
    });
    let mut content =
        row![icon, text(label).size(12).width(Length::Fill)].spacing(8).align_y(Alignment::Center);
    if matches!(&message, Some(Message::OpenMediaModelPicker(_))) {
        content = content.push(super::icons::icon_svg(Icon::ChevronRight, 11.0).style(
            |theme: &Theme, _| svg::Style {
                color: Some(super::styles::selector_chevron_color(theme, false)),
            },
        ));
    }

    button(content)
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = styles::quick_message_button_style(theme, status, false);
            if !enabled {
                style.text_color = theme.palette().text.scale_alpha(0.45);
            }
            style
        })
        .on_press_maybe(message)
        .into()
}
