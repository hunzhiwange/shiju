//! 构建识矩技能市场、已安装、系统、公司与我的技能列表。

pub(super) mod cards;
mod maintained;

use super::styles;
use super::{Message, ViewInput};
use crate::message::skill_hub::{SkillCatalogMessage, SkillHubMessage};
use desktop_assets::assets::Icon;
use desktop_skill_hub::skill_hub::{
    InstalledSkill, LoadState, MarketplaceSkill, MineFilterMenu, MineSkillStatus, Page, SkillTab,
    SkillVersionAction, SkillVersionDirection, SystemSkill,
};
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_select, settings_text_input,
};
use gateway_shiju_skill::ShijuSkillSource;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, checkbox, column, container, responsive, row, scrollable, text};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Point, Shadow, Theme, Vector,
};
use std::collections::BTreeMap;
use std::fmt;

/// 技能列表统一使用的滚动条宽度。
pub const SCROLLBAR_WIDTH: u32 = 4;
pub const SCROLLBAR_GUTTER: u32 = 12;
const DESCRIPTION_PREVIEW_CHARS: usize = 72;
const CARD_GAP: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TagFilterOption {
    id: Option<u64>,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// “我的技能”标签筛选菜单中的稳定选项。
pub struct MineTagFilterOption {
    pub id: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketTagFilterOption {
    pub id: Option<u64>,
    pub label: String,
}

impl fmt::Display for TagFilterOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub fn market_search_messages(value: String) -> (Message, Message) {
    (
        Message::SkillHub(SkillHubMessage::SearchDraftChanged(value)),
        Message::SkillHub(SkillHubMessage::SearchSubmitted),
    )
}

/// 根据页码和总数生成可用的上一页、下一页消息。
pub fn pagination_messages<T>(page: &Page<T>) -> (Option<Message>, Option<Message>) {
    let previous = (page.page > 1).then_some(Message::SkillHub(SkillHubMessage::PreviousPage));
    let next = (u64::from(page.page).saturating_mul(u64::from(page.page_size)) < page.total)
        .then_some(Message::SkillHub(SkillHubMessage::NextPage));
    (previous, next)
}

pub fn access_action_message() -> Message {
    Message::StartBrowserLogin
}

pub fn detail_action_message(skill_id: u64) -> Message {
    Message::SkillHub(SkillHubMessage::OpenSkillDetail(skill_id))
}

pub fn install_action_message(skill_id: u64, installed: bool) -> Message {
    Message::SkillHub(if installed {
        SkillHubMessage::UninstallSkill(skill_id)
    } else {
        SkillHubMessage::InstallSkill(skill_id)
    })
}

/// 仅为已安装且服务端标记可升级的市场技能生成快捷升级操作。
pub fn marketplace_quick_upgrade_action(
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

pub fn installed_quick_upgrade_message(skill: &InstalledSkill) -> Option<Message> {
    skill
        .upgrade_available
        .then_some(Message::SkillHub(SkillHubMessage::UpgradeSkill(skill.skill_id)))
}

/// 按字符数截断说明，避免切断多字节文本。
pub fn description_preview(description: &str) -> String {
    let mut chars = description.chars();
    let preview = chars.by_ref().take(DESCRIPTION_PREVIEW_CHARS).collect::<String>();
    if chars.next().is_some() { format!("{preview}…") } else { preview }
}

pub fn description_tooltip(description: String) -> Element<'static, Message> {
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

fn tab_button(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    tab: SkillTab,
    selected: bool,
) -> Element<'static, Message> {
    button(text(i18n.__(label)).size(14))
        .on_press(Message::SkillHub(SkillHubMessage::SelectSkillTab(tab)))
        .padding([8, 12])
        .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
        .into()
}

pub fn skill_tab_specs() -> &'static [(&'static str, SkillTab)] {
    &[
        ("技能广场", SkillTab::Marketplace),
        ("我安装的技能", SkillTab::Installed),
        ("我创建的技能", SkillTab::Mine),
    ]
}

fn tag_summary(tags: &[desktop_skill_hub::skill_hub::SkillTag]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>().join(" · ")
    }
}

