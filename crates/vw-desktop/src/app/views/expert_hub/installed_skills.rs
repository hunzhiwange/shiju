use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, checkbox, column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::DesktopSkillCatalogEntryDto;

use super::public_skills::SCROLLBAR_WIDTH;
use super::styles;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style,
};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::LoadState;
use crate::app::{App, Message};

const CARD_GAP: f32 = 14.0;
const MIN_CARD_WIDTH: f32 = 340.0;

fn public_message(message: PublicSkillMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Public(message))
}

fn grid_columns(width: f32) -> usize {
    (((width + CARD_GAP) / (MIN_CARD_WIDTH + CARD_GAP)).floor() as usize).max(1)
}

fn badge(label: String) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(styles::meta_badge_style)
        .into()
}

fn source_badges(item: &DesktopSkillCatalogEntryDto) -> Element<'static, Message> {
    let mut badges = row![].spacing(6);
    if item.origin.is_some() {
        badges = badges.push(badge(format!("来源 {}", item.source)));
    } else {
        badges = badges.push(badge(format!("本地 {}", item.source)));
    }
    if let Some(package_type) = &item.package_type {
        badges = badges.push(badge(if package_type == "meta-skill" {
            "套件".to_string()
        } else {
            "技能".to_string()
        }));
    }
    if let Some(source_pack) = &item.source_pack {
        badges = badges.push(badge(format!("套件 {source_pack}")));
    }
    badges.wrap().into()
}

fn card_grid(
    app: &App,
    items: Vec<DesktopSkillCatalogEntryDto>,
) -> Element<'static, Message> {
    let selected = app.expert_hub.public_market.installed_selection.clone();
    let busy = app.expert_hub.public_market.batch_action.is_some();
    responsive(move |size| {
        let columns = grid_columns(size.width);
        let mut grid = column![].spacing(CARD_GAP);
        for chunk in items.chunks(columns) {
            let mut cards = row![].spacing(CARD_GAP);
            for item in chunk {
                let fixture_app = InstalledCardContext {
                    selected: selected.contains(&item.id),
                    busy,
                };
                cards = cards.push(installed_card_from_context(&fixture_app, item));
            }
            for _ in chunk.len()..columns {
                cards = cards.push(Space::new().width(Length::FillPortion(1)));
            }
            grid = grid.push(cards);
        }
        grid.into()
    })
    .width(Length::Fill)
    .into()
}

struct InstalledCardContext {
    selected: bool,
    busy: bool,
}

