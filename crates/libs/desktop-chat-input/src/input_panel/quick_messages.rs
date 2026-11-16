//! 聊天输入编辑器上方的分组快捷消息条。

use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::AboveOverlay;
#[cfg(not(target_arch = "wasm32"))]
use desktop_skill_hub::skill_hub::{
    InstalledSkill, LoadState, ShijuPackageRefreshState, ShijuPackageStatus, version_is_newer,
};

use super::quick_message::Message;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text};
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::{Tooltip, tooltip::Position as TooltipPosition};
use iced::{Element, Length, Theme};
use config_ui::{QuickMessageConfig, QuickMessageGroup};

use super::quick_styles as styles;

const QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS: usize = 8;
const QUICK_MESSAGE_MENU_MAX_HEIGHT: f32 = 280.0;
const QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH: f32 = 4.0;
const QUICK_MESSAGE_MENU_SCROLLBAR_GAP: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputStripControl {
    Workspace,
    #[cfg(not(target_arch = "wasm32"))]
    Shiju,
    GeneralActions,
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

fn dropdown_icon<'a>() -> Element<'a, Message> {
    container(super::icons::icon_svg(Icon::ChevronDown, 10.0))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
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

fn menu_view<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
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

    let scrollbar = Scrollbar::new()
        .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH);
    let scrollbar = if menu_requires_scrolling(indices.len()) {
        scrollbar.spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP)
    } else {
        scrollbar
    };
    let items = scrollable(items)
        .height(menu_height(indices.len()))
        .direction(Direction::Vertical(scrollbar));

    container(items)
        .padding(6)
        .width(Length::Fixed(300.0))
        .style(styles::quick_message_menu_style)
        .into()
}

fn group_menu<'a>(
    app: QuickInput<'a>,
    group: QuickMessageGroup,
    indices: &[usize],
) -> Element<'a, Message> {
    let active = app.quick_message_group == Some(group);
    let next_group = toggled_group(app.quick_message_group, group);
    let trigger = button(
        row![
            text(group.label()).size(12),
            text(format!("({})", indices.len())).size(11).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }
            }),
            dropdown_icon(),
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
    )
    .on_press(group_state_message(next_group))
    .padding([6, 10])
    .style(move |theme: &Theme, status| {
        styles::quick_message_group_button_style(theme, status, active)
    });

    AboveOverlay::new(trigger, menu_view(app, indices))
        .show(active)
        .gap(2.0)
        .on_close(group_state_message(None))
        .into()
}

pub fn recommended_indices(app: QuickInput<'_>) -> Vec<usize> {
    app.recommended_prompts.unwrap_or_default().iter().enumerate()
        .filter_map(|(index, prompt)| (!prompt.trim().is_empty()).then_some(index)).collect()
}

fn recommendations_menu_view<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
    let entry = app.recommended_prompts;
    let mut items = column![].spacing(3).width(Length::Fill);
    if let Some(entry) = entry {
        for &index in indices {
            if let Some(prompt) = entry.get(index) {
                items = items.push(
                    button(
                        text(prompt)
                            .size(12)
                            .width(Length::Fill)
                            .wrapping(iced::widget::text::Wrapping::Word),
                    )
                    .on_press(Message::AgentRecommendedPromptSelected(
                        index,
                    ))
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(|theme, status| {
                        styles::quick_message_button_style(theme, status, false)
                    }),
                );
            }
        }
    }
    let scrollbar = Scrollbar::new()
        .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
        .spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP);
    // Wrapped recommendations can exceed the height limit even with fewer than eight rows.
    let content =
        scrollable(container(items).padding(iced::Padding { right: 10.0, ..Default::default() }))
            .direction(Direction::Vertical(scrollbar));
    container(content)
        .max_height(QUICK_MESSAGE_MENU_MAX_HEIGHT)
        .padding(6)
        .width(Length::Fixed(300.0))
        .style(styles::quick_message_menu_style)
        .into()
}

