//! 构建识矩技能市场、已安装、系统、公司与我的技能列表。

use super::styles;
use crate::app::assets::Icon;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings_common::{
    icon_svg, settings_checkbox_style, settings_pick_list_menu_style, settings_pick_list_style,
};
use crate::app::message::{AccountMessage, SkillHubMessage};
use crate::app::skill_hub::{
    InstalledSkill, LoadState, MarketplaceSkill, MineFilterMenu, MineSkillStatus, Page, SkillTab,
    SkillVersionAction, SkillVersionDirection, SystemSkill, TenantSkill,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, button, checkbox, column, container, pick_list, responsive, row, scrollable, text,
    text_input,
};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Point, Shadow, Theme, Vector,
};
use std::collections::BTreeMap;
use std::fmt;

/// 技能列表统一使用的滚动条宽度。
pub(super) const SCROLLBAR_WIDTH: u32 = 4;
pub(super) const SCROLLBAR_GUTTER: u32 = 12;
const DESCRIPTION_PREVIEW_CHARS: usize = 72;
const MIN_CARD_WIDTH: f32 = 300.0;
const CARD_GAP: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TagFilterOption {
    id: Option<u64>,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// “我的技能”标签筛选菜单中的稳定选项。
pub(super) struct MineTagFilterOption {
    pub(super) id: u64,
    pub(super) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MarketTagFilterOption {
    pub(super) id: Option<u64>,
    pub(super) label: String,
}

impl fmt::Display for TagFilterOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub(super) fn market_search_messages(value: String) -> (Message, Message) {
    (
        Message::SkillHub(SkillHubMessage::SearchDraftChanged(value)),
        Message::SkillHub(SkillHubMessage::SearchSubmitted),
    )
}

/// 根据页码和总数生成可用的上一页、下一页消息。
pub(super) fn pagination_messages<T>(page: &Page<T>) -> (Option<Message>, Option<Message>) {
    let previous = (page.page > 1).then_some(Message::SkillHub(SkillHubMessage::PreviousPage));
    let next = (u64::from(page.page).saturating_mul(u64::from(page.page_size)) < page.total)
        .then_some(Message::SkillHub(SkillHubMessage::NextPage));
    (previous, next)
}

pub(super) fn access_action_message() -> Message {
    Message::Account(AccountMessage::StartBrowserLogin)
}

pub(super) fn detail_action_message(skill_id: u64) -> Message {
    Message::SkillHub(SkillHubMessage::OpenSkillDetail(skill_id))
}

pub(super) fn install_action_message(skill_id: u64, installed: bool) -> Message {
    Message::SkillHub(if installed {
        SkillHubMessage::UninstallSkill(skill_id)
    } else {
        SkillHubMessage::InstallSkill(skill_id)
    })
}

/// 仅为已安装且服务端标记可升级的市场技能生成快捷升级操作。
pub(super) fn marketplace_quick_upgrade_action(
    skill: &MarketplaceSkill,
    installed: &[InstalledSkill],
) -> Option<SkillVersionAction> {
    if skill.current_version_id == 0 {
        return None;
    }
    let installed =
        installed.iter().find(|item| item.skill_id == skill.id && item.upgrade_available)?;
    Some(SkillVersionAction {
        skill_id: skill.id,
        version_id: skill.current_version_id,
        target_version: installed.current_version.clone(),
        direction: SkillVersionDirection::Upgrade,
    })
}

pub(super) fn installed_quick_upgrade_message(skill: &InstalledSkill) -> Option<Message> {
    skill
        .upgrade_available
        .then_some(Message::SkillHub(SkillHubMessage::UpgradeSkill(skill.skill_id)))
}

/// 按字符数截断说明，避免切断多字节文本。
pub(super) fn description_preview(description: &str) -> String {
    let mut chars = description.chars();
    let preview = chars.by_ref().take(DESCRIPTION_PREVIEW_CHARS).collect::<String>();
    if chars.next().is_some() { format!("{preview}…") } else { preview }
}

pub(super) fn description_tooltip(description: String) -> Element<'static, Message> {
    container(text(description).size(12).width(Length::Fixed(320.0)))
        .padding([8, 10])
        .style(|_theme: &Theme| container::Style {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgba8(18, 18, 20, 0.96))),
            border: Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn tab_button(label: &'static str, tab: SkillTab, selected: bool) -> Element<'static, Message> {
    button(text(label).size(14))
        .on_press(Message::SkillHub(SkillHubMessage::SelectSkillTab(tab)))
        .padding([8, 12])
        .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
        .into()
}

pub(super) fn skill_tab_specs() -> &'static [(&'static str, SkillTab)] {
    &[
        ("技能市场", SkillTab::Marketplace),
        ("已安装技能", SkillTab::Installed),
        ("我的技能", SkillTab::Mine),
        ("安装识矩技能", SkillTab::Install),
    ]
}

fn tag_summary(tags: &[crate::app::skill_hub::SkillTag]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>().join(" · ")
    }
}

