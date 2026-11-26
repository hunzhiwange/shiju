#[derive(Clone, Copy)]
pub struct ExecutorView<'a> {
 pub task_board_draft: &'a desktop_task_model::TaskDraft,
 pub task_board_bulk_acp_agent: &'a Option<String>,
 pub task_board_executor_popover: bool,
 pub task_board_bulk_executor_popover: bool,
 pub desktop_locale: config_ui::DesktopLocale,
 pub acp_agents: &'a [String],
}
// 任务看板侧栏面板模块，负责任务编辑、执行器选择和日志弹窗界面。
//
// 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, svg};
use iced::{Alignment, Element, Length, Theme};

use desktop_assets::assets::{self, Icon};
use config_agent::{AgentRuntime, is_retired_builtin_agent_key};
use desktop_chat_input::input_panel::icons::{acp_agent_icon, icon_svg};
use desktop_chat_input::input_panel::panel::{agent_option, agent_popover_style, agent_toggle};
use desktop_i18n::I18n;
use desktop_settings_agents::acp::EMPLOYEE_ACP_BACKEND;
use desktop_settings_agents::state::{DelegateAgentSettingsEntry, MAIN_AGENT_KEY};
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::SettingsSelectLabel;
use crate::message::TaskBoardMessage;
use crate::message::TaskBoardMessage as Message;

use super::styles::{popover_style, selector_option_style, selector_trigger_style};

const ACP_SELECTOR_MAX_HEIGHT: f32 = 240.0;
const ACP_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
const ACP_SELECTOR_LIST_GUTTER: f32 = 7.0;
const DEFAULT_TASK_EXECUTOR_LABEL: &str = "未使用 ACP";

#[derive(Clone, Copy)]
enum TaskBoardExecutorSelectorKind {
    Edit,
    Bulk,
}

impl TaskBoardExecutorSelectorKind {
    fn current_executor(&self, app: ExecutorView<'_>) -> Option<String> {
        match self {
            Self::Edit => app.task_board_draft.acp_agent.clone(),
            Self::Bulk => (*app.task_board_bulk_acp_agent).clone(),
        }
    }

    fn toggle_message(&self) -> TaskBoardMessage {
        match self {
            Self::Edit => TaskBoardMessage::ToggleExecutorPopover,
            Self::Bulk => TaskBoardMessage::ToggleBulkExecutorPopover,
        }
    }

    fn close_message(&self) -> TaskBoardMessage {
        match self {
            Self::Edit => TaskBoardMessage::CloseExecutorPopover,
            Self::Bulk => TaskBoardMessage::CloseBulkExecutorPopover,
        }
    }

    fn is_open(&self, app: ExecutorView<'_>) -> bool {
        match self {
            Self::Edit => app.task_board_executor_popover,
            Self::Bulk => app.task_board_bulk_executor_popover,
        }
    }

    fn select_message(&self, executor: Option<String>) -> TaskBoardMessage {
        match self {
            Self::Edit => TaskBoardMessage::UpdateEditingTaskExecutor(executor),
            Self::Bulk => TaskBoardMessage::BulkExecutorSelected(executor),
        }
    }
}

pub(super) fn build_edit_executor_selector(app: ExecutorView<'_>) -> Element<'_, Message> {
    build_selector(app, TaskBoardExecutorSelectorKind::Edit)
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_bulk_executor_selector<'a>(app: ExecutorView<'a>) -> Element<'a, Message> {
    build_selector(app, TaskBoardExecutorSelectorKind::Bulk)
}

fn build_selector<'a>(app: ExecutorView<'a>, kind: TaskBoardExecutorSelectorKind) -> Element<'a, Message> {
    let current_executor = kind.current_executor(app);
    let executor_label_text = current_executor.as_deref().map(str::to_owned).unwrap_or_else(|| {
        desktop_i18n::I18n::new(app.desktop_locale).__(DEFAULT_TASK_EXECUTOR_LABEL).to_string()
    });

    let open = kind.is_open(app);
    let executor_toggle = button(
        row![
            acp_agent_icon(
                current_executor.as_deref().unwrap_or(DEFAULT_TASK_EXECUTOR_LABEL),
                12.0
            ),
            SettingsSelectLabel::new(executor_label_text),
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::ChevronDown))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|theme: &Theme, _| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.65))
                })
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(move |theme, status| selector_trigger_style(theme, status, open))
    .padding([6, 8])
    .width(Length::Fill)
    .height(38)
    .on_press(kind.toggle_message());

    let executor_pop_content = build_executor_popover(current_executor, app, kind);

    BelowOverlay::new(executor_toggle, executor_pop_content)
        .show(open)
        .gap(6.0)
        .on_close(kind.close_message())
        .into()
}

struct DraftAgentChoice<'a> {
    key: &'a str,
    backend: Option<&'a str>,
    label: String,
}