pub fn recommendations_menu<'a>(app: QuickInput<'a>, indices: &[usize]) -> Element<'a, Message> {
    let active = app.recommended_prompt;
    let trigger = button(
        row![
            text("推荐会话").size(12),
            text(format!("({})", indices.len())).size(11),
            dropdown_icon()
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::RecommendedPromptMenuChanged(!active))
    .padding([6, 10])
    .style(move |theme, status| styles::quick_message_group_button_style(theme, status, active));
    AboveOverlay::new(trigger, recommendations_menu_view(app, indices))
        .show(active)
        .gap(2.0)
        .on_close(Message::RecommendedPromptMenuChanged(false))
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn shiju_trigger_label(state: &LoadState<Vec<InstalledSkill>>) -> String {
    match state {
        LoadState::Loaded(skills) => format!("识矩（{}）", skills.len()),
        LoadState::Idle
        | LoadState::NeedsLogin
        | LoadState::Loading { .. }
        | LoadState::Error(_) => "识矩（…）".to_string(),
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
fn shiju_menu_status_text(status: ShijuMenuStatus<'_>) -> String {
    match status {
        ShijuMenuStatus::Loading => "正在加载识矩技能…".to_string(),
        ShijuMenuStatus::NeedsLogin => "请先登录识矩账户".to_string(),
        ShijuMenuStatus::Empty => "暂无已安装识矩技能".to_string(),
        ShijuMenuStatus::Error(error) => format!("加载失败：{error}"),
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
                    _ => app.i18n
                        .__(&shiju_menu_status_text(status))
                        .to_owned(),
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
    let mut trigger_content = row![text(shiju_trigger_label(app.installed)).size(12)]
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
    let trigger_button = button(trigger_content.push(dropdown_icon()))
        .on_press(Message::ToggleShijuSkillMenu)
        .padding([6, 10])
        .style(move |theme: &Theme, status| {
            styles::quick_message_group_button_style(theme, status, active)
        });
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
        .gap(2.0)
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
    .style(|theme: &Theme, status| styles::quick_message_group_button_style(theme, status, false))
    .into()
}

fn workspace_menu_view(app: QuickInput<'_>) -> Element<'_, Message> {
    let mut items = column![].spacing(3).width(Length::Fill);
    if app.recent_projects.is_empty() {
        items = items.push(
            container(
                text(app.i18n.__("暂无空间"))
                    .size(12)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.58)),
                    }),
            )
            .padding([9, 10]),
        );
    }

    for path in app.recent_projects {
        let selected = app.project_path == Some(path.as_str());
        let label = workspace_label(app, path);
        let mut content = row![text(label).size(12).width(Length::Fill)]
            .spacing(8)
            .align_y(iced::Alignment::Center);
        if selected {
            content = content.push(super::icons::icon_svg(Icon::Check, 13.0));
        }
        items = items.push(
            button(content)
                .on_press(workspace_select_message(path.clone()))
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |theme: &Theme, status| {
                    styles::quick_message_group_button_style(theme, status, selected)
                }),
        );
    }

    let items = scrollable(items).height(menu_height(app.recent_projects.len())).direction(
        Direction::Vertical(
            Scrollbar::new()
                .width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
                .scroller_width(QUICK_MESSAGE_MENU_SCROLLBAR_WIDTH)
                .spacing(QUICK_MESSAGE_MENU_SCROLLBAR_GAP),
        ),
    );
    container(
        column![
            items,
            workspace_menu_divider(),
            workspace_add_button(app.i18n)
        ]
        .spacing(3)
        .width(Length::Fill),
    )
    .padding(6)
    .width(Length::Fixed(300.0))
    .style(styles::quick_message_menu_style)
    .into()
}

fn workspace_menu(app: QuickInput<'_>) -> Element<'_, Message> {
    let active = app.work_space_picker_open;
    let label =
        app.project_path.map(|path| workspace_label(app, path)).unwrap_or_else(|| {
            app.i18n.__("选择空间").to_string()
        });
    let trigger =
        button(
            row![
                super::icons::icon_svg(Icon::FolderOpen, 13.0),
                text(label).size(12),
                dropdown_icon(),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::WorkSpacePickerToggled)
        .padding([6, 10])
        .style(move |theme: &Theme, status| {
            styles::quick_message_group_button_style(theme, status, active)
        });

    AboveOverlay::new(trigger, workspace_menu_view(app))
        .show(active)
        .gap(2.0)
        .on_close(Message::WorkSpacePickerClosed)
        .into()
}

fn input_strip_control_order(app: QuickInput<'_>) -> Vec<InputStripControl> {
    let show_workspace = app.show_workspace;
    let mut controls = Vec::with_capacity(4);
    if show_workspace {
        controls.push(InputStripControl::Workspace);
    }
    #[cfg(not(target_arch = "wasm32"))]
    controls.push(InputStripControl::Shiju);
    if !app.quick_messages.is_empty() {
        controls.push(InputStripControl::GeneralActions);
        if app.project_is_git_repo {
            controls.push(InputStripControl::GitActions);
        }
    }
    controls
}

/// 按平台能力渲染工作区、原生识矩技能、通用和 Git 快捷控件。
pub fn view(app: QuickInput<'_>) -> Option<Element<'_, Message>> {
    let (general, git) = grouped_indices(app.quick_messages);
    let mut controls = row![].spacing(6).align_y(iced::Alignment::Center);
    for control in input_strip_control_order(app) {
        controls = match control {
            InputStripControl::Workspace => controls.push(workspace_menu(app)),
            #[cfg(not(target_arch = "wasm32"))]
            InputStripControl::Shiju => controls.push(shiju_menu(app)),
            InputStripControl::GeneralActions => {
                controls.push(group_menu(app, QuickMessageGroup::General, &general))
            }
            InputStripControl::GitActions => {
                controls.push(group_menu(app, QuickMessageGroup::Git, &git))
            }
        };
    }
    if app.recommended_prompts.is_some() {
        let indices = recommended_indices(app);
        if !indices.is_empty() {
            controls = controls.push(recommendations_menu(app, &indices));
        }
    }
    Some(
        container(controls.wrap())
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
    )
}


#[derive(Clone, Copy)]
pub struct QuickInput<'a> {
    pub i18n: desktop_i18n::I18n,
    pub quick_messages: &'a [QuickMessageConfig],
    pub quick_message_group: Option<QuickMessageGroup>,
    pub recommended_prompt: bool,
    pub recommended_prompts: Option<&'a [String]>,
    pub show_workspace: bool,
    pub project_is_git_repo: bool,
    pub recent_projects: &'a [String],
    pub recent_projects_edits: &'a [String],
    pub project_path: Option<&'a str>,
    pub work_space_picker_open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub shiju_skill: bool,
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