fn system_card(
    i18n: desktop_i18n::I18n,
    skill: &SystemSkill,
    width: f32,
) -> Element<'static, Message> {
    cards::Card {
        id: skill.id,
        name: &skill.name,
        description: &skill.description,
        developer: None,
        updated_at: None,
        source: None,
        version: "",
        properties: format!("{} · {}", skill.slug, tag_summary(&skill.tags)),
        featured: false,
        installed: false,
        can_install: false,
        pending: false,
        upgrade: None,
        force_upgrade: false,
        width,
    }
    .view(i18n)
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

fn empty_view(i18n: desktop_i18n::I18n, label: &'static str) -> Element<'static, Message> {
    container(text(i18n.__(label)).size(15))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn error_view(i18n: desktop_i18n::I18n, error: &str) -> Element<'static, Message> {
    container(
        column![
            text(i18n.__("加载失败")).size(17),
            text(error.to_string()).size(13),
            button(text(i18n.__("重试")))
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

fn needs_login_view(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    container(
        column![
            text(i18n.__("登录识矩账户后浏览技能")).size(17),
            button(text(i18n.__("登录识矩")))
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

fn pagination<T>(i18n: desktop_i18n::I18n, page: &Page<T>) -> Element<'static, Message> {
    let (previous, next) = pagination_messages(page);
    let mut previous_button =
        button(text(i18n.__("上一页"))).padding([7, 12]).style(styles::action_button_style);
    if let Some(message) = previous {
        previous_button = previous_button.on_press(message);
    }
    let mut next_button =
        button(text(i18n.__("下一页"))).padding([7, 12]).style(styles::action_button_style);
    if let Some(message) = next {
        next_button = next_button.on_press(message);
    }
    row![
        previous_button,
        text(i18n.format("第 %u 页", &[page.page.into()]).expect("valid page number")).size(13),
        next_button,
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10)
    .into()
}

fn scrolling(content: Element<'static, Message>, load_more: bool) -> Element<'static, Message> {
    let scroll = scrollable(content)
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(SCROLLBAR_WIDTH)
                .scroller_width(SCROLLBAR_WIDTH)
                .spacing(SCROLLBAR_GUTTER - SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill);
    if load_more {
        scroll
            .on_scroll(|viewport| {
                let remaining = viewport.content_bounds().height
                    - viewport.bounds().height
                    - viewport.absolute_offset().y;
                if remaining < 200.0 {
                    catalog(SkillCatalogMessage::LoadMoreMarketplace)
                } else {
                    Message::None
                }
            })
            .into()
    } else {
        scroll.into()
    }
}

/// 根据可用宽度计算响应式卡片列数，并保证至少一列。
pub fn grid_column_count(width: f32) -> usize {
    if width >= 1080.0 {
        4
    } else if width >= 840.0 {
        3
    } else if width >= 560.0 {
        2
    } else {
        1
    }
}

fn card_grid<T>(
    items: &[T],
    columns: usize,
    card: impl Fn(&T) -> Element<'static, Message>,
) -> Element<'static, Message> {
    let mut grid = column![].spacing(CARD_GAP).width(Length::Fill);
    for chunk in items.chunks(columns) {
        let mut cards = row(chunk.iter().map(&card)).spacing(CARD_GAP);
        for _ in chunk.len()..columns {
            cards = cards.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(cards);
    }
    grid.into()
}

fn marketplace_content(
    app: &ViewInput<'_>,
    width: f32,
    columns: usize,
) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let state = app.expert_hub;
    match &state.marketplace {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(i18n),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(page) => {
            let installed = match &state.installed {
                LoadState::Loaded(items) => items.as_slice(),
                _ => &[],
            };
            let mut content = column![].spacing(16).width(Length::Fill);
            if page.items.is_empty() {
                content = content.push(empty_view(i18n, i18n.__("未找到匹配的技能")));
            } else {
                content = content.push(card_grid(&page.items, columns, |skill| {
                    cards::marketplace(
                        i18n,
                        skill,
                        installed,
                        state.skill_operations_busy(),
                        false,
                        column_width(width, columns),
                    )
                }));
            }
            if let Some(error) = &state.market_more_error {
                content = content.push(text(error.render(i18n)).size(13).style(cards::muted_text));
            }
            if page.has_more && page.next_cursor.is_some() {
                let label = if state.market_loading_more { "加载中…" } else { "加载更多" };
                content = content.push(
                    button(text(i18n.__(label)).size(13))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            (!state.market_loading_more)
                                .then_some(catalog(SkillCatalogMessage::LoadMoreMarketplace)),
                        ),
                );
            }
            content.into()
        }
    }
}

fn installed_content(app: &ViewInput<'_>, width: f32, columns: usize) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let state = app.expert_hub;
    match &state.installed {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(i18n),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(items) if items.is_empty() => empty_view(i18n, i18n.__("尚未安装技能")),
        LoadState::Loaded(items) => {
            let search = state.installed_search.trim().to_lowercase();
            let matches = items
                .iter()
                .filter(|item| {
                    search.is_empty()
                        || item.name.to_lowercase().contains(&search)
                        || item.description.to_lowercase().contains(&search)
                })
                .collect::<Vec<_>>();
            if matches.is_empty() {
                return empty_view(i18n, i18n.__("未找到匹配的技能"));
            }
            card_grid(&matches, columns, |skill| {
                cards::installed(
                    i18n,
                    skill,
                    state.skill_operations_busy(),
                    column_width(width, columns),
                )
            })
        }
    }
}

fn system_content(app: &ViewInput<'_>, width: f32, columns: usize) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    match &app.expert_hub.system {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => needs_login_view(i18n),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(page) if page.items.is_empty() => {
            empty_view(i18n, i18n.__("暂无系统技能"))
        }
        LoadState::Loaded(page) => column![
            card_grid(&page.items, columns, |skill| system_card(
                i18n,
                skill,
                column_width(width, columns)
            )),
            pagination(i18n, page),
        ]
        .spacing(16)
        .into(),
    }
}

fn tag_filter_options<'a>(
    i18n: desktop_i18n::I18n,
    tags: impl Iterator<Item = &'a desktop_skill_hub::skill_hub::SkillTag>,
    selected_id: Option<u64>,
) -> Vec<TagFilterOption> {
    let mut labels = BTreeMap::new();
    for tag in tags {
        labels.entry(tag.id).or_insert_with(|| tag.name.clone());
    }
    if let Some(selected_id) = selected_id {
        labels.entry(selected_id).or_insert_with(|| {
            i18n.format("标签 %s", &[(selected_id).to_string().into()])
                .expect("valid display template")
        });
    }
    std::iter::once(TagFilterOption { id: None, label: i18n.__("全部标签").to_string() })
        .chain(labels.into_iter().map(|(id, label)| TagFilterOption { id: Some(id), label }))
        .collect()
}

fn current_tag_options(app: &ViewInput<'_>, selected_id: Option<u64>) -> Vec<TagFilterOption> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    match app.expert_hub.skill_tab {
        SkillTab::System => match &app.expert_hub.system {
            LoadState::Loaded(page) => tag_filter_options(
                i18n,
                page.items.iter().flat_map(|skill| skill.tags.iter()),
                selected_id,
            ),
            _ => tag_filter_options(i18n, std::iter::empty(), selected_id),
        },
        SkillTab::Mine => match &app.expert_hub.mine {
            LoadState::Loaded(page) => tag_filter_options(
                i18n,
                page.items.iter().flat_map(|skill| skill.tags.iter()),
                selected_id,
            ),
            _ => tag_filter_options(i18n, std::iter::empty(), selected_id),
        },
        SkillTab::Marketplace | SkillTab::Installed | SkillTab::Install => {
            tag_filter_options(i18n, std::iter::empty(), selected_id)
        }
    }
}

/// 按服务端标签顺序合并多个列表中的可筛选标签。
pub fn mine_tag_filter_options<'a>(
    i18n: desktop_i18n::I18n,
    tags: impl Iterator<Item = &'a desktop_skill_hub::skill_hub::SkillTag>,
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
                label: i18n
                    .format("标签 %s", &[(selected_id).to_string().into()])
                    .expect("valid display template"),
            });
        }
    }
    options
}