fn installed_card_from_context(
    context: &InstalledCardContext,
    item: &DesktopSkillCatalogEntryDto,
) -> Element<'static, Message> {
    let skill_id = item.id.clone();
    let selection = checkbox(context.selected)
        .on_toggle(move |selected| {
            public_message(PublicSkillMessage::SetInstalledSelected {
                skill_id: skill_id.clone(),
                selected,
            })
        })
        .style(settings_checkbox_style);
    let toggle_id = item.id.clone();
    let mut toggle = checkbox(item.enabled).style(settings_checkbox_style);
    if !context.busy {
        toggle = toggle.on_toggle(move |enabled| {
            public_message(PublicSkillMessage::ToggleInstalled {
                skill_id: toggle_id.clone(),
                enabled,
            })
        });
    }
    let title = if item.title.trim().is_empty() {
        item.id.clone()
    } else {
        item.title.clone()
    };
    container(
        column![
            row![
                selection,
                column![
                    text(title)
                        .size(16)
                        .font(Font {
                            weight: iced::font::Weight::Semibold,
                            ..Font::default()
                        }),
                    text(item.id.clone())
                        .size(11)
                        .style(styles::muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                row![
                    text(if item.enabled { "已启用" } else { "已关闭" }).size(12),
                    toggle,
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(item.description.clone())
                .size(13)
                .height(Length::Fixed(42.0)),
            source_badges(item),
            text(format!("资源 {} 项", item.resource_count))
                .size(11)
                .style(styles::muted_text_style),
        ]
        .spacing(11),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fixed(184.0))
    .style(styles::card_style)
    .into()
}

fn batch_button(
    label: &'static str,
    message: PublicSkillMessage,
    enabled: bool,
) -> Element<'static, Message> {
    let mut action = button(text(label).size(12))
        .padding([8, 12])
        .style(styles::quiet_button_style);
    if enabled {
        action = action.on_press(public_message(message));
    }
    action.into()
}

fn toolbar(app: &App, count: usize) -> Element<'static, Message> {
    let state = &app.expert_hub.public_market;
    let selected = state.installed_selection.len();
    let enabled = selected > 0 && state.batch_action.is_none();
    row![
        text(format!("已安装 {count} 项 · 已选择 {selected} 项"))
            .size(12)
            .style(styles::muted_text_style),
        Space::new().width(Length::Fill),
        batch_button(
            "全选",
            PublicSkillMessage::SelectAllInstalled,
            state.batch_action.is_none() && count > 0,
        ),
        batch_button(
            "清空",
            PublicSkillMessage::ClearInstalledSelection,
            state.batch_action.is_none() && selected > 0,
        ),
        batch_button(
            "启用",
            PublicSkillMessage::StartBatchEnabled(true),
            enabled,
        ),
        batch_button(
            "关闭",
            PublicSkillMessage::StartBatchEnabled(false),
            enabled,
        ),
        batch_button(
            "卸载",
            PublicSkillMessage::RequestBatchDelete,
            enabled,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.public_market;
    let installed = match &state.installed {
        LoadState::Loaded(items) => items
            .iter()
            .filter(|item| item.installed && item.source_path.is_some())
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let content = match &state.installed {
        LoadState::Idle | LoadState::Loading { .. } => {
            container(text("正在读取本地技能目录…").size(13))
                .height(Length::Fixed(180.0))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fixed(180.0))
                .into()
        }
        LoadState::Error(error) => column![
            text(format!("读取已安装技能失败：{error}")).size(13),
            button(text("重试").size(12))
                .on_press(public_message(PublicSkillMessage::OpenInstalled))
                .padding([8, 12])
                .style(styles::action_button_style),
        ]
        .spacing(10)
        .into(),
        LoadState::NeedsLogin => {
            container(text("本地技能目录无需登录").size(13)).into()
        }
        LoadState::Loaded(_) if installed.is_empty() => {
            container(text("当前范围尚未安装可管理技能").size(13))
                .height(Length::Fixed(180.0))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fixed(180.0))
                .into()
        }
        LoadState::Loaded(_) => card_grid(app, installed.clone()),
    };
    let page = column![
        row![
            column![
                text("我安装的技能")
                    .size(25)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::default()
                    }),
                text("管理当前目录范围内可启用、关闭或卸载的本地技能")
                    .size(13)
                    .style(styles::muted_text_style),
            ]
            .spacing(5)
            .width(Length::Fill),
            button(text("返回市场").size(12))
                .on_press(public_message(PublicSkillMessage::CloseInstalled))
                .padding([8, 13])
                .style(styles::quiet_button_style),
        ]
        .align_y(Alignment::Center),
        toolbar(app, installed.len()),
        content,
    ]
    .spacing(16)
    .width(Length::Fill);
    scrollable(page)
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(SCROLLBAR_WIDTH)
                .scroller_width(SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(super) fn with_delete_confirmation<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.expert_hub.public_market.delete_confirmation_open {
        return base;
    }
    let count = app.expert_hub.public_market.installed_selection.len();
    let cancel = public_message(PublicSkillMessage::CancelBatchDelete);
    let card = settings_modal_card(
        column![
            row![
                column![
                    text("确认批量卸载").size(22),
                    text(format!("将卸载当前选择的 {count} 个本地技能。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(6)
                .width(Length::Fill),
                settings_close_button(cancel.clone()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(
                text("只会删除 gateway 判定可管理的本地技能目录；失败项会保留选择，便于重试。")
                    .size(13),
            )
            .padding([14, 16])
            .width(Length::Fill)
            .style(styles::inset_panel_style),
            row![
                Space::new().width(Length::Fill),
                button(text("取消").size(13))
                    .on_press(cancel.clone())
                    .padding([9, 14])
                    .style(rounded_action_btn_style),
                button(text("确认卸载").size(13))
                    .on_press(public_message(PublicSkillMessage::ConfirmBatchDelete))
                    .padding([9, 14])
                    .style(danger_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(540.0))
    .max_width(580.0);
    settings_modal_overlay(Some(base), cancel, card)
}