fn skill_card(
    skill_id: u64,
    name: String,
    description: String,
    metadata: Vec<String>,
    tags: String,
    actions: Vec<(&'static str, Message, bool)>,
) -> Element<'static, Message> {
    let preview = description_preview(&description);
    let description_view = Tooltip::new(
        container(text(preview).size(13).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(42.0)),
        description_tooltip(description),
        TooltipPosition::Top,
    )
    .gap(8);
    let initial = name.chars().next().unwrap_or('技').to_string();
    let badge = container(
        text(initial).size(15).font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
    )
    .width(Length::Fixed(38.0))
    .height(Length::Fixed(38.0))
    .center_x(Length::Fixed(38.0))
    .center_y(Length::Fixed(38.0))
    .style(styles::skill_badge_style);
    let title = text(name)
        .size(16)
        .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() })
        .width(Length::Fill);
    let detail_button = button(text("详情").size(12))
        .padding([6, 10])
        .style(styles::quiet_button_style)
        .on_press(detail_action_message(skill_id));
    let mut action_row = row![detail_button].spacing(6).align_y(Alignment::Center);
    for (label, message, pending) in actions {
        let mut action_button = button(text(if pending { "处理中…" } else { label }).size(12))
            .padding([6, 10])
            .style(styles::quiet_button_style);
        if !pending {
            action_button = action_button.on_press(message);
        }
        action_row = action_row.push(action_button);
    }
    let actions: Element<'static, Message> = action_row.into();
    let metadata = description_preview(&metadata.join(" · "));
    let mut content = column![
        row![badge, title, actions].spacing(10).align_y(Alignment::Center),
        description_view,
        text(metadata).size(11).style(styles::muted_text_style),
    ]
    .spacing(9);
    if !tags.is_empty() {
        content = content.push(
            text(format!("标签  {}", description_preview(&tags)))
                .size(11)
                .style(styles::muted_text_style),
        );
    }
    let card = container(content)
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fixed(178.0))
        .style(styles::card_style);
    card.into()
}

fn marketplace_card(
    skill: &MarketplaceSkill,
    installed: &[InstalledSkill],
    pending_skill_id: Option<u64>,
) -> Element<'static, Message> {
    let action = marketplace_quick_upgrade_action(skill, installed).map_or_else(
        || {
            (
                if skill.is_installed { "卸载" } else { "安装" },
                install_action_message(skill.id, skill.is_installed),
            )
        },
        |target| ("升级", Message::SkillHub(SkillHubMessage::ChangeSkillVersion(target))),
    );
    skill_card(
        skill.id,
        skill.name.clone(),
        skill.description.clone(),
        vec![format!("{} · {}", skill.slug, skill.format), format!("状态：{}", skill.status)],
        tag_summary(&skill.tags),
        vec![(action.0, action.1, pending_skill_id == Some(skill.id))],
    )
}

fn installed_card(
    skill: &InstalledSkill,
    pending_skill_id: Option<u64>,
) -> Element<'static, Message> {
    let upgrade = if skill.upgrade_available { " · 可升级" } else { "" };
    let mut actions = Vec::with_capacity(2);
    if let Some(message) = installed_quick_upgrade_message(skill) {
        actions.push(("升级", message, pending_skill_id == Some(skill.skill_id)));
    }
    actions.push((
        "卸载",
        install_action_message(skill.skill_id, true),
        pending_skill_id == Some(skill.skill_id),
    ));
    skill_card(
        skill.skill_id,
        skill.name.clone(),
        skill.description.clone(),
        vec![
            format!("{} · {}", skill.slug, skill.format),
            format!("已安装 {} · 当前 {}{upgrade}", skill.installed_version, skill.current_version),
        ],
        String::new(),
        actions,
    )
}