/// 从账号范围目录构建菜单选项，并为未加载目录保留已选项。
pub fn mine_catalog_filter_options(
    i18n: desktop_i18n::I18n,
    catalog: &LoadState<Vec<desktop_skill_hub::skill_hub::SkillTag>>,
    selected_ids: &[u64],
) -> (Vec<MineTagFilterOption>, Option<&'static str>) {
    let tags = match catalog {
        LoadState::Loaded(tags) => tags.as_slice(),
        _ => &[],
    };
    let options = mine_tag_filter_options(i18n, tags.iter(), selected_ids);
    let empty_label = if !options.is_empty() {
        None
    } else {
        Some(match catalog {
            LoadState::Idle | LoadState::Loading { .. } => i18n.__("标签目录加载中"),
            LoadState::NeedsLogin => i18n.__("登录后加载标签"),
            LoadState::Error(_) => i18n.__("标签目录加载失败"),
            LoadState::Loaded(_) => i18n.__("暂无可选标签"),
        })
    };
    let warning = matches!(catalog, LoadState::Error(_)).then_some(i18n.__("标签目录加载失败"));
    (options, warning.or(empty_label))
}

/// 从完整账号目录构建“全部 + 单选标签”，不使用当前市场页条目兜底。
pub fn market_catalog_filter_options(
    i18n: desktop_i18n::I18n,
    catalog: &LoadState<Vec<desktop_skill_hub::skill_hub::SkillTag>>,
) -> (Vec<MarketTagFilterOption>, Option<&'static str>) {
    let mut options =
        vec![MarketTagFilterOption { id: None, label: i18n.__("全部").to_string() }];
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
        LoadState::Idle | LoadState::Loading { .. } => Some(i18n.__("标签目录加载中")),
        LoadState::NeedsLogin => Some(i18n.__("登录后加载标签")),
        LoadState::Error(_) => Some(i18n.__("标签目录加载失败")),
        LoadState::Loaded(tags) if tags.is_empty() => Some(i18n.__("暂无可选标签")),
        LoadState::Loaded(_) => None,
    };
    (options, feedback)
}

