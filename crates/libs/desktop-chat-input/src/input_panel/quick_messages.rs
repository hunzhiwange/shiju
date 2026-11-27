//! 聊天输入编辑器上方的分组快捷消息条。

use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::{AboveOverlay, BelowOverlay};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_session::message::chat::input::popovers::ComposerScene;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_divider, settings_muted_text_style,
    settings_text_input_style,
};
#[cfg(not(target_arch = "wasm32"))]
use desktop_skill_hub::skill_hub::{
    InstalledSkill, LoadState, ShijuPackageRefreshState, ShijuPackageStatus, version_is_newer,
};

use super::quick_message::Message;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, stack, text, text_input};
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::{Tooltip, tooltip::Position as TooltipPosition};
use iced::{Element, Length, Padding, Theme};
use config_ui::{QuickMessageConfig, QuickMessageGroup};

use super::quick_styles as styles;
use super::surface_styles::BOTTOM_BAR_ICON_BUTTON_SIZE;

pub(super) const DROPDOWN_ICON_BOX_SIZE: f32 = 14.0;
const QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS: usize = 8;
const QUICK_MESSAGE_MENU_MAX_HEIGHT: f32 = 280.0;
const QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH: f32 = 4.0;
const QUICK_MESSAGE_MENU_SCROLLBAR_GAP: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputStripControl {
    #[cfg(not(target_arch = "wasm32"))]
    Shiju,
    GeneralActions,
    #[cfg(not(target_arch = "wasm32"))]
    MediaShortcuts(bool),
    Categories(ComposerScene, Option<usize>),
    GitActions,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShijuMenuStatus<'a> {
    Loading,
    NeedsLogin,
    Empty,
    Error(&'a str),
    Loaded,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShijuMenuRow<'a> {
    skill_id: u64,
    name: &'a str,
    version: String,
    tooltip: String,
    selected: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShijuRefreshAction {
    label: &'static str,
    enabled: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShijuVersionHint {
    local_version: String,
    server_version: String,
    detail: String,
    upgrade_enabled: bool,
}

fn grouped_indices(items: &[QuickMessageConfig]) -> (Vec<usize>, Vec<usize>) {
    let mut general = Vec::new();
    let mut git = Vec::new();
    for (index, item) in items.iter().enumerate() {
        match item.group {
            QuickMessageGroup::General => general.push(index),
            QuickMessageGroup::Git => git.push(index),
        }
    }
    (general, git)
}

fn select_message(index: usize) -> Message {
    Message::QuickMessageSelected(index)
}

fn group_state_message(group: Option<QuickMessageGroup>) -> Message {
    Message::QuickMessageGroupChanged(group)
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_select_message(skill_id: u64) -> Message {
    Message::SelectShijuSkill(skill_id)
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_refresh_message() -> Message {
    Message::RefreshInstalledSkills
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_upgrade_message() -> Message {
    Message::Batch(vec![
        Message::ToggleShijuSkillMenu,
        Message::OpenInstalledShijuSkills,
    ])
}

fn toggled_group(
    open_group: Option<QuickMessageGroup>,
    clicked_group: QuickMessageGroup,
) -> Option<QuickMessageGroup> {
    (open_group != Some(clicked_group)).then_some(clicked_group)
}

pub(super) fn shortcut_trigger<'a>(
    content: impl Into<Element<'a, Message>>,
    action: Message,
    active: bool,
) -> iced::widget::Button<'a, Message> {
    button(container(content).center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
        .padding([0, 10])
        .style(move |theme, status| styles::quick_message_group_button_style(theme, status, active))
        .on_press(action)
}

pub(super) fn shortcut_dropdown_trigger<'a>(
    label: impl Into<String>,
    action: Message,
    open: bool,
) -> iced::widget::Button<'a, Message> {
    let label = container(SingleLineLabel::new(label.into(), 12.0).shrink())
        .padding(Padding { right: DROPDOWN_ICON_BOX_SIZE + 5.0, ..Padding::ZERO });
    shortcut_trigger(
        stack![
            label,
            container(dropdown_icon()).align_right(Length::Fill).center_y(Length::Fill),
        ],
        action,
        open,
    )
}

pub(super) fn dropdown_icon<'a>() -> Element<'a, Message> {
    container(super::icons::icon_svg(Icon::ChevronDown, 10.0))
        .center(DROPDOWN_ICON_BOX_SIZE)
        .into()
}

fn menu_requires_scrolling(item_count: usize) -> bool {
    item_count > QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS
}

fn menu_height(item_count: usize) -> Length {
    if menu_requires_scrolling(item_count) {
        Length::Fixed(QUICK_MESSAGE_MENU_MAX_HEIGHT)
    } else {
        Length::Shrink
    }
}

pub(super) fn shortcut_menu_items<'a>(
    items: impl Into<Element<'a, Message>>,
    count: usize,
) -> iced::widget::Scrollable<'a, Message> {
    let scrollbar = Scrollbar::new()
        .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH);
    let scrollbar = if menu_requires_scrolling(count) {
        scrollbar.spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP)
    } else {
        scrollbar
    };
    scrollable(items)
        .height(menu_height(count))
        .direction(Direction::Vertical(scrollbar))
}