fn system_card(skill: &SystemSkill) -> Element<'static, Message> {
    skill_card(
        skill.id,
        skill.name.clone(),
        skill.description.clone(),
        vec![format!("slug：{}", skill.slug)],
        tag_summary(&skill.tags),
        Vec::new(),
    )
}

fn tenant_card(skill: &TenantSkill) -> Element<'static, Message> {
    let review = if skill.has_pending_review { " · 待审核" } else { "" };
    skill_card(
        skill.id,
        skill.name.clone(),
        skill.description.clone(),
        vec![format!("slug：{}", skill.slug), format!("状态：{}{review}", skill.status)],
        tag_summary(&skill.tags),
        Vec::new(),
    )
}

fn loading_view() -> Element<'static, Message> {
    let skeleton = || {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(92.0))
            .style(styles::card_style)
    };
    column![skeleton(), skeleton(), skeleton()].spacing(12).into()
}

fn empty_view(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(15))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn error_view(error: &str) -> Element<'static, Message> {
    container(
        column![
            text("加载失败").size(17),
            text(error.to_string()).size(13),
            button(text("重试"))
                .on_press(Message::SkillHub(SkillHubMessage::Retry))
                .padding([8, 14])
                .style(styles::action_button_style),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn needs_login_view() -> Element<'static, Message> {
    container(
        column![
            text("登录识矩账户后浏览技能").size(17),
            button(text("登录识矩"))
                .on_press(access_action_message())
                .padding([8, 14])
                .style(styles::action_button_style),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn pagination<T>(page: &Page<T>) -> Element<'static, Message> {
    let (previous, next) = pagination_messages(page);
    let mut previous_button =
        button(text("上一页")).padding([7, 12]).style(styles::action_button_style);
    if let Some(message) = previous {
        previous_button = previous_button.on_press(message);
    }
    let mut next_button =
        button(text("下一页")).padding([7, 12]).style(styles::action_button_style);
    if let Some(message) = next {
        next_button = next_button.on_press(message);
    }
    row![previous_button, text(format!("第 {} 页", page.page)).size(13), next_button,]
        .align_y(iced::Alignment::Center)
        .spacing(10)
        .into()
}

fn scrolling(content: Element<'static, Message>) -> Element<'static, Message> {
    scrollable(content)
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(SCROLLBAR_WIDTH)
                .scroller_width(SCROLLBAR_WIDTH)
                .spacing(SCROLLBAR_GUTTER - SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 根据可用宽度计算响应式卡片列数，并保证至少一列。
pub(super) fn grid_column_count(width: f32) -> usize {
    (((width + CARD_GAP) / (MIN_CARD_WIDTH + CARD_GAP)).floor() as usize).max(1)
}

fn card_grid<T, F>(items: Vec<T>, card: F) -> Element<'static, Message>
where
    T: 'static,
    F: Fn(&T) -> Element<'static, Message> + 'static,
{
    responsive(move |size| {
        let columns = grid_column_count(size.width);
        let mut grid = column![].spacing(CARD_GAP);
        for chunk in items.chunks(columns) {
            let mut cards = row![].spacing(CARD_GAP);
            for item in chunk {
                cards = cards.push(card(item));
            }
            for _ in chunk.len()..columns {
                cards = cards.push(Space::new().width(Length::FillPortion(1)));
            }
            grid = grid.push(cards);
        }
        scrolling(grid.into())
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn marketplace_content(app: &App) -> Element<'static, Message> {
    match &app.expert_hub.marketplace {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(),
        LoadState::Error(error) => error_view(error),
        LoadState::Loaded(page) if page.items.is_empty() => empty_view("未找到匹配的技能"),
        LoadState::Loaded(page) => {
            let pending =
                app.expert_hub.action.as_ref().map(|action| action.skill_id).or_else(|| {
                    app.expert_hub.version_confirmation.as_ref().map(|action| action.skill_id)
                });
            let installed = match &app.expert_hub.installed {
                LoadState::Loaded(items) => items.clone(),
                _ => Vec::new(),
            };
            card_grid(page.items.clone(), move |skill| marketplace_card(skill, &installed, pending))
        }
    }
}

fn installed_content(app: &App) -> Element<'static, Message> {
    match &app.expert_hub.installed {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(),
        LoadState::Error(error) => error_view(error),
        LoadState::Loaded(items) if items.is_empty() => empty_view("尚未安装技能"),
        LoadState::Loaded(items) => {
            let pending =
                app.expert_hub.action.as_ref().map(|action| action.skill_id).or_else(|| {
                    app.expert_hub.version_confirmation.as_ref().map(|action| action.skill_id)
                });
            card_grid(items.clone(), move |skill| installed_card(skill, pending))
        }
    }
}

fn system_content(app: &App) -> Element<'static, Message> {
    page_content(&app.expert_hub.system, "暂无系统技能", system_card)
}

fn tenant_content(
    state: &LoadState<Page<TenantSkill>>,
    empty_label: &'static str,
) -> Element<'static, Message> {
    page_content(state, empty_label, tenant_card)
}

fn page_content<T: Clone + 'static>(
    state: &LoadState<Page<T>>,
    empty_label: &'static str,
    card: fn(&T) -> Element<'static, Message>,
) -> Element<'static, Message> {
    match state {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(),
        LoadState::Error(error) => error_view(error),
        LoadState::Loaded(page) if page.items.is_empty() => empty_view(empty_label),
        LoadState::Loaded(page) => {
            column![card_grid(page.items.clone(), card), pagination(page),].spacing(10).into()
        }
    }
}

fn tag_filter_options<'a>(
    tags: impl Iterator<Item = &'a crate::app::skill_hub::SkillTag>,
    selected_id: Option<u64>,
) -> Vec<TagFilterOption> {
    let mut labels = BTreeMap::new();
    for tag in tags {
        labels.entry(tag.id).or_insert_with(|| tag.name.clone());
    }
    if let Some(selected_id) = selected_id {
        labels.entry(selected_id).or_insert_with(|| format!("标签 {selected_id}"));
    }
    std::iter::once(TagFilterOption { id: None, label: "全部标签".to_string() })
        .chain(labels.into_iter().map(|(id, label)| TagFilterOption { id: Some(id), label }))
        .collect()
}

fn current_tag_options(app: &App, selected_id: Option<u64>) -> Vec<TagFilterOption> {
    match app.expert_hub.skill_tab {
        SkillTab::System => match &app.expert_hub.system {
            LoadState::Loaded(page) => tag_filter_options(
                page.items.iter().flat_map(|skill| skill.tags.iter()),
                selected_id,
            ),
            _ => tag_filter_options(std::iter::empty(), selected_id),
        },
        SkillTab::Mine => match &app.expert_hub.mine {
            LoadState::Loaded(page) => tag_filter_options(
                page.items.iter().flat_map(|skill| skill.tags.iter()),
                selected_id,
            ),
            _ => tag_filter_options(std::iter::empty(), selected_id),
        },
        SkillTab::Marketplace | SkillTab::Installed | SkillTab::Install => {
            tag_filter_options(std::iter::empty(), selected_id)
        }
    }
}

/// 按服务端标签顺序合并多个列表中的可筛选标签。
pub(super) fn mine_tag_filter_options<'a>(
    tags: impl Iterator<Item = &'a crate::app::skill_hub::SkillTag>,
    selected_ids: &[u64],
) -> Vec<MineTagFilterOption> {
    let mut catalog_options = Vec::new();
    for tag in tags {
        if !catalog_options
            .iter()
            .any(|(_, option): &(i64, MineTagFilterOption)| option.id == tag.id)
        {
            catalog_options.push((
                tag.sort_order,
                MineTagFilterOption { id: tag.id, label: tag.name.clone() },
            ));
        }
    }
    catalog_options.sort_by_key(|(sort_order, _)| *sort_order);
    let mut options = catalog_options.into_iter().map(|(_, option)| option).collect::<Vec<_>>();
    for selected_id in selected_ids {
        if !options.iter().any(|option| option.id == *selected_id) {
            options.push(MineTagFilterOption {
                id: *selected_id,
                label: format!("标签 {selected_id}"),
            });
        }
    }
    options
}

/// 从账号范围目录构建菜单选项，并为未加载目录保留已选项。
pub(super) fn mine_catalog_filter_options(
    catalog: &LoadState<Vec<crate::app::skill_hub::SkillTag>>,
    selected_ids: &[u64],
) -> (Vec<MineTagFilterOption>, Option<&'static str>) {
    let tags = match catalog {
        LoadState::Loaded(tags) => tags.as_slice(),
        _ => &[],
    };
    let options = mine_tag_filter_options(tags.iter(), selected_ids);
    let empty_label = if !options.is_empty() {
        None
    } else {
        Some(match catalog {
            LoadState::Idle | LoadState::Loading { .. } => "标签目录加载中",
            LoadState::NeedsLogin => "登录后加载标签",
            LoadState::Error(_) => "标签目录加载失败",
            LoadState::Loaded(_) => "暂无可选标签",
        })
    };
    let warning = matches!(catalog, LoadState::Error(_)).then_some("标签目录加载失败");
    (options, warning.or(empty_label))
}

/// 从完整账号目录构建“全部 + 单选标签”，不使用当前市场页条目兜底。
pub(super) fn market_catalog_filter_options(
    catalog: &LoadState<Vec<crate::app::skill_hub::SkillTag>>,
) -> (Vec<MarketTagFilterOption>, Option<&'static str>) {
    let mut options = vec![MarketTagFilterOption { id: None, label: "全部".to_string() }];
    if let LoadState::Loaded(tags) = catalog {
        let mut tags = tags.iter().collect::<Vec<_>>();
        tags.sort_by_key(|tag| tag.sort_order);
        for tag in tags {
            if !options.iter().any(|option| option.id == Some(tag.id)) {
                options.push(MarketTagFilterOption { id: Some(tag.id), label: tag.name.clone() });
            }
        }
    }
    let feedback = match catalog {
        LoadState::Idle | LoadState::Loading { .. } => Some("标签目录加载中"),
        LoadState::NeedsLogin => Some("登录后加载标签"),
        LoadState::Error(_) => Some("标签目录加载失败"),
        LoadState::Loaded(tags) if tags.is_empty() => Some("暂无可选标签"),
        LoadState::Loaded(_) => None,
    };
    (options, feedback)
}

pub(super) fn market_tag_option_style(
    theme: &Theme,
    selected: bool,
) -> iced::widget::button::Style {
    let mut style = styles::quiet_button_style(theme, iced::widget::button::Status::Active);
    style.background =
        selected.then(|| Background::Color(theme.palette().primary.scale_alpha(0.18)));
    style.text_color = theme.palette().text;
    style.border.color = if selected {
        theme.palette().primary
    } else {
        theme.extended_palette().background.strong.color
    };
    style.border.width = 1.0;
    style.border.radius = 8.0.into();
    style
}

fn market_tag_row(
    label: &'static str,
    catalog: &LoadState<Vec<crate::app::skill_hub::SkillTag>>,
    selected_id: Option<u64>,
    message: fn(Option<u64>) -> SkillHubMessage,
) -> Element<'static, Message> {
    let (options, feedback) = market_catalog_filter_options(catalog);
    let mut tags =
        row![text(label).size(13).width(Length::Fixed(76.0))].spacing(8).align_y(Alignment::Center);
    for option in options {
        let selected = option.id == selected_id;
        let option_id = option.id;
        tags = tags.push(
            button(text(option.label).size(12))
                .on_press(Message::SkillHub(message(option_id)))
                .padding([6, 10])
                .style(move |theme, _| market_tag_option_style(theme, selected)),
        );
    }
    if let Some(feedback) = feedback {
        tags = tags.push(text(feedback).size(12).style(styles::muted_text_style));
    }
    tags.wrap().into()
}