pub fn market_tag_option_style(theme: &Theme, selected: bool) -> iced::widget::button::Style {
    let mut style = styles::quiet_button_style(theme, iced::widget::button::Status::Active);
    let palette = theme.extended_palette();
    let pair = if selected { palette.primary.weak } else { palette.background.weak };
    style.background = Some(Background::Color(pair.color));
    style.text_color = pair.text;
    style.border.color = if selected {
        theme.palette().primary
    } else {
        theme.extended_palette().background.strong.color
    };
    style.border.width = 1.0;
    style.border.radius = 8.0.into();
    style
}

pub fn mine_named_tag_filter_label(name: &str, selected_ids: &[u64]) -> String {
    if selected_ids.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", selected_ids.len())
    }
}

pub fn mine_status_filter_label(i18n: desktop_i18n::I18n, selected: &[MineSkillStatus]) -> String {
    i18n.format("已选 %u 个状态", &[selected.len().into()]).expect("valid filter count")
}

pub fn mine_filter_menu_style(theme: &Theme) -> iced::widget::container::Style {
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

pub fn mine_filter_menu_entry_style(
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
    app: &ViewInput<'a>,
    menu_kind: MineFilterMenu,
    selected_ids: &'a [u64],
    catalog: &'a LoadState<Vec<desktop_skill_hub::skill_hub::SkillTag>>,
    label: &'static str,
    toggle_message: SkillHubMessage,
    toggle_option: fn(u64) -> SkillHubMessage,
) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let open = app.expert_hub.mine_filter_menu == Some(menu_kind);
    let trigger = mine_filter_trigger(
        mine_named_tag_filter_label(i18n.__(label), selected_ids),
        open,
        toggle_message,
    );
    let (tags, empty_label) = mine_catalog_filter_options(i18n, catalog, selected_ids);
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
    entries = entries.push(
        button(text(i18n.__("应用筛选")).size(13))
            .padding([7, 10])
            .style(primary_action_btn_style)
            .on_press(Message::SkillHub(SkillHubMessage::FiltersSubmitted)),
    );
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

fn list_search_input(
    i18n: desktop_i18n::I18n,
    value: &str,
) -> iced::widget::TextInput<'static, Message> {
    settings_text_input(i18n.__("搜索技能名称或描述"), value)
        .on_input(|value| Message::SkillHub(SkillHubMessage::FilterKeywordChanged(value)))
        .on_submit(Message::SkillHub(SkillHubMessage::FiltersSubmitted))
        .width(Length::Fixed(360.0))
}

