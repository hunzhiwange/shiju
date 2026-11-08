//! 构建公共技能市场的推荐、检索、来源筛选与套件列表。

use std::fmt;

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::{
    SkillMarketSkillDto, SkillMarketSkillLocatorDto, SkillMarketSkillSetDto, SkillMarketSort,
    SkillMarketSourceDto,
};

use super::list::{description_preview, description_tooltip};
use super::styles;
use crate::app::components::system_settings_common::{settings_select, settings_text_input};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::{LoadState, PublicSkillMarketState, PublicSkillTab};
use crate::app::{App, Message};

/// 公共技能页面统一使用的滚动条宽度。
pub(super) const SCROLLBAR_WIDTH: u32 = 4;
const SCROLLBAR_GUTTER: u32 = 12;
const CARD_GAP: f32 = 16.0;
const MIN_CARD_WIDTH: f32 = 300.0;
const DESCRIPTION_CHARS: usize = 88;
const COORDINATE_PREVIEW_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceOption {
    source: String,
    label: String,
}

impl fmt::Display for SourceOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SortOption {
    sort: SkillMarketSort,
    label: &'static str,
}

impl fmt::Display for SortOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label)
    }
}

const SORT_OPTIONS: [SortOption; 4] = [
    SortOption { sort: SkillMarketSort::Score, label: "综合评分" },
    SortOption { sort: SkillMarketSort::Downloads, label: "下载量" },
    SortOption { sort: SkillMarketSort::UpdatedAt, label: "最近更新" },
    SortOption { sort: SkillMarketSort::Installs, label: "安装量" },
];

pub(super) fn public_tab_specs() -> &'static [(&'static str, PublicSkillTab)] {
    &[
        ("推荐", PublicSkillTab::Recommended),
        ("SkillHub", PublicSkillTab::SkillHub),
        ("套件", PublicSkillTab::Suites),
    ]
}

/// 根据可用宽度计算响应式卡片列数，并保证至少一列。
pub(super) fn grid_column_count(width: f32) -> usize {
    (((width + CARD_GAP) / (MIN_CARD_WIDTH + CARD_GAP)).floor() as usize).max(1)
}

/// 仅在成功加载多个来源时显示来源选择器。
pub(super) fn source_selector_visible(sources: &LoadState<Vec<SkillMarketSourceDto>>) -> bool {
    matches!(sources, LoadState::Loaded(items) if items.len() > 1)
}

fn public_message(message: PublicSkillMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Public(message))
}

fn tab_button(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    tab: PublicSkillTab,
    selected: bool,
) -> Element<'static, Message> {
    button(text(i18n.__(label)).size(14))
        .on_press(public_message(PublicSkillMessage::SelectTab(tab)))
        .padding([8, 16])
        .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
        .into()
}

pub(super) fn source_selector(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    if !source_selector_visible(&state.sources) {
        return Space::new().width(Length::Shrink).into();
    }
    let LoadState::Loaded(sources) = &state.sources else {
        return Space::new().width(Length::Shrink).into();
    };
    let options = sources
        .iter()
        .map(|source| SourceOption {
            source: source.source.clone(),
            label: source.display_name.clone(),
        })
        .collect::<Vec<_>>();
    let selected = options
        .iter()
        .find(|option| state.current_source.as_deref() == Some(&option.source))
        .cloned();
    settings_select(options, selected, |option| {
        public_message(PublicSkillMessage::SelectSource(option.source))
    })
    .placeholder(i18n.__("选择来源"))
    .width(Length::Fixed(160.0))
    .into()
}

fn search_header(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub.public_market;
    let tabs = row(public_tab_specs()
        .iter()
        .map(|(label, tab)| tab_button(i18n, label, *tab, state.tab == *tab))
        .collect::<Vec<_>>())
    .spacing(6)
    .width(Length::Fill)
    .wrap();
    let search = settings_text_input(i18n.__("搜索技能或套件"), &state.query.keyword_draft)
        .on_input(|query| public_message(PublicSkillMessage::QueryDraftChanged(query)))
        .on_submit(public_message(PublicSkillMessage::SubmitQuery))
        .width(Length::Fixed(340.0));
    column![
        row![
            column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("技能"))
                    .size(26)
                    .font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
                text(
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("发现适合当前工作的公开技能与套件")
                )
                .size(13)
                .style(styles::muted_text_style),
            ]
            .spacing(5)
            .width(Length::Fill),
            button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("我安装的")).size(13))
                .on_press(public_message(PublicSkillMessage::OpenInstalled))
                .padding([9, 14])
                .style(styles::quiet_button_style),
        ]
        .align_y(Alignment::Center),
        row![tabs, source_selector(i18n, state)].spacing(10).align_y(Alignment::Center),
        row![
            search,
            button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("搜索")).size(13))
                .on_press(public_message(PublicSkillMessage::SubmitQuery))
                .padding([10, 18])
                .style(styles::action_button_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(14)
    .into()
}