pub(super) fn shortcut_menu<'a>(
    trigger: impl Into<Element<'a, Message>>,
    items: impl Into<Element<'a, Message>>,
    close: Message,
) -> Element<'a, Message> {
    let popup = container(items)
        .padding(6)
        .width(Length::Fixed(300.0))
        .style(styles::quick_message_menu_style);
    AboveOverlay::new(trigger, popup)
        .show(true)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(6.0)
        .on_close(close)
        .into()
}

fn menu_items<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
    let mut items = column![].spacing(3).width(Length::Fill);
    #[cfg(not(target_arch = "wasm32"))]
    if app.package_status.local_version.is_none() {
        items = items.push(
            button(
                text(app.i18n.__("安装识矩原生技能"))
                    .size(12),
            )
            .on_press(Message::PrepareNativeShijuSkill)
            .padding([8, 10])
            .width(Length::Fill)
            .style(|theme: &Theme, status| styles::shiju_skill_button_style(theme, status, false)),
        );
    }
    if indices.is_empty() {
        items = items.push(
            container(
                text(app.i18n.__("暂无配置"))
                    .size(12)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.58)),
                    }),
            )
            .padding([9, 10]),
        );
    }

    for &index in indices {
        let item = &app.quick_messages[index];
        let action_badge = item.kind.action_badge();
        let is_action = action_badge.is_some();
        let action_label: Element<'_, Message> = if let Some(label) = action_badge {
            container(text(label).size(10))
                .padding([2, 6])
                .style(styles::quick_message_action_badge_style)
                .into()
        } else {
            iced::widget::Space::new().width(Length::Shrink).into()
        };
        items = items.push(
            button(
                row![text(&item.text).size(12).width(Length::Fill), action_label,]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            )
            .on_press(select_message(index))
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme, status| {
                styles::quick_message_button_style(theme, status, is_action)
            }),
        );
    }

    let count = indices.len().max(1);
    #[cfg(not(target_arch = "wasm32"))]
    let count = count + usize::from(app.package_status.local_version.is_none());
    shortcut_menu_items(items, count).into()
}

fn group_menu<'a>(
    app: QuickInput<'a>,
    group: QuickMessageGroup,
    indices: &[usize],
) -> Element<'a, Message> {
    let active = app.quick_message_group == Some(group);
    let next_group = toggled_group(app.quick_message_group, group);
    let trigger = shortcut_trigger(
        row![
            text(app.i18n.__(group.label())).size(12),
            text(format!("({})", indices.len())).size(11).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }
            }),
            dropdown_icon(),
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
        group_state_message(next_group),
        active,
    );

    if active {
        shortcut_menu(trigger, menu_items(app, indices), group_state_message(None))
    } else {
        trigger.into()
    }
}

pub fn recommended_indices(app: QuickInput<'_>) -> Vec<usize> {
    app.recommended_prompts.unwrap_or_default().iter().enumerate()
        .filter_map(|(index, prompt)| (!prompt.trim().is_empty()).then_some(index)).collect()
}