pub(super) fn mine_named_tag_filter_label(name: &str, selected_ids: &[u64]) -> String {
    if selected_ids.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", selected_ids.len())
    }
}

pub(super) fn mine_status_filter_label(selected: &[MineSkillStatus]) -> String {
    format!("已选 {} 个状态", selected.len())
}

pub(super) fn mine_filter_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = {
        let background = theme.palette().background;
        background.r + background.g + background.b < 1.5
    };
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color
        } else {
            Color::WHITE
        })),
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.72 } else { 0.28 }),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.30 } else { 0.14 }),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub(super) fn mine_filter_menu_entry_style(
    theme: &Theme,
    selected: bool,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: selected.then(|| Background::Color(theme.palette().primary.scale_alpha(0.16))),
        text_color: Some(theme.palette().text),
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn mine_filter_trigger_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = styles::quiet_button_style(theme, status);
    style.border.radius = 8.0.into();
    style
}

fn mine_filter_trigger(
    label: String,
    open: bool,
    message: SkillHubMessage,
) -> Element<'static, Message> {
    let chevron = icon_svg(if open { Icon::ChevronUp } else { Icon::ChevronDown }, 12.0)
        .style(|theme: &Theme, _| iced::widget::svg::Style { color: Some(theme.palette().text) });
    button(
        row![text(label).size(13).width(Length::Fill), chevron]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .on_press(Message::SkillHub(message))
    .padding([9, 12])
    .width(Length::Fixed(160.0))
    .style(mine_filter_trigger_style)
    .into()
}

