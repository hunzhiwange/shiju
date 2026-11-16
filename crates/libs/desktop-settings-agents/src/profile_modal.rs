//! 智能体资料草稿弹窗；持久化成功前不修改列表快照。
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use crate::message::{AgentsMessage, Event as Message};
use crate::state::*;
use crate::help::settings_hover_help_label;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use config_agent::builtin_agent_spec;

fn message(value: AgentsMessage) -> Message {
    Message::Agents(value)
}

pub fn card<'a>(input: Input<'a>, fields: Element<'a, Message>) -> Option<Element<'a, Message>> {
    if !input.config_ready { None }
    else if input.profile_entering_key { Some(key_form(input)) }
    else { input.draft.map(|draft| profile_form(input, draft, fields)) }
}

fn key_form(input: Input<'_>) -> Element<'_, Message> {
    let label = |source| tr(input.locale, source);
    let submit = message(AgentsMessage::AddAgentRequested);
    let key_label = label("新增智能体 key");
    let body = column![
        settings_field_label(key_label),
        settings_text_input(key_label, input.new_agent_key_input)
            .on_input(|value| message(AgentsMessage::AddAgentKeyChanged(value)))
            .on_submit(submit.clone())
            .width(Length::Fill),
    ]
    .spacing(6);
    let footer = row![
        action(label("取消"), Some(AgentsMessage::ProfileCancel)),
        primary_action(label("下一步"), Some(submit)),
    ]
    .spacing(10);
    container(
        settings_modal_card(
            column![header(&input, label("新增智能体")), body, actions(footer)].spacing(16),
        )
        .width(Length::Fixed(520.0)),
    )
    .padding(12)
    .into()
}

fn profile_form<'a>(
    input: Input<'a>,
    draft: &'a DelegateAgentSettingsEntry,
    fields: Element<'a, Message>,
) -> Element<'a, Message> {
    let settings = &input;
    let busy = settings.profile_saving;
    let label = |source| tr(input.locale, source);
    let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
    let native = match draft.runtime {
        config_agent_profile::AgentRuntime::Native => true,
        config_agent_profile::AgentRuntime::Acp { .. } => false,
    };
    let detail_tab =
        |label, key| super::shared::detail_tab_button(label, key, settings.active_detail_tab);
    let tabs = settings_tab_row(
        [
            Some(detail_tab(label("基础参数"), crate::state::AGENT_DETAIL_BASIC_TAB)),
            Some(detail_tab(label("身份"), crate::state::AGENT_DETAIL_IDENTITY_TAB)),
            native.then(|| detail_tab(label("工具"), crate::state::AGENT_DETAIL_TOOLS_TAB)),
            Some(detail_tab(label("技能"), crate::state::AGENT_DETAIL_SKILLS_TAB)),
            Some(detail_tab("MCP", crate::state::AGENT_DETAIL_MCP_TAB)),
        ]
        .into_iter()
        .flatten(),
    )
    .wrap();
    let mut footer =
        row![action(label("取消"), (!busy).then_some(AgentsMessage::ProfileCancel))].spacing(10);
    if !fixed {
        footer = footer
            .push(action(label("删除智能体"), (!busy).then_some(AgentsMessage::ProfileDelete)));
    }
    footer = footer.push(primary_action(
        label(if busy { "保存中…" } else { "保存配置" }),
        (!busy).then_some(message(AgentsMessage::ProfileSave)),
    ));
    let header = column![
        header(&input, label("智能体配置")),
        tabs,
        text(label("身份文件正文即时保存；其余配置点击保存后生效。"))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    let card = settings_modal_layout(header, fields, Some(actions(footer)))
        .width(Length::Fixed(1000.0))
        .height(Length::Fill)
        .max_height(820);
    container(card).padding(12).into()
}

fn header<'a>(input: &Input<'a>, title: &'a str) -> Element<'a, Message> {
    settings_modal_header(
        settings_hover_help_label(input.help,
            I18n::new(input.locale),
            input.theme,
            title,
            text(title).size(20),
            tr(input.locale, crate::profile_modal::AGENT_CONFIGURATION_HELP),
        )
        .wrap(),
        settings_close_button(message(AgentsMessage::ProfileCancel)),
    )
}

fn actions(footer: iced::widget::Row<'_, Message>) -> Element<'_, Message> {
    container(footer.align_y(Alignment::Center).wrap()).align_right(Length::Fill).into()
}

fn primary_action(label: &'static str, value: Option<Message>) -> Element<'static, Message> {
    button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
        .on_press_maybe(value)
        .padding([8, 14])
        .style(primary_action_btn_style)
        .into()
}

fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label))
        .on_press_maybe(value.map(message))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}