fn recommendation_rows<'a>(
    app: QuickInput<'a>,
    indices: &[usize],
) -> iced::widget::Column<'a, Message> {
    let prompts = app.recommended_prompts.unwrap_or_default();
    column![].spacing(3).width(Length::Fill).extend(
        indices.iter().filter_map(|&index| {
            let prompt = prompts.get(index)?;
            Some(recommendation_row(prompt, Message::AgentRecommendedPromptSelected(index)).into())
        }),
    )
}

fn recommendation_row(label: &str, action: Message) -> iced::widget::Button<'_, Message> {
    button(
        row![
            SingleLineLabel::new(label.to_owned(), 12.0),
            super::icons::icon_svg(Icon::ArrowRight, 12.0),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .on_press(action)
    .padding([8, 10])
    .width(Length::Fill)
    .style(|theme, status| styles::quick_message_button_style(theme, status, false))
}

fn recommendation_popup(items: iced::widget::Column<'_, Message>) -> Element<'_, Message> {
    let scrollbar = Scrollbar::new()
        .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP);
    let content =
        scrollable(container(items).padding(iced::Padding { right: 10.0, ..Default::default() }))
            .direction(Direction::Vertical(scrollbar));
    container(content)
        .max_height(QUICK_MESSAGE_MENU_MAX_HEIGHT)
        .padding(6)
        .width(Length::Fixed(560.0))
        .style(styles::quick_message_menu_style)
        .into()
}

fn recommendations_menu_view<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
    recommendation_popup(recommendation_rows(app, indices))
}

fn recommendation_trigger<'a>(
    app: QuickInput<'a>,
    count: usize,
) -> iced::widget::Button<'a, Message> {
    let active = app.recommended_prompt;
    let chevron = if active { Icon::ChevronUp } else { Icon::ChevronDown };
    let label = row![
        SingleLineLabel::new(app.i18n.__("推荐会话").to_owned(), 12.0).shrink(),
        text(format!("({count})")).size(11).style(|theme: &Theme| {
            text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }
        }),
        container(super::icons::icon_svg(chevron, 10.0)).center(DROPDOWN_ICON_BOX_SIZE),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);
    button(container(label).center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
        .padding([0, 9])
        .style(move |theme, status| super::styles::selector_pill_button_style(theme, status, active))
        .on_press(Message::RecommendedPromptMenuChanged(!active))
}

pub fn recommendations_menu<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
    AboveOverlay::new(
        recommendation_trigger(app, indices.len()),
        recommendations_menu_view(app, indices),
    )
    .show(app.recommended_prompt)
    .block_background_events(true)
    .flip_if_clipped()
    .gap(6.0)
    .on_close(Message::RecommendedPromptMenuChanged(false))
    .into()
}