fn mine_tag_filter<'a>(
    app: &'a App,
    menu_kind: MineFilterMenu,
    selected_ids: &'a [u64],
    catalog: &'a LoadState<Vec<crate::app::skill_hub::SkillTag>>,
    label: &'static str,
    toggle_message: SkillHubMessage,
    toggle_option: fn(u64) -> SkillHubMessage,
) -> Element<'a, Message> {
    let open = app.expert_hub.mine_filter_menu == Some(menu_kind);
    let trigger =
        mine_filter_trigger(mine_named_tag_filter_label(label, selected_ids), open, toggle_message);
    let (tags, empty_label) = mine_catalog_filter_options(catalog, selected_ids);
    let mut entries = column![].spacing(2);
    for option in tags {
        let selected = selected_ids.contains(&option.id);
        let tag_id = option.id;
        entries = entries.push(
            container(
                checkbox(selected)
                    .label(option.label)
                    .on_toggle(move |_| Message::SkillHub(toggle_option(tag_id)))
                    .size(16)
                    .spacing(8)
                    .style(settings_checkbox_style)
                    .width(Length::Fill),
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme| mine_filter_menu_entry_style(theme, selected)),
        );
    }
    if let Some(empty_label) = empty_label {
        entries = entries.push(
            container(text(empty_label).size(13).style(styles::muted_text_style)).padding([8, 10]),
        );
    }
    let menu = container(
        scrollable(entries)
            .direction(Direction::Vertical(
                Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
            ))
            .width(Length::Fill),
    )
    .padding(6)
    .width(Length::Fixed(200.0))
    .max_height(240.0)
    .style(mine_filter_menu_style);

    PointBelowOverlay::new(trigger, menu)
        .show(open)
        .anchor(Point::new(0.0, 40.0))
        .gap(4.0)
        .on_close(Message::SkillHub(SkillHubMessage::CloseMineFilterMenu))
        .into()
}