fn truncate(value: &str, limit: usize) -> String {
    let mut characters = value.chars();
    let text = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_some() { format!("{text}…") } else { text }
}

fn initial_badge(i18n: crate::app::i18n::I18n, name: &str) -> Element<'static, Message> {
    let initial = name
        .chars()
        .next()
        .unwrap_or_else(|| i18n.__("技能").chars().next().unwrap_or('S'))
        .to_string();
    container(
        text(initial).size(17).font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
    )
    .width(Length::Fixed(42.0))
    .height(Length::Fixed(42.0))
    .center_x(Length::Fixed(42.0))
    .center_y(Length::Fixed(42.0))
    .style(styles::skill_badge_style)
    .into()
}

fn skill_coordinate_line(namespace: &str, slug: &str, version: &str) -> Element<'static, Message> {
    let coordinate = format!("{namespace} / {slug}");
    let coordinate_preview = container(
        text(truncate(&coordinate, COORDINATE_PREVIEW_CHARS))
            .size(11)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(styles::muted_text_style),
    )
    .width(Length::Fill)
    .height(Length::Fixed(16.0))
    .clip(true);
    let coordinate_view =
        Tooltip::new(coordinate_preview, description_tooltip(coordinate), TooltipPosition::Top)
            .gap(8);

    row![
        coordinate_view,
        text(format!("· {version}"))
            .size(11)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(styles::muted_text_style),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn skill_card(
    i18n: crate::app::i18n::I18n,
    skill: &SkillMarketSkillDto,
    featured: bool,
) -> Element<'static, Message> {
    let locator = skill.coordinate.locator.clone();
    let metadata = format!(
        "{} · 下载 {} · 安装 {} · {:.1} 分",
        skill.author, skill.downloads, skill.installs, skill.rating
    );
    let description_view = Tooltip::new(
        container(text(description_preview(&skill.description)).size(13).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(44.0)),
        description_tooltip(skill.description.clone()),
        TooltipPosition::Top,
    )
    .gap(8);
    container(
        column![
            row![
                initial_badge(i18n, &skill.name),
                column![
                    text(truncate(&skill.name, 34))
                        .size(16)
                        .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
                    skill_coordinate_line(
                        &skill.coordinate.locator.namespace,
                        &skill.coordinate.locator.slug,
                        &skill.coordinate.version,
                    ),
                ]
                .spacing(4)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            description_view,
            text(truncate(&metadata, 70)).size(11).style(styles::muted_text_style),
            row![
                text(truncate(&skill.tags.join(" · "), 42))
                    .size(11)
                    .style(styles::muted_text_style)
                    .width(Length::Fill),
                button(text(i18n.__("详情")).size(12))
                    .on_press(skill_detail_message(locator))
                    .padding([6, 11])
                    .style(styles::quiet_button_style),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fixed(196.0))
    .style(
        move |theme| {
            if featured { styles::featured_card_style(theme) } else { styles::card_style(theme) }
        },
    )
    .into()
}

fn suite_card(
    i18n: crate::app::i18n::I18n,
    suite: &SkillMarketSkillSetDto,
) -> Element<'static, Message> {
    let source = suite.source.clone();
    let slug = suite.slug.clone();
    container(
        column![
            row![
                initial_badge(i18n, &suite.name),
                column![
                    text(truncate(&suite.name, 34))
                        .size(16)
                        .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
                    text(
                        i18n.format(
                            "%s 个成员 · %s",
                            &[
                                (suite.member_count).to_string().into(),
                                (suite.author).to_string().into()
                            ]
                        )
                        .expect("valid display template")
                    )
                    .size(11)
                    .style(styles::muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(truncate(&suite.description, DESCRIPTION_CHARS))
                .size(13)
                .height(Length::Fixed(56.0)),
            row![
                text(truncate(&suite.updated_at, 32))
                    .size(11)
                    .style(styles::muted_text_style)
                    .width(Length::Fill),
                button(text(i18n.__("查看套件")).size(12))
                    .on_press(public_message(PublicSkillMessage::OpenSuiteDetail { source, slug }))
                    .padding([6, 11])
                    .style(styles::quiet_button_style),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fixed(196.0))
    .style(styles::card_style)
    .into()
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
        grid.into()
    })
    .width(Length::Fill)
    .into()
}

pub(super) fn loading_view() -> Element<'static, Message> {
    column![
        container(Space::new())
            .height(Length::Fixed(120.0))
            .width(Length::Fill)
            .style(styles::card_style),
        container(Space::new())
            .height(Length::Fixed(120.0))
            .width(Length::Fill)
            .style(styles::card_style),
    ]
    .spacing(12)
    .into()
}

fn empty_view(i18n: crate::app::i18n::I18n, label: &'static str) -> Element<'static, Message> {
    container(text(i18n.__(label)).size(14).style(styles::muted_text_style))
        .width(Length::Fill)
        .height(Length::Fixed(180.0))
        .center_x(Length::Fill)
        .center_y(Length::Fixed(180.0))
        .into()
}

pub(super) fn error_view(i18n: crate::app::i18n::I18n, error: &str) -> Element<'static, Message> {
    container(
        column![
            text(i18n.__("加载失败")).size(16),
            text(truncate(error, 100)).size(12).style(styles::muted_text_style),
            button(text(i18n.__("重试")).size(12))
                .on_press(public_message(PublicSkillMessage::Retry))
                .padding([8, 14])
                .style(styles::action_button_style),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fixed(200.0))
    .center_x(Length::Fill)
    .center_y(Length::Fixed(200.0))
    .into()
}

fn skill_state_content(
    i18n: crate::app::i18n::I18n,
    state: &LoadState<Vec<SkillMarketSkillDto>>,
    featured: bool,
    empty: &'static str,
) -> Element<'static, Message> {
    match state {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => error_view(i18n, i18n.__("公开市场不需要登录，请重试")),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(skills) if skills.is_empty() => empty_view(i18n, empty),
        LoadState::Loaded(skills) => {
            card_grid(skills.clone(), move |skill| skill_card(i18n, skill, featured))
        }
    }
}

fn recommended_content(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    let all_content = skill_state_content(i18n, &state.recommended, false, i18n.__("暂无推荐技能"));
    column![
        text(i18n.__("推荐技能"))
            .size(19)
            .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
        all_content,
    ]
    .spacing(14)
    .into()
}

fn category_controls(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    let all_selected = state.query.category.is_none();
    let mut categories = row![
        button(text(i18n.__("全部")).size(12))
            .on_press(public_message(PublicSkillMessage::SelectCategory(None)))
            .padding([7, 12])
            .style(move |theme, status| styles::selected_tab_style(theme, status, all_selected)),
    ]
    .spacing(6);
    if let LoadState::Loaded(items) = &state.categories {
        for category in items {
            let slug = category.slug.clone();
            let selected = state.query.category.as_deref() == Some(&slug);
            categories = categories.push(
                button(text(format!("{} {}", category.name, category.skill_count)).size(12))
                    .on_press(public_message(PublicSkillMessage::SelectCategory(Some(slug))))
                    .padding([7, 12])
                    .style(move |theme, status| {
                        styles::selected_tab_style(theme, status, selected)
                    }),
            );
        }
    }
    let sort_options =
        SORT_OPTIONS.map(|option| SortOption { label: i18n.__(option.label), ..option });
    let selected_sort = sort_options.iter().copied().find(|option| option.sort == state.query.sort);
    row![
        categories.width(Length::Fill).wrap(),
        settings_select(sort_options, selected_sort, |option| {
            public_message(PublicSkillMessage::SelectSort(option.sort))
        })
        .width(Length::Fixed(130.0)),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn pagination(
    i18n: crate::app::i18n::I18n,
    page: u32,
    page_size: u32,
    total: u64,
) -> Element<'static, Message> {
    let mut previous =
        button(text(i18n.__("上一页")).size(12)).padding([7, 12]).style(styles::quiet_button_style);
    if page > 1 {
        previous = previous.on_press(public_message(PublicSkillMessage::PreviousPage));
    }
    let mut next =
        button(text(i18n.__("下一页")).size(12)).padding([7, 12]).style(styles::quiet_button_style);
    if u64::from(page) * u64::from(page_size) < total {
        next = next.on_press(public_message(PublicSkillMessage::NextPage));
    }
    row![
        previous,
        text(i18n.format("第 %u 页", &[page.into()]).expect("valid page number")).size(12),
        next
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn market_content(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    let content = match &state.market {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => error_view(i18n, i18n.__("公开市场不需要登录，请重试")),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(page) if page.skills.is_empty() => {
            empty_view(i18n, i18n.__("未找到匹配的技能"))
        }
        LoadState::Loaded(page) => column![
            card_grid(page.skills.clone(), move |skill| skill_card(i18n, skill, false)),
            pagination(i18n, page.page, page.page_size, page.total),
        ]
        .spacing(12)
        .into(),
    };
    column![category_controls(i18n, state), content].spacing(12).into()
}

pub(super) fn suites_content(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    let content = match &state.suites {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::NeedsLogin => error_view(i18n, i18n.__("公开市场不需要登录，请重试")),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(page) if page.skillsets.is_empty() => {
            empty_view(i18n, i18n.__("未找到匹配的套件"))
        }
        LoadState::Loaded(page) => column![
            card_grid(page.skillsets.clone(), move |suite| suite_card(i18n, suite)),
            pagination(i18n, page.page, page.page_size, page.total),
        ]
        .spacing(12)
        .into(),
    };
    column![suite_category_controls(i18n, state), content].spacing(12).into()
}

fn suite_category_controls(
    i18n: crate::app::i18n::I18n,
    state: &PublicSkillMarketState,
) -> Element<'static, Message> {
    // SkillHub expert-package scenes differ from individual skill categories.
    let categories = [
        (None, i18n.__("全部")),
        (Some("finance"), i18n.__("金融")),
        (Some("tech"), i18n.__("科技")),
        (Some("design"), i18n.__("设计")),
        (Some("marketing"), i18n.__("营销")),
        (Some("legal"), i18n.__("法律")),
        (Some("academic"), i18n.__("学术")),
        (Some("education"), i18n.__("教育")),
        (Some("hr"), i18n.__("人力")),
        (Some("ecommerce"), i18n.__("电商")),
        (Some("media"), i18n.__("媒体")),
        (Some("healthcare"), i18n.__("医疗")),
        (Some("mysticism"), i18n.__("玄学")),
    ];
    row(categories.into_iter().map(|(category, label)| {
        let selected = state.suites_category.as_deref() == category;
        button(text(i18n.__(label)).size(12))
            .on_press(public_message(PublicSkillMessage::SelectSuiteCategory(
                category.map(str::to_string),
            )))
            .padding([7, 12])
            .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
            .into()
    }))
    .spacing(6)
    .wrap()
    .into()
}

/// 构建公共市场页签、来源、搜索、列表与分页界面。
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub.public_market;
    let content = match &state.sources {
        LoadState::Idle | LoadState::Loading { .. } => loading_view(),
        LoadState::Error(error) => error_view(i18n, &error.render(i18n)),
        LoadState::NeedsLogin => error_view(
            i18n,
            crate::app::i18n::I18n::new(app.desktop_locale).__("公开市场不需要登录，请重试"),
        ),
        LoadState::Loaded(_) => match state.tab {
            PublicSkillTab::Recommended => recommended_content(i18n, state),
            PublicSkillTab::SkillHub => market_content(i18n, state),
            PublicSkillTab::Suites => suites_content(i18n, state),
        },
    };
    let body = column![search_header(app), content].spacing(18).width(Length::Fill);
    scrollable(body)
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

/// 使用完整来源坐标构造打开技能详情的消息。
pub(super) fn skill_detail_message(locator: SkillMarketSkillLocatorDto) -> Message {
    public_message(PublicSkillMessage::OpenSkillDetail(locator))
}