pub fn starter_recommendations(
    app: QuickInput<'_>,
    scene: ComposerScene,
    category: Option<usize>,
) -> Element<'_, Message> {
    let indices = if category.is_none() { recommended_indices(app) } else { Vec::new() };
    let presets = super::starter::prompts(scene, category);
    let count = indices.len() + presets.len();
    let popup = if app.recommended_prompt {
        let presets = presets.iter().enumerate().map(|(index, prompt)| {
            recommendation_row(
                app.i18n.__(prompt.title),
                Message::ComposerPromptSelected(scene, category, index),
            )
            .into()
        });
        let items = column![].spacing(3).width(Length::Fill).extend(presets);
        let items = if indices.is_empty() {
            items
        } else {
            items.push(settings_divider()).push(recommendation_rows(app, &indices))
        };
        recommendation_popup(items)
    } else {
        Space::new().into()
    };
    BelowOverlay::new(recommendation_trigger(app, count), popup)
        .show(app.recommended_prompt)
        .gap(6.0)
        .dismiss_on_escape()
        .on_close(Message::RecommendedPromptMenuChanged(false))
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_trigger_label(i18n: desktop_i18n::I18n, state: &LoadState<Vec<InstalledSkill>>) -> String {
    match state {
        LoadState::Loaded(skills) => i18n.format("识矩（%u）", &[skills.len().into()]).expect("valid skill count"),
        LoadState::Idle
        | LoadState::NeedsLogin
        | LoadState::Loading { .. }
        | LoadState::Error(_) => i18n.__("识矩（…）").to_string(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_update_available(status: &ShijuPackageStatus) -> bool {
    match (&status.local_version, &status.server_version) {
        (Some(local), Some(server)) => version_is_newer(local, server),
        _ => false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_function_upgrade_count(installed: &LoadState<Vec<InstalledSkill>>) -> usize {
    match installed {
        LoadState::Loaded(skills) => skills.iter().filter(|skill| skill.upgrade_available).count(),
        _ => 0,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_upgrade_count(app: QuickInput<'_>) -> usize {
    usize::from(shiju_update_available(app.package_status))
        + shiju_function_upgrade_count(app.installed)
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_update_badge(label: String) -> Element<'static, Message> {
    let dot = container(Space::new().width(6).height(6)).style(|theme: &Theme| container::Style {
        background: Some(theme.extended_palette().danger.strong.color.into()),
        border: iced::Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    });
    row![
        dot,
        text(label).size(11).wrapping(iced::widget::text::Wrapping::None).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.strong.color) }
        }),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_version_hint(
    i18n: desktop_i18n::I18n,
    status: &ShijuPackageStatus,
) -> ShijuVersionHint {
    let local_version =
        status.local_version.clone().unwrap_or_else(|| i18n.__("未安装").to_string());
    let server_version = match (&status.server_version, status.refresh) {
        (Some(version), ShijuPackageRefreshState::Failed) => {
            desktop_i18n::__!(i18n, "%s（上次检测）", version.as_str()).expect("valid version template")
        }
        (Some(version), _) => version.clone(),
        (None, ShijuPackageRefreshState::Refreshing { .. }) => i18n.__("检测中").to_string(),
        (None, ShijuPackageRefreshState::Failed) => i18n.__("未知（刷新失败）").to_string(),
        (None, ShijuPackageRefreshState::Idle | ShijuPackageRefreshState::Fresh) => {
            i18n.__("未知").to_string()
        }
    };
    let upgrade_enabled = status.local_version.is_none()
        || matches!(
            (&status.local_version, &status.server_version),
            (Some(local), Some(server)) if local != server
        );
    let detail = match status.refresh {
        ShijuPackageRefreshState::Refreshing { .. } if status.server_version.is_none() => {
            "正在检测服务端版本…"
        }
        ShijuPackageRefreshState::Refreshing { .. } => "正在刷新服务端版本…",
        ShijuPackageRefreshState::Failed if upgrade_enabled => "刷新失败，建议按上次结果前往处理",
        ShijuPackageRefreshState::Failed => "刷新失败，服务端版本为上次检测结果",
        ShijuPackageRefreshState::Idle | ShijuPackageRefreshState::Fresh
            if status.local_version.is_none() =>
        {
            "本地未安装，建议前往安装"
        }
        ShijuPackageRefreshState::Idle | ShijuPackageRefreshState::Fresh
            if status.server_version.is_none() =>
        {
            "暂未获取服务端版本"
        }
        ShijuPackageRefreshState::Idle | ShijuPackageRefreshState::Fresh if !upgrade_enabled => {
            "当前版本已是最新"
        }
        ShijuPackageRefreshState::Idle | ShijuPackageRefreshState::Fresh => {
            "版本不一致，建议前往处理"
        }
    };

    ShijuVersionHint {
        local_version,
        server_version,
        detail: i18n.__(detail).to_string(),
        upgrade_enabled,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_trigger_tooltip(
    i18n: desktop_i18n::I18n,
    status: &ShijuPackageStatus,
) -> Element<'static, Message> {
    let hint = shiju_version_hint(i18n, status);
    container(
        column![
            text(i18n.__("识矩技能版本")).size(12),
            row![text(i18n.__("本地版本：")).size(12), text(hint.local_version).size(12),],
            row![text(i18n.__("服务端版本：")).size(12), text(hint.server_version).size(12),],
            text(hint.detail).size(12),
        ]
        .spacing(5),
    )
    .padding([9, 11])
    .style(desktop_ui_controls::tooltip::tooltip_dark_style)
    .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_menu_status(state: &LoadState<Vec<InstalledSkill>>) -> ShijuMenuStatus<'_> {
    match state {
        LoadState::Idle | LoadState::Loading { .. } => ShijuMenuStatus::Loading,
        LoadState::NeedsLogin => ShijuMenuStatus::NeedsLogin,
        LoadState::Loaded(skills) if skills.is_empty() => ShijuMenuStatus::Empty,
        LoadState::Loaded(_) => ShijuMenuStatus::Loaded,
        LoadState::Error(error) => ShijuMenuStatus::Error(error),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_menu_status_text(i18n: desktop_i18n::I18n, status: ShijuMenuStatus<'_>) -> String {
    match status {
        ShijuMenuStatus::Loading => i18n.__("正在加载识矩技能…").to_string(),
        ShijuMenuStatus::NeedsLogin => i18n.__("请先登录识矩账户").to_string(),
        ShijuMenuStatus::Empty => i18n.__("暂无已安装识矩技能").to_string(),
        ShijuMenuStatus::Error(error) => i18n.format("加载失败：%s", &[error.into()]).expect("valid skill error"),
        ShijuMenuStatus::Loaded => String::new(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_refresh_action(state: &LoadState<Vec<InstalledSkill>>) -> ShijuRefreshAction {
    if matches!(state, LoadState::Loading { .. }) {
        ShijuRefreshAction { label: "刷新中…", enabled: false }
    } else {
        ShijuRefreshAction { label: "刷新功能", enabled: true }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_menu_rows<'a>(app: QuickInput<'a>) -> Vec<ShijuMenuRow<'a>> {
    let selector = app.tool_selector;
    let LoadState::Loaded(skills) = app.installed else {
        return Vec::new();
    };

    skills
        .iter()
        .map(|skill| ShijuMenuRow {
            skill_id: skill.skill_id,
            name: &skill.name,
            version: if skill.upgrade_available {
                format!("{} → {}", skill.installed_version, skill.current_version)
            } else {
                skill.installed_version.clone()
            },
            tooltip: desktop_i18n::__!(
                app.i18n,
                "%s\n标识：%s\n版本：%s\n\n%s",
                skill.name.as_str(),
                skill.slug.as_str(),
                skill.installed_version.as_str(),
                skill.description.as_str()
            )
            .expect("valid skill tooltip template"),
            selected: selector
                .is_some_and(|selector| selector.is_shiju_skill_selected(skill.skill_id)),
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_tooltip(row: &ShijuMenuRow<'_>) -> Element<'static, Message> {
    container(
        text(row.tooltip.clone())
            .size(12)
            .width(Length::Fixed(276.0))
            .wrapping(iced::widget::text::Wrapping::Word),
    )
    .padding([9, 11])
    .style(desktop_ui_controls::tooltip::tooltip_dark_style)
    .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_menu_view(app: QuickInput<'_>) -> Element<'_, Message> {
    let i18n = app.i18n;
    let refresh_action = shiju_refresh_action(app.installed);
    let refresh_button = button(
        row![
            super::icons::icon_svg(Icon::ArrowRepeat, 13.0),
            text(i18n.__(refresh_action.label)).size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .on_press_maybe(refresh_action.enabled.then(shiju_refresh_message))
    .padding([8, 10])
    .width(Length::Fill)
    .style(|theme: &Theme, status| styles::shiju_skill_button_style(theme, status, false));
    let main_upgrade = shiju_update_available(app.package_status);
    let function_upgrades = shiju_function_upgrade_count(app.installed);
    let upgrade_enabled = function_upgrades > 0
        || shiju_version_hint(i18n, app.package_status).upgrade_enabled;
    let mut upgrade_content = row![
        text(i18n.__("前往识矩技能升级")).size(12).wrapping(iced::widget::text::Wrapping::None)
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);
    if main_upgrade {
        upgrade_content = upgrade_content.push(shiju_update_badge(i18n.__("主技能").to_string()));
    }
    if function_upgrades > 0 {
        upgrade_content = upgrade_content.push(shiju_update_badge(
            i18n.format("%s项功能", &[function_upgrades.to_string().into()])
                .expect("valid function upgrade count"),
        ));
    }
    let upgrade_button = button(upgrade_content)
        .on_press(shiju_upgrade_message())
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |theme: &Theme, status| {
            styles::shiju_upgrade_button_style(theme, status, upgrade_enabled)
        });
    let status = shiju_menu_status(app.installed);
    let rows = shiju_menu_rows(app);
    let item_count = rows.len();
    let mut items = column![].spacing(3).width(Length::Fill);
    if app.package_status.local_version.is_none() {
        items = items.push(
            button(
                text(app.i18n.__("安装识矩原生技能"))
                    .size(12),
            )
            .on_press(Message::PrepareNativeShijuSkill)
            .padding([8, 10])
            .width(Length::Fill)
            .style(|theme: &Theme, status| styles::shiju_skill_button_style(theme, status, false)),
        );
    }

    if status != ShijuMenuStatus::Loaded {
        items = items.push(
            container(
                text(match status {
                    ShijuMenuStatus::Error(error) => desktop_i18n::__!(
                        app.i18n,
                        "加载失败：%s",
                        error
                    )
                    .expect("valid skill error template"),
                    _ => shiju_menu_status_text(app.i18n, status),
                })
                .size(12)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.66)),
                }),
            )
            .padding([9, 10]),
        );
    }

    for row in rows {
        let mut content =
            row![text(row.name).size(12).width(Length::Fill), text(row.version.clone()).size(12),]
                .spacing(8)
                .align_y(iced::Alignment::Center);
        if row.selected {
            content = content.push(super::icons::icon_svg(Icon::Check, 13.0));
        }
        let selected = row.selected;
        let skill_id = row.skill_id;
        let skill_button = button(content)
            .on_press(shiju_select_message(skill_id))
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme, status| {
                styles::shiju_skill_button_style(theme, status, selected)
            });
        items = items.push(
            Tooltip::new(skill_button, shiju_tooltip(&row), TooltipPosition::Right)
                .gap(6.0)
                .snap_within_viewport(true),
        );
    }

    let scrollbar = Scrollbar::new()
        .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH);
    let scrollbar = if menu_requires_scrolling(item_count) {
        scrollbar.spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP)
    } else {
        scrollbar
    };
    let items =
        scrollable(items).height(menu_height(item_count)).direction(Direction::Vertical(scrollbar));

    container(column![upgrade_button, refresh_button, items].spacing(3).width(Length::Fill))
        .padding(6)
        .width(Length::Fixed(300.0))
        .style(styles::quick_message_menu_style)
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_menu(app: QuickInput<'_>) -> Element<'_, Message> {
    let active = app.shiju_skill;
    let mut trigger_content = row![text(shiju_trigger_label(app.i18n, app.installed)).size(12)]
        .spacing(5)
        .align_y(iced::Alignment::Center);
    let upgrade_count = shiju_upgrade_count(app);
    if upgrade_count > 0 {
        let i18n = app.i18n;
        trigger_content = trigger_content.push(shiju_update_badge(
            i18n.format("%s 项待升级", &[upgrade_count.to_string().into()])
                .expect("valid upgrade count"),
        ));
    }
    let trigger_button = shortcut_trigger(
        trigger_content.push(dropdown_icon()),
        Message::ToggleShijuSkillMenu,
        active,
    );
    let trigger = Tooltip::new(
        trigger_button,
        shiju_trigger_tooltip(
            app.i18n,
            app.package_status,
        ),
        TooltipPosition::Bottom,
    )
    .gap(6.0)
    .snap_within_viewport(true);

    AboveOverlay::new(trigger, shiju_menu_view(app))
        .show(active)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(6.0)
        .on_close(Message::ToggleShijuSkillMenu)
        .into()
}

fn workspace_label(app: QuickInput<'_>, path: &str) -> String {
    app.recent_projects
        .iter()
        .position(|candidate| candidate == path)
        .and_then(|index| app.recent_projects_edits.get(index))
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| path.to_string())
}

fn workspace_select_message(path: String) -> Message {
    Message::Batch(vec![
        Message::WorkSpacePickerClosed,
        Message::SwitchWorkSpacePressed(path),
    ])
}

fn workspace_add_message() -> Message {
    Message::Batch(vec![
        Message::WorkSpacePickerClosed,
        Message::OpenFolderPressed,
    ])
}

fn workspace_menu_divider() -> Element<'static, Message> {
    container(Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.52),
            )),
            ..Default::default()
        })
        .into()
}

fn workspace_add_button(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    button(
        row![
            super::icons::icon_svg(Icon::Plus, 13.0),
            text(i18n.__("新增项目")).size(12).width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(workspace_add_message())
    .padding([8, 10])
    .width(Length::Fill)
    .style(|theme: &Theme, status| styles::shiju_skill_button_style(theme, status, false))
    .into()
}

pub fn workspace_search_id() -> iced::widget::Id {
    iced::widget::Id::new("composer-workspace-search")
}

pub fn workspace_results_id() -> iced::widget::Id {
    iced::widget::Id::new("composer-workspace-results")
}

fn workspace_menu_view(app: QuickInput<'_>) -> Element<'_, Message> {
    let query = app.workspace_query.trim().to_lowercase();
    let matches: Vec<_> = app
        .recent_projects
        .iter()
        .map(|path| (path, workspace_label(app, path)))
        .filter(|(path, label)| {
            query.is_empty()
                || label.to_lowercase().contains(&query)
                || path.to_lowercase().contains(&query)
        })
        .collect();
    let search = text_input(app.i18n.__("搜索项目"), app.workspace_query)
        .id(workspace_search_id())
        .on_input(Message::WorkSpacePickerQueryChanged)
        .on_submit_maybe((matches.len() == 1).then(|| workspace_select_message(matches[0].0.clone())))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let mut items = column![].spacing(3).width(Length::Fill);

    if matches.is_empty() {
        let label = if query.is_empty() { "暂无空间" } else { "没有匹配的项目" };
        items = items.push(
            container(text(app.i18n.__(label)).size(12).style(settings_muted_text_style))
                .padding([9, 10]),
        );
    }
    for (path, label) in matches {
        let selected = app.project_path == Some(path.as_str());
        let mut content = row![
            super::icons::icon_svg(Icon::FolderOpen, 13.0),
            text(label.clone())
                .size(13)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        if selected {
            content = content.push(super::icons::icon_svg(Icon::Check, 13.0));
        }
        let option = button(content)
            .on_press(workspace_select_message(path.clone()))
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme, status| styles::shiju_skill_button_style(theme, status, selected));
        items = items.push(desktop_ui_controls::tooltip::project_tooltip(
            option,
            label,
            path,
            iced::widget::tooltip::Position::Right,
        ));
    }

    let items = scrollable(container(items).padding([0, 7]))
        .id(workspace_results_id())
        .height(Length::Fill)
        .direction(
        Direction::Vertical(
            Scrollbar::new()
                .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
                .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
                .spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP),
        ),
    );
    let list_height = (app.recent_projects.len().max(1) as f32 * 36.0)
        .min(QUICK_MESSAGE_MENU_MAX_HEIGHT);
    container(
        column![
            search,
            items,
            workspace_menu_divider(),
            workspace_add_button(app.i18n)
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding(8)
    .height(Length::Fixed(97.0 + list_height))
    .width(Length::Fixed(300.0))
    .style(styles::quick_message_menu_style)
    .into()
}

pub fn workspace_menu(app: QuickInput<'_>) -> Element<'_, Message> {
    let active = app.work_space_picker_open;
    let label =
        app.project_path.map(|path| workspace_label(app, path)).unwrap_or_else(|| {
            app.i18n.__("选择空间").to_string()
        });
    let short_label = desktop_chat_tools::text_layout::truncate_chars(&label, 25);
    let workspace_icon = super::icons::icon_svg(Icon::FolderOpen, 14.0)
        .style(move |theme: &Theme, _| iced::widget::svg::Style {
            color: Some(super::styles::selector_text_color(theme, active).scale_alpha(0.88)),
        });
    let chevron =
        super::icons::icon_svg(if active { Icon::ChevronUp } else { Icon::ChevronDown }, 10.0)
            .style(move |theme: &Theme, _| iced::widget::svg::Style {
                color: Some(super::styles::selector_chevron_color(theme, active)),
            });
    let trigger = button(
        container(
            row![workspace_icon, text(short_label).size(12), chevron]
                .spacing(7)
                .align_y(iced::Alignment::Center),
        )
        .center_y(BOTTOM_BAR_ICON_BUTTON_SIZE),
    )
    .on_press(Message::WorkSpacePickerToggled)
    .padding([0, 9])
    .style(move |theme: &Theme, status| {
        super::styles::selector_pill_button_style(theme, status, active)
    });
    let trigger = match app.project_path.filter(|_| !active) {
        Some(path) => desktop_ui_controls::tooltip::project_tooltip(
            trigger,
            label,
            path,
            iced::widget::tooltip::Position::Top,
        ),
        None => trigger.into(),
    };

    if active {
        AboveOverlay::new(trigger, workspace_menu_view(app))
            .show(true)
            .block_background_events(true)
            .flip_if_clipped()
            .gap(6.0)
            .on_close(Message::WorkSpacePickerClosed)
            .into()
    } else {
        trigger.into()
    }
}

fn input_strip_control_order(
    app: QuickInput<'_>,
    starter: Option<(ComposerScene, Option<usize>)>,
) -> Vec<InputStripControl> {
    let mut controls = Vec::with_capacity(5);
    #[cfg(not(target_arch = "wasm32"))]
    controls.push(InputStripControl::Shiju);
    if !app.quick_messages.is_empty() {
        controls.push(InputStripControl::GeneralActions);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(open) = app.media_shortcuts {
        controls.push(InputStripControl::MediaShortcuts(open));
    }
    if let Some((scene, category)) = starter {
        controls.push(InputStripControl::Categories(scene, category));
    }
    if !app.quick_messages.is_empty() && app.project_is_git_repo {
        controls.push(InputStripControl::GitActions);
    }
    controls
}

/// 渲染快捷控件，把场景分类放在多模态入口之后。
pub fn view(
    app: QuickInput<'_>,
    starter: Option<(ComposerScene, Option<usize>)>,
) -> Element<'_, Message> {
    let (general, git) = grouped_indices(app.quick_messages);
    let mut controls = row![].spacing(6).align_y(iced::Alignment::Center);
    for control in input_strip_control_order(app, starter) {
        controls = match control {
            #[cfg(not(target_arch = "wasm32"))]
            InputStripControl::Shiju => controls.push(shiju_menu(app)),
            InputStripControl::GeneralActions => {
                controls.push(group_menu(app, QuickMessageGroup::General, &general))
            }
            #[cfg(not(target_arch = "wasm32"))]
            InputStripControl::MediaShortcuts(open) => {
                controls.push(super::media_shortcuts::menu(app, open))
            }
            InputStripControl::Categories(scene, category) => {
                controls.push(super::starter::category_selector(
                    app.i18n,
                    scene,
                    category,
                    app.category_menu_open,
                ))
            }
            InputStripControl::GitActions => {
                controls.push(group_menu(app, QuickMessageGroup::Git, &git))
            }
        };
    }
    if let Some(open) = app.employee_commands {
        controls = controls.push(super::employee_commands::menu(app.i18n, open));
    }
    if starter.is_none() {
        let indices = recommended_indices(app);
        if !indices.is_empty() {
            controls = controls.push(recommendations_menu(app, &indices));
        }
    }
    container(controls.wrap().vertical_spacing(5))
        .width(Length::Fill)
        .into()
}


#[derive(Clone, Copy)]
pub struct QuickInput<'a> {
    pub i18n: desktop_i18n::I18n,
    pub quick_messages: &'a [QuickMessageConfig],
    pub quick_message_group: Option<QuickMessageGroup>,
    pub employee_commands: Option<bool>,
    pub category_menu_open: bool,
    pub recommended_prompt: bool,
    pub recommended_prompts: Option<&'a [String]>,
    pub project_is_git_repo: bool,
    pub recent_projects: &'a [String],
    pub recent_projects_edits: &'a [String],
    pub project_path: Option<&'a str>,
    pub work_space_picker_open: bool,
    pub workspace_query: &'a str,
    #[cfg(not(target_arch = "wasm32"))]
    pub shiju_skill: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub media_shortcuts: Option<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub media_models: Option<super::media_shortcuts::MediaModelPicker<'a>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_media_model: Option<&'a str>,
    #[cfg(not(target_arch = "wasm32"))]
    pub installed: &'a LoadState<Vec<InstalledSkill>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub package_status: &'a ShijuPackageStatus,
    #[cfg(not(target_arch = "wasm32"))]
    pub tool_selector: Option<&'a desktop_chat_tools::state::SessionToolSelectorState>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "quick_test_fixtures.rs"]
mod fixtures;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "quick_messages_tests.rs"]
mod quick_messages_tests;