fn mine_status_filter(app: &App) -> Element<'_, Message> {
    let open = app.expert_hub.mine_filter_menu == Some(MineFilterMenu::Statuses);
    let trigger = mine_filter_trigger(
        mine_status_filter_label(&app.expert_hub.mine_statuses_draft),
        open,
        SkillHubMessage::ToggleMineStatusFilterMenu,
    );
    let mut entries = column![].spacing(2);
    for status in MineSkillStatus::ALL {
        let selected = app.expert_hub.mine_statuses_draft.contains(&status);
        entries = entries.push(
            container(
                checkbox(selected)
                    .label(status.label())
                    .on_toggle(move |_| {
                        Message::SkillHub(SkillHubMessage::MineStatusToggled(status))
                    })
                    .size(16)
                    .spacing(8)
                    .style(settings_checkbox_style)
                    .width(Length::Fill),
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme| mine_filter_menu_entry_style(theme, selected)),
        );
    }
    let menu =
        container(entries).padding(6).width(Length::Fixed(180.0)).style(mine_filter_menu_style);

    PointBelowOverlay::new(trigger, menu)
        .show(open)
        .anchor(Point::new(0.0, 40.0))
        .gap(4.0)
        .on_close(Message::SkillHub(SkillHubMessage::CloseMineFilterMenu))
        .into()
}

fn list_search_input<'a>(value: &'a str) -> iced::widget::TextInput<'a, Message> {
    text_input("搜索技能名称或描述", value)
        .on_input(|value| Message::SkillHub(SkillHubMessage::FilterKeywordChanged(value)))
        .on_submit(Message::SkillHub(SkillHubMessage::FiltersSubmitted))
        .padding([10, 14])
        .style(styles::search_input_style)
        .width(Length::Fixed(360.0))
}