fn filter_buttons(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    row![
        button(text(i18n.__("搜索")).size(12))
            .on_press(Message::SkillHub(SkillHubMessage::FiltersSubmitted))
            .padding([9, 16])
            .style(styles::action_button_style),
        button(text(i18n.__("重置")).size(12))
            .on_press(Message::SkillHub(SkillHubMessage::FiltersReset))
            .padding([9, 16])
            .style(styles::quiet_button_style),
    ]
    .spacing(8)
    .into()
}

fn search_box(
    i18n: desktop_i18n::I18n,
    value: &str,
    width: f32,
    change: fn(String) -> SkillHubMessage,
    submit: Option<SkillHubMessage>,
) -> Element<'static, Message> {
    let mut field = settings_text_input(i18n.__("搜索技能名称或用途"), value)
        .on_input(move |value| Message::SkillHub(change(value)))
        .style(|theme, status| {
            let mut style = desktop_ui_controls::system_settings_common::settings_text_input_style(
                theme, status,
            );
            style.background = cards::surface(theme).into();
            if !matches!(status, iced::widget::text_input::Status::Focused { .. }) {
                style.border.width = 0.0;
            }
            style.placeholder = cards::muted(theme);
            style.value = cards::ink(theme);
            style
        });
    let icon = icon_svg(Icon::Search, 16.0)
        .style(|theme, _| iced::widget::svg::Style { color: Some(cards::muted(theme)) });
    let icon: Element<'_, Message> = if let Some(submit) = submit {
        let submit = Message::SkillHub(submit);
        field = field.on_submit(submit.clone());
        button(icon).padding([10, 0]).style(button::text).on_press(submit).into()
    } else {
        icon.into()
    };
    container(row![icon, field].align_y(Alignment::Center))
        .padding([0, 12])
        .width(Length::Fixed(width))
        .height(42)
        .align_y(Alignment::Center)
        .style(|theme| cards::panel(theme, 8.0))
        .into()
}

fn heading(
    i18n: desktop_i18n::I18n,
    title: &'static str,
    description: &'static str,
) -> Element<'static, Message> {
    column![
        text(i18n.__(title)).font(cards::bold()).size(24),
        text(i18n.__(description)).size(14).style(cards::muted_text),
    ]
    .spacing(4)
    .into()
}

fn category_button(
    label: String,
    selected: bool,
    status_tab: bool,
    action: SkillHubMessage,
) -> Element<'static, Message> {
    button(text(label).size(13).font(cards::bold()))
        .padding([9, 14])
        .on_press(Message::SkillHub(action))
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            style.background = if status_tab && (selected || hovered) {
                Some(cards::accent(theme).scale_alpha(0.10).into())
            } else if selected || hovered {
                Some(cards::surface(theme).into())
            } else {
                None
            };
            style.text_color = if status_tab && selected {
                cards::accent(theme)
            } else if selected {
                cards::ink(theme)
            } else {
                cards::muted(theme)
            };
            style.border = Border { radius: 8.0.into(), ..Default::default() };
            style
        })
        .into()
}

fn filter_strip(theme: &Theme) -> container::Style {
    let color = if cards::dark(theme) {
        theme.extended_palette().background.weak.color
    } else {
        Color::from_rgb8(239, 237, 231)
    };
    container::Style {
        background: Some(color.into()),
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

#[derive(Clone, PartialEq)]
struct SourceFilterOption {
    source: Option<ShijuSkillSource>,
    label: &'static str,
}

impl fmt::Display for SourceFilterOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label)
    }
}