fn draft_agent_choices<'a>(
    app: ExecutorView<'a>,
    entries: &'a [DelegateAgentSettingsEntry],
) -> Vec<DraftAgentChoice<'a>> {
    let i18n = I18n::new(app.desktop_locale);
    let mut choices = vec![DraftAgentChoice {
        key: MAIN_AGENT_KEY,
        backend: None,
        label: i18n.__("主智能体").to_owned(),
    }];
    choices.extend(entries.iter().filter_map(|entry| {
        if entry.key == MAIN_AGENT_KEY || !entry.enabled || is_retired_builtin_agent_key(&entry.key)
        {
            return None;
        }
        let backend = match &entry.runtime {
            AgentRuntime::Native => None,
            AgentRuntime::Acp { backend }
                if entry.can_start_session() && backend != EMPLOYEE_ACP_BACKEND =>
            {
                Some(backend.as_str())
            }
            AgentRuntime::Acp { .. } => return None,
        };
        Some(DraftAgentChoice { key: &entry.key, backend, label: entry.label.clone() })
    }));
    choices.extend(app.acp_agents.iter().filter_map(|backend| {
        // A backend's default profile and its direct entry resolve to the same gateway route.
        let default_profile = format!("acp-{backend}");
        if backend == EMPLOYEE_ACP_BACKEND
            || entries.iter().any(|entry| entry.key == default_profile)
        {
            return None;
        }
        Some(DraftAgentChoice {
            key: MAIN_AGENT_KEY,
            backend: Some(backend),
            label: crate::draft::task_acp_agent_label(Some(backend)),
        })
    }));
    choices
}

pub(super) fn build_draft_agent_selector<'a>(
    app: ExecutorView<'a>,
    entries: &'a [DelegateAgentSettingsEntry],
) -> Element<'a, Message> {
    let open = app.task_board_executor_popover;
    let draft = app.task_board_draft;
    let key = draft.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
    let backend = draft.acp_agent.as_deref();
    let default_profile = backend.map(|backend| format!("acp-{backend}"));
    let choices = draft_agent_choices(app, entries);
    let selected = choices.iter().position(|choice| {
        choice.backend == backend
            && (choice.key == key
                || (key == MAIN_AGENT_KEY && default_profile.as_deref() == Some(choice.key)))
    });
    let label = selected.map(|index| choices[index].label.clone()).unwrap_or_else(|| {
        if key == MAIN_AGENT_KEY {
            crate::draft::task_acp_agent_label(backend)
        } else {
            entries.iter().find(|entry| entry.key == key)
                .map_or_else(|| key.to_owned(), |entry| entry.label.clone())
        }
    });
    let toggle = agent_toggle(
        open || key != MAIN_AGENT_KEY || backend.is_some(),
        label,
        backend,
        None,
        TaskBoardMessage::ToggleExecutorPopover,
    );
    if !open {
        return toggle;
    }

    let i18n = I18n::new(app.desktop_locale);
    let list = column(choices.into_iter().enumerate().map(|(index, choice)| {
        let (icon, subtitle) = match choice.backend {
            Some(backend) => (acp_agent_icon(backend, 16.0), format!("ACP · {backend}")),
            None => (
                icon_svg(Icon::Hexagon, 16.0)
                    .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
                    .into(),
                i18n.__("识矩原生").to_owned(),
            ),
        };
        agent_option(
            choice.label,
            subtitle,
            icon,
            selected == Some(index),
            Some(TaskBoardMessage::SelectDraftAgent {
                agent: choice.key.to_owned(),
                acp_agent: choice.backend.map(str::to_owned),
            }),
        )
        .into()
    }))
    .spacing(6);
    let menu = container(
        scrollable(container(list).padding([0.0, ACP_SELECTOR_LIST_GUTTER]))
            .direction(Direction::Vertical(
                Scrollbar::new()
                    .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                    .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
            ))
            .height(Length::Shrink),
    )
    .style(agent_popover_style)
    .padding([6, 0])
    .max_height(ACP_SELECTOR_MAX_HEIGHT)
    .width(Length::Fixed(300.0));

    BelowOverlay::new(toggle, menu)
        .show(true)
        .gap(6.0)
        .within_window()
        .dismiss_on_escape()
        .on_close(TaskBoardMessage::CloseExecutorPopover)
        .into()
}

fn build_executor_popover<'a>(
    current_executor: Option<String>,
    app: ExecutorView<'a>,
    kind: TaskBoardExecutorSelectorKind,
) -> Element<'a, Message> {
    let default_label =
        desktop_i18n::I18n::new(app.desktop_locale).__(DEFAULT_TASK_EXECUTOR_LABEL);
    let entries = std::iter::once((DEFAULT_TASK_EXECUTOR_LABEL, default_label, None)).chain(
        app.acp_agents.iter().map(|agent| (agent.as_str(), agent.as_str(), Some(agent.clone()))),
    );
    let executor_list = column(entries.map(|(icon, label, value)| {
        let selected = current_executor == value;
        let mark: Element<'_, Message> = if selected {
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Check))
                .width(14)
                .height(14)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().primary) })
                .into()
        } else {
            Space::new().width(14).height(14).into()
        };
        button(
            row![acp_agent_icon(icon, 14.0), SettingsSelectLabel::new(label.to_owned()), mark]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .width(Length::Fill)
        .height(36)
        .style(move |theme, status| selector_option_style(theme, status, selected))
        .on_press(kind.select_message(value))
        .into()
    }))
    .spacing(2);

    container(
        scrollable(container(executor_list).padding([0.0, ACP_SELECTOR_LIST_GUTTER]))
            .id(iced::widget::Id::new("task_board_executor_selector_scroll"))
            .direction(Direction::Vertical(
                Scrollbar::new()
                    .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                    .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
            ))
            .height(Length::Shrink),
    )
    .style(popover_style)
    .padding([6, 0])
    .max_height(ACP_SELECTOR_MAX_HEIGHT)
    .width(Length::Fixed(216.0))
    .into()
}

#[cfg(test)]
#[path = "executor_selector_tests.rs"]
mod executor_selector_tests;
