//! 多模态快捷菜单与模型选择。

use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_skill_hub::skill_hub::LoadState;
use desktop_ui_controls::overlays::AboveOverlay;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_api_media::{MediaModel, MediaModelSelection, MediaOperation};

use super::quick_message::Message;
use super::quick_messages::{QuickInput, dropdown_icon};
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
    let trigger =
        button(row![text(label).size(12), dropdown_icon()].spacing(5).align_y(Alignment::Center))
            .padding([6, 10])
            .style(move |theme, status| {
                styles::quick_message_group_button_style(theme, status, open)
            })
            .on_press(Message::MediaShortcutMenuChanged(!open));

    let (items, width) = match app.media_models {
        Some(picker) => (model_list(app.i18n, picker), 304.0),
        None => (operations(app), 164.0),
    };
    let popup = container(items)
        .padding(6)
        .width(Length::Fixed(width))
        .style(styles::quick_message_menu_style);
    AboveOverlay::new(trigger, popup)
        .show(open)
        .gap(2.0)
        .block_background_events(true)
        .on_close(Message::MediaShortcutMenuChanged(false))
        .into()
}

fn operations(app: QuickInput<'_>) -> Element<'_, Message> {
    let can_select_skill = !matches!(app.installed, LoadState::Loading { .. });
    let mut items = column![
        item(
            app.i18n.__("生图"),
            Icon::Image,
            Some(Message::OpenMediaModelPicker(MediaOperation::ImageGenerate))
        ),
        item(
            app.i18n.__("生视频"),
            Icon::ArrowRight,
            Some(Message::OpenMediaModelPicker(MediaOperation::VideoGenerate))
        ),
        item(
            app.i18n.__("识图"),
            Icon::Eye,
            Some(Message::OpenMediaModelPicker(MediaOperation::ImageUnderstand))
        ),
    ]
    .spacing(3);
    for (label, icon, prompt) in PROMPTS {
        items = items.push(item(
            app.i18n.__(label),
            icon,
            Some(Message::MediaPromptSelected(app.i18n.__(prompt).to_owned())),
        ));
    }
    items
        .push(item(
            app.i18n.__("识矩剪辑"),
            Icon::Scissors,
            can_select_skill.then_some(Message::SelectVideoEditorSkill),
        ))
        .push(item(
            app.i18n.__("应用开发"),
            Icon::Code,
            can_select_skill.then_some(Message::SelectAppDevelopmentSkill),
        ))
        .push(item(
            app.i18n.__("应用管理"),
            Icon::Sliders,
            can_select_skill.then_some(Message::SelectAppManagementSkill),
        ))
        .into()
}

fn model_list(i18n: I18n, picker: MediaModelPicker<'_>) -> Element<'_, Message> {
    let title = match picker.operation {
        MediaOperation::ImageGenerate => "生图模型",
        MediaOperation::VideoGenerate => "生视频模型",
        MediaOperation::ImageUnderstand => "识图模型",
        _ => return column![].into(),
    };
    let back = button(super::icons::icon_svg(Icon::ChevronLeft, 13.0))
        .padding([5, 7])
        .style(|theme, status| styles::quick_message_group_button_style(theme, status, false))
        .on_press(Message::BackFromMediaModelPicker);
    let header = row![back, text(i18n.__(title)).size(13)].spacing(7).align_y(Alignment::Center);
    let auto = item(i18n.__("自动选择"), Icon::Sliders, Some(Message::MediaModelPicked(None)));
    let status = |label| container(text(i18n.__(label)).size(12)).padding([5, 8]);
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
                        column![text(&model.display_name).size(12), text(subtitle).size(10)]
                            .spacing(2),
                    )
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(|theme, status| styles::shiju_skill_button_style(theme, status, false))
                    .on_press(Message::MediaModelPicked(Some(
                        MediaModelSelection {
                            operation: picker.operation,
                            deployment_id: model.deployment_id.clone(),
                            model_id: model.model_id.clone(),
                        },
                    ))),
                );
            }
            container(scrollable(rows).direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(7),
            )))
            .max_height(224)
            .into()
        }
    };
    column![header, auto, list].spacing(5).width(Length::Fill).into()
}

fn item(label: &str, icon: Icon, message: Option<Message>) -> Element<'_, Message> {
    button(
        row![super::icons::icon_svg(icon, 13.0), text(label).size(12)]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([7, 10])
    .width(Length::Fill)
    .style(|theme, status| styles::quick_message_group_button_style(theme, status, false))
    .on_press_maybe(message)
    .into()
}