fn marketplace_filters(app: &ViewInput<'_>, width: f32) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let state = app.expert_hub;
    let (platform, platform_feedback) =
        market_catalog_filter_options(i18n, &state.platform_tag_catalog);
    let (company, company_feedback) =
        market_catalog_filter_options(i18n, &state.company_tag_catalog);
    let mut categories = row![category_button(
        i18n.__("全部").to_string(),
        state.market_tag_id.is_none() && state.market_company_tag_id.is_none(),
        false,
        SkillHubMessage::Catalog(SkillCatalogMessage::MarketCategoriesReset),
    )];
    for (options, selected, select) in [
        (platform, state.market_tag_id, SkillHubMessage::MarketTagSelected as fn(_) -> _),
        (
            company,
            state.market_company_tag_id,
            SkillHubMessage::MarketCompanyTagSelected as fn(_) -> _,
        ),
    ] {
        for option in options.into_iter().filter(|option| option.id.is_some()) {
            categories = categories.push(category_button(
                option.label,
                selected == option.id,
                false,
                select(option.id),
            ));
        }
    }
    let categories = container(categories.spacing(3).wrap()).padding(4).style(filter_strip);
    let options =
        [None, Some(ShijuSkillSource::Marketplace), Some(ShijuSkillSource::Tenant)].map(|source| {
            (
                source,
                if source.is_none() {
                    i18n.__("全部来源")
                } else {
                    cards::source_label(i18n, source)
                },
            )
        });
    let options = options.map(|(source, label)| SourceFilterOption { source, label });
    let selected = options.iter().find(|option| option.source == state.market_source).cloned();
    let source_width =
        if app.desktop_locale == config_ui::DesktopLocale::English { 184.0 } else { 144.0 };
    let source = settings_select(options, selected, |option| {
        catalog(SkillCatalogMessage::MarketSourceSelected(option.source))
    })
    .width(source_width)
    .height(42.0);
    let search_width = (width - source_width - 8.0).clamp(1.0, 244.0);
    let search = search_box(
        i18n,
        &state.search_draft,
        search_width,
        SkillHubMessage::SearchDraftChanged,
        Some(SkillHubMessage::SearchSubmitted),
    );
    let controls = row![search, source].spacing(6).align_y(Alignment::Center);
    let toolbar: Element<'_, Message> = if width >= 1060.0 {
        row![
            categories.max_width(width - search_width - source_width - 18.0),
            Space::new().width(Length::Fill),
            controls,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    } else {
        column![categories, controls].spacing(10).into()
    };
    let mut result = column![toolbar].spacing(6);
    if let Some(feedback) = platform_feedback.or(company_feedback) {
        result = result.push(text(feedback).size(12).style(cards::muted_text));
    }
    result.into()
}