fn filter_buttons() -> Element<'static, Message> {
    row![
        button(text("搜索").size(12))
            .on_press(Message::SkillHub(SkillHubMessage::FiltersSubmitted))
            .padding([9, 16])
            .style(styles::action_button_style),
        button(text("重置").size(12))
            .on_press(Message::SkillHub(SkillHubMessage::FiltersReset))
            .padding([9, 16])
            .style(styles::quiet_button_style),
    ]
    .spacing(8)
    .into()
}

fn search_controls(app: &App) -> Element<'_, Message> {
    match app.expert_hub.skill_tab {
        SkillTab::Marketplace => {
            let (_, submitted) = market_search_messages(app.expert_hub.search_draft.clone());
            let search = row![
                text_input("搜索技能名称或描述", &app.expert_hub.search_draft)
                    .on_input(|value| market_search_messages(value).0)
                    .on_submit(submitted)
                    .padding([10, 14])
                    .style(styles::search_input_style)
                    .width(Length::Fixed(360.0)),
                button(text("搜索").size(12))
                    .on_press(Message::SkillHub(SkillHubMessage::SearchSubmitted))
                    .padding([9, 16])
                    .style(styles::action_button_style),
                button(text("重置").size(12))
                    .on_press(Message::SkillHub(SkillHubMessage::FiltersReset))
                    .padding([9, 16])
                    .style(styles::quiet_button_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap();
            column![
                search,
                market_tag_row(
                    "平台标签",
                    &app.expert_hub.platform_tag_catalog,
                    app.expert_hub.market_tag_id,
                    SkillHubMessage::MarketTagSelected,
                ),
                market_tag_row(
                    "公司标签",
                    &app.expert_hub.company_tag_catalog,
                    app.expert_hub.market_company_tag_id,
                    SkillHubMessage::MarketCompanyTagSelected,
                ),
            ]
            .spacing(8)
            .into()
        }
        SkillTab::Installed | SkillTab::Install => Space::new().height(Length::Shrink).into(),
        SkillTab::System => {
            let filter = &app.expert_hub.system_filter;
            let options = current_tag_options(app, filter.tag_id_draft);
            let selected = options.iter().find(|option| option.id == filter.tag_id_draft).cloned();
            let tag_picker = pick_list(options, selected, |option| {
                Message::SkillHub(SkillHubMessage::FilterTagChanged(option.id))
            })
            .placeholder("全部标签")
            .padding([9, 12])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fixed(150.0));
            row![list_search_input(&filter.keyword_draft), tag_picker, filter_buttons()]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap()
                .into()
        }
        SkillTab::Mine => row![
            list_search_input(&app.expert_hub.mine_filter.keyword_draft),
            mine_tag_filter(
                app,
                MineFilterMenu::Tags,
                &app.expert_hub.mine_tag_ids_draft,
                &app.expert_hub.platform_tag_catalog,
                "平台标签",
                SkillHubMessage::ToggleMineTagFilterMenu,
                SkillHubMessage::MineTagToggled,
            ),
            mine_tag_filter(
                app,
                MineFilterMenu::CompanyTags,
                &app.expert_hub.mine_company_tag_ids_draft,
                &app.expert_hub.company_tag_catalog,
                "公司标签",
                SkillHubMessage::ToggleMineCompanyTagFilterMenu,
                SkillHubMessage::MineCompanyTagToggled,
            ),
            mine_status_filter(app),
            filter_buttons(),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap()
        .into(),
    }
}

/// 按当前技能页签展示列表、筛选器和分页控件。
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let tabs = row(skill_tab_specs()
        .iter()
        .map(|(label, tab)| tab_button(label, *tab, app.expert_hub.skill_tab == *tab))
        .collect::<Vec<_>>())
    .spacing(6)
    .wrap();

    let search_controls = search_controls(app);

    let content = match app.expert_hub.skill_tab {
        SkillTab::Marketplace => marketplace_content(app),
        SkillTab::Installed => installed_content(app),
        SkillTab::System => system_content(app),
        SkillTab::Mine => tenant_content(&app.expert_hub.mine, "暂无我的技能"),
        SkillTab::Install => super::install_guide::view(app),
    };

    column![
        text("识矩技能")
            .size(24)
            .font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
        text("发现、安装和管理识矩技能").size(13).style(styles::muted_text_style),
        tabs,
        search_controls,
        content,
    ]
    .spacing(14)
    .height(Length::Fill)
    .into()
}
