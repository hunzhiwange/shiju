use crate::message::{Event, AgentsMessage, AcpMessage};
use crate::state::*;
// 基础参数中的资料字段；描述编辑缓冲不参与持久化。
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Element, Length};
use config_agent_profile::AgentRuntime;
use config_agent::builtin_agent_spec;

fn message(value: AgentsMessage) -> Event {
    Event::Agents(value)
}
pub fn view<'a>(
    input: Input<'a>,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let locale = input.locale;
    let label = move |source| tr(locale, source);
    let fixed = builtin_agent_spec(&draft.key).is_some()
        || draft.key.starts_with("acp-")
        || input.entries.iter().any(|entry| entry.key == draft.key);
    let text_input = |placeholder, value, callback: fn(String) -> AgentsMessage| {
        settings_text_input(placeholder, value).on_input(move |value| message(callback(value)))
    };
    let name = settings_responsive_row(label("名称"), "", move || {
        text_input(label("智能体名称"), &draft.label, AgentsMessage::ProfileNameChanged)
            .width(Length::Fill)
    });
    let description = column![
        settings_field_label(label("描述")),
        settings_text_editor(&draft.description_editor)
            .on_action(|action| message(AgentsMessage::ProfileDescriptionAction(action)))
            .font(iced::Font::DEFAULT)
            .height(120),
    ]
    .spacing(6);
    let mut prompts =
        column![settings_field_label(label("推荐提示词"))].spacing(10).width(Length::Fill);
    let prompt_count = draft.recommended_prompts.len();
    let locale = input.locale;
    for (index, prompt) in draft.recommended_prompts.iter().enumerate() {
        prompts = prompts.push(
            responsive(move |size| {
                let input = settings_text_input(tr(locale, "推荐提示词"), prompt)
                    .on_input(move |value| {
                        message(AgentsMessage::ProfilePromptChanged(index, value))
                    })
                    .width(Length::Fill);
                let actions = row![
                    action(
                        tr(locale, "上移"),
                        (index > 0).then_some(AgentsMessage::ProfilePromptMove(
                            index,
                            index.saturating_sub(1)
                        ))
                    ),
                    action(
                        tr(locale, "下移"),
                        (index + 1 < prompt_count)
                            .then_some(AgentsMessage::ProfilePromptMove(index, index + 1))
                    ),
                    action(tr(locale, "移除"), Some(AgentsMessage::ProfilePromptRemove(index))),
                ]
                .spacing(6);
                if size.width < 440.0 {
                    column![input, actions.wrap()].spacing(6).into()
                } else {
                    row![input, actions].spacing(6).align_y(Alignment::Center).into()
                }
            })
            .height(Length::Shrink),
        );
    }
    prompts = prompts.push(action(label("添加提示词"), Some(AgentsMessage::ProfilePromptAdd)));
    let runtime = match &draft.runtime {
        AgentRuntime::Native => "native".to_string(),
        AgentRuntime::Acp { backend } => backend.clone(),
    };
    let runtime = settings_responsive_row(
        label("运行智能体"),
        if fixed {
            ""
        } else {
            label("首次保存成功后，运行引擎及 ACP backend 不可修改。")
        },
        move || -> Element<'_, Event> {
            if fixed {
                text(
                    desktop_i18n::__!(I18n::new(input.locale), "固定运行引擎：%s", runtime.as_str())
                        .expect("valid fixed agent runtime template"),
                )
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .wrapping(text::Wrapping::WordOrGlyph)
                .into()
            } else {
                let mut options = vec!["native".to_string()];
                options.extend(input.acp_agents.iter().cloned());
                let busy = input.saving;
                let picker = settings_select(options, Some(runtime.clone()), |value| {
                    message(AgentsMessage::ProfileRuntimeChanged(value))
                })
                .width(Length::Fill);
                let mut controls = column![picker].spacing(8);
                if input.acp_agents.is_empty() {
                    controls = controls.push(
                        text(label(if input.acp_loading {
                            "正在加载 ACP 引擎目录…"
                        } else {
                            "暂无可用 ACP 引擎，可刷新目录或在 ACP 设置中启用。"
                        }))
                        .size(12)
                        .style(settings_muted_text_style),
                    );
                }
                if let Some(error) = input.acp_error {
                    controls = controls.push(settings_error_banner(error.clone()));
                }
                controls = controls.push(
                    button(
                        text(label(if input.acp_loading {
                            "加载中…"
                        } else {
                            "刷新引擎目录"
                        }))
                        .size(SETTINGS_CONTROL_TEXT_SIZE),
                    )
                    .on_press_maybe((!busy && !input.acp_loading).then_some(
                        Event::Acp(
                            AcpMessage::Refresh,
                        ),
                    ))
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                );
                controls.into()
            }
        },
    );
    settings_group_card(
        container(column![name, description, prompts, settings_divider(), runtime].spacing(14))
            .padding([14, 0]),
    )
    .padding([0, 16])
    .into()
}
fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Event> {
    button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
        .on_press_maybe(value.map(message))
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .into()
}

#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub locale: config_ui::DesktopLocale,
    pub entries: &'a [DelegateAgentSettingsEntry],
    pub acp_agents: &'a [String],
    pub saving: bool,
    pub acp_loading: bool,
    pub acp_error: Option<&'a String>,
}