fn marketplace(app: &ViewInput<'_>, width: f32, columns: usize) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let state = app.expert_hub;
    let settings = button(text(i18n.__("技能配置")).size(14))
        .padding([10, 16])
        .style(|theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            style.border.radius = 8.0.into();
            style
        })
        .on_press(Message::OpenSkillSettings);
    let install = button(
        row![
            icon_svg(Icon::CloudDownload, 16.0)
                .style(|_, _| iced::widget::svg::Style { color: Some(Color::WHITE) }),
            text(i18n.__("安装识矩技能")).size(14).font(cards::bold()),
        ]
        .align_y(Alignment::Center)
        .spacing(9),
    )
    .padding([10, 16])
    .style(|theme, status| {
        let mut style = primary_action_btn_style(theme, status);
        style.border.radius = 8.0.into();
        if !cards::dark(theme) {
            let color = match status {
                button::Status::Hovered => Color::from_rgb8(186, 79, 42),
                button::Status::Pressed => Color::from_rgb8(170, 68, 34),
                _ => Color::from_rgb8(198, 84, 46),
            };
            style.background = Some(color.into());
            style.text_color = Color::WHITE;
        }
        style
    })
    .on_press(Message::SkillHub(SkillHubMessage::OpenInstallGuide));
    let actions = row![settings, install].spacing(8).align_y(Alignment::Center).wrap();
    let title = heading(i18n, "技能广场", "发现并安装当前账号可使用的技能。");
    let header: Element<'_, Message> = if width >= 610.0 {
        row![title, Space::new().width(Length::Fill), actions].align_y(Alignment::Center).into()
    } else {
        column![title, actions].spacing(12).into()
    };
    let mut content = column![header].spacing(28).width(Length::Fill);
    if !state.featured_skills.is_empty() {
        let refresh = button(
            row![
                icon_svg(Icon::ArrowRepeat, 14.0).style(|theme, _| iced::widget::svg::Style {
                    color: Some(cards::muted(theme))
                }),
                text(i18n.__(if state.featured_loading { "加载中…" } else { "换一换" })).size(13),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([8, 10])
        .style(button::text)
        .on_press_maybe(
            (!state.featured_loading).then_some(catalog(SkillCatalogMessage::RefreshFeatured)),
        );
        let installed = match &state.installed {
            LoadState::Loaded(items) => items.as_slice(),
            _ => &[],
        };
        content = content.push(
            column![
                row![
                    text(i18n.__("精选技能")).size(18).font(cards::bold()),
                    Space::new().width(Length::Fill),
                    refresh
                ]
                .align_y(Alignment::Center),
                card_grid(&state.featured_skills, columns, |skill| {
                    cards::marketplace(
                        i18n,
                        skill,
                        installed,
                        state.skill_operations_busy(),
                        true,
                        column_width(width, columns),
                    )
                }),
            ]
            .spacing(14),
        );
    }
    content
        .push(
            column![
                text(i18n.__("全部技能")).size(18).font(cards::bold()),
                marketplace_filters(app, width),
                marketplace_content(app, width, columns),
            ]
            .spacing(18),
        )
        .into()
}

fn column_width(width: f32, columns: usize) -> f32 {
    (width - CARD_GAP * (columns - 1) as f32) / columns as f32
}

fn installed(app: &ViewInput<'_>, width: f32, columns: usize) -> Element<'static, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let field_width = column_width(width, columns).min(300.0);
    let search = search_box(
        i18n,
        &app.expert_hub.installed_search,
        field_width,
        |value| SkillHubMessage::Catalog(SkillCatalogMessage::InstalledSearchChanged(value)),
        None,
    );
    let toolbar = row![search, super::upgrade_actions::view(app)]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap();
    column![
        heading(i18n, "我安装的技能", "快速查看已安装技能，及时升级到最新版本。"),
        toolbar,
        installed_content(app, width, columns),
    ]
    .spacing(20)
    .width(Length::Fill)
    .into()
}

/// 按当前技能页签展示列表、筛选器和分页控件。
pub fn view<'a>(app: &ViewInput<'a>, install: Element<'a, Message>) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let input = ViewInput {
        expert_hub: app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: app.app_theme,
    };
    let tabs = row(skill_tab_specs()
        .iter()
        .map(|(label, tab)| tab_button(i18n, label, *tab, app.expert_hub.skill_tab == *tab)))
    .spacing(6)
    .wrap();

    let content: Element<'_, Message> = if app.expert_hub.skill_tab == SkillTab::Install {
        install
    } else {
        responsive(move |size| {
            let width = (size.width - SCROLLBAR_GUTTER as f32).max(1.0);
            let columns = grid_column_count(width);
            let content = match input.expert_hub.skill_tab {
                SkillTab::Marketplace => marketplace(&input, width, columns),
                SkillTab::Installed => installed(&input, width, columns),
                SkillTab::Mine => maintained::view(&input, width),
                SkillTab::System => {
                    let filter = &input.expert_hub.system_filter;
                    let options = current_tag_options(&input, filter.tag_id_draft);
                    let selected = options.iter().find(|option| option.id == filter.tag_id_draft).cloned();
                    let tags = settings_select(options, selected, |option| {
                        Message::SkillHub(SkillHubMessage::FilterTagChanged(option.id))
                    }).width(150);
                    column![heading(i18n, "系统技能", "发现、安装和管理识矩技能"),
                        row![list_search_input(i18n, &filter.keyword_draft), tags, filter_buttons(i18n)]
                            .spacing(8).align_y(Alignment::Center).wrap(),
                        system_content(&input, width, columns),
                    ].spacing(16).into()
                }
                SkillTab::Install => unreachable!("install guide is handled before layout"),
            };
            let state = input.expert_hub;
            let load_more = state.skill_tab == SkillTab::Marketplace
                && !state.market_loading_more && state.market_more_error.is_none()
                && matches!(&state.marketplace, LoadState::Loaded(page) if page.has_more && page.next_cursor.is_some());
            scrolling(content, load_more)
        }).into()
    };

    container(column![tabs, content].spacing(20).height(Length::Fill))
        .style(|theme| container::Style {
            text_color: Some(cards::ink(theme)),
            ..Default::default()
        })
        .into()
}

pub(crate) fn maintained_action_dialog(
    app: &ViewInput<'_>,
) -> Option<(Message, Element<'static, Message>)> {
    maintained::action_dialog(app)
}

fn catalog(message: SkillCatalogMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Catalog(message))
}