pub const AGENT_CONFIGURATION_HELP: &str =
    "全局管理内建与自定义智能体的模型、身份文件、工具、技能和运行参数。";
pub struct Input<'a> { pub help: crate::help::Renderer, pub locale: config_ui::DesktopLocale, pub theme: &'a iced::Theme, pub config_ready: bool, pub profile_entering_key: bool, pub new_agent_key_input: &'a str, pub draft: Option<&'a DelegateAgentSettingsEntry>, pub profile_saving: bool, pub active_detail_tab: &'a str }

/// Borrowed page context. Session capability and timestamp projection stay at their owners.
pub struct PageInput<'a> {
    pub state: &'a crate::agents_state::AgentsSettingsState,
    pub locale: config_ui::DesktopLocale,
    pub theme: &'a iced::Theme,
    pub help: crate::help::Renderer,
    pub acp_agents: &'a [String],
    pub acp_loading: bool,
    pub acp_error: Option<&'a String>,
    pub inspect: Option<&'a vw_api_mcp_inspect::McpInspectResponse>,
    pub inspect_loading: bool,
    pub inspect_error: Option<&'a String>,
    pub model_source_available: bool,
    pub workspace_metadata: Option<String>,
    pub project_options: Vec<String>,
}
pub fn page_card(input: PageInput<'_>) -> Option<Element<'_, Message>> {
    let state = input.state;
    let draft = state.profile_modal.as_ref();
    let defaults = || {
        crate::defaults::catalog_input(
            state,
            input.locale,
            input.theme,
            input.help,
            input.model_source_available,
            input.inspect,
            input.inspect_loading,
            input.inspect_error,
        )
    };
    let fields = if let Some(entry) = draft {
        match state.active_detail_tab.as_str() {
            crate::state::AGENT_DETAIL_IDENTITY_TAB => crate::identity::view(
                crate::identity::Input {
                    help: input.help,
                    locale: input.locale,
                    theme: input.theme,
                    active_prompt_tab: &state.active_prompt_tab,
                    workspace_identity_files: &state.workspace_identity_files,
                    workspace_identity_root_path: state.workspace_identity_root_path.as_deref(),
                    selected_agent: &state.selected_agent,
                    workspace_metadata: input.workspace_metadata,
                },
                entry,
            ),
            crate::state::AGENT_DETAIL_TOOLS_TAB
                if matches!(entry.runtime, config_agent_profile::AgentRuntime::Native) =>
            {
                crate::tools::view(
                    crate::tools::Input {
                        locale: input.locale,
                        theme: input.theme,
                        help: input.help,
                        available_tools: &state.available_tools,
                        active_tools_list_tab: &state.active_tools_list_tab,
                    },
                    entry,
                )
            }
            crate::state::AGENT_DETAIL_SKILLS_TAB => crate::skills::view(
                crate::skills::Input {
                    locale: input.locale,
                    theme: input.theme,
                    help: input.help,
                    project_options: input.project_options,
                    skill_directory: &state.skill_directory,
                    skill_directory_scope: state.skill_directory_scope,
                    skill_catalog: &state.skill_catalog,
                    skill_catalog_loading: state.skill_catalog_loading,
                    skill_catalog_error: &state.skill_catalog_error,
                },
                entry,
            ),
            crate::state::AGENT_DETAIL_MCP_TAB => crate::mcp::view(
                crate::mcp::Input {
                    help: input.help,
                    locale: input.locale,
                    theme: input.theme,
                    inspect: input.inspect,
                    inspect_loading: input.inspect_loading,
                    inspect_error: input.inspect_error,
                    defaults: defaults(),
                },
                entry,
            ),
            _ => crate::basic::view(
                crate::basic::Input {
                    help: input.help,
                    locale: input.locale,
                    theme: input.theme,
                    profile: crate::basic_profile::Input {
                        locale: input.locale,
                        entries: &state.entries,
                        acp_agents: input.acp_agents,
                        saving: state.profile_saving,
                        acp_loading: input.acp_loading,
                        acp_error: input.acp_error,
                    },
                    defaults: defaults(),
                },
                entry,
                Vec::new(),
            ),
        }
    } else {
        iced::widget::Space::new().into()
    };
    card(
        Input {
            help: input.help,
            locale: input.locale,
            theme: input.theme,
            config_ready: state.config_ready,
            profile_entering_key: state.profile_entering_key,
            new_agent_key_input: &state.new_agent_key_input,
            draft,
            profile_saving: state.profile_saving,
            active_detail_tab: &state.active_detail_tab,
        },
        fields,
    )
}
