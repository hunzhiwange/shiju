use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{
    bindings_api::{SkillBindingStatus, SkillBindingsResponse},
    repos_api::SkillReposQuery,
    storage_api::{SkillStorageLocation, SkillStorageMoveResponse, SkillStorageResponse},
};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub current: Option<SkillStorageResponse>,
    pub confirm_move: bool,
    pub pending: Option<SkillStorageLocation>,
    pub results: Vec<String>,
    pub error: Option<String>,
    pub generation: u64,
    pub catalog: u64,
    pub context: Option<AgentAppsContext>,
    pub scope: Option<SkillReposQuery>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Select(SkillStorageLocation),
    Mode(SkillBindingMode),
    Move,
    ConfirmMove,
    CancelMove,
    Resync,
    OpenDirectory,
    Completed(u64, Result<Readback, String>),
}
#[derive(Clone, Debug)]
pub enum Readback {
    Storage(SkillStorageResponse),
    Move(SkillStorageMoveResponse),
    Sync(SkillBindingsResponse),
    Opened,
}

impl State {
    pub fn accept(&mut self, result: Readback) -> Result<(), String> {
        match result {
            Readback::Opened => {}
            Readback::Storage(value) if Some(&value.scope) == self.scope.as_ref() => {
                self.current = Some(value);
                self.pending = None;
                self.confirm_move = false;
            }
            Readback::Move(value) if Some(&value.storage.scope) == self.scope.as_ref() => {
                self.confirm_move = false;
                self.results = value
                    .migrated
                    .iter()
                    .map(|id| format!("已迁移：{id}"))
                    .chain(value.skipped.iter().map(|id| format!("已跳过：{id}")))
                    .collect();
                self.results.extend(value.errors.iter().cloned());
                if !value.switched || !value.errors.is_empty() {
                    return Err("迁移未完成，保留原主副本位置；请刷新确认执行端状态".into());
                }
                self.current = Some(value.storage);
                self.pending = None;
            }
            Readback::Sync(value) if Some(&value.scope) == self.scope.as_ref() => {
                self.results = value
                    .results
                    .iter()
                    .map(|item| {
                        format!(
                            "{} / {}：{:?}",
                            item.target.installation_id, item.directory, item.status
                        )
                    })
                    .collect();
                if value.results.iter().any(|item| item.status == SkillBindingStatus::Failed) {
                    return Err("部分同步失败，请检查逐项结果".into());
                }
            }
            _ => return Err("响应作用域不匹配，请刷新".into()),
        }
        Ok(())
    }
}
// Execution-host storage and synchronization controls.
use crate::third_party::style;
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{icons::icon_btn_svg_style, *};
use iced::{
    Alignment, Element, Font, Length, Padding,
    widget::{Space, button, column, container, radio, row, text},
};
use vw_api_app_skills::bindings_api::SkillBindingMode;

const OPTION_INDENT: u32 = 26;
const HEADING: Font = Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT };

pub fn view<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    skill_count: Option<usize>,
    can_open_directory: bool,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let control = |label: &'static str, event, enabled| {
        button(text(label).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy && enabled).then(|| message(event)))
    };
    let muted = |label| text(label).size(12).style(settings_muted_text_style);
    let mut body = column![].spacing(16).width(Length::Fill);
    if let Some(current) = &s.current {
        let current_agents = std::path::Path::new(&current.root).ends_with(".agents/skills");
        let selected_agents = s
            .pending
            .map_or(current_agents, |location| matches!(location, SkillStorageLocation::Agents));
        let mut locations = column![text(i18n.__("主副本位置")).font(HEADING).size(15)].spacing(11);
        for (agents, location, label) in [
            (false, SkillStorageLocation::Runtime, i18n.__("ShiJu 运行时目录")),
            (true, SkillStorageLocation::Agents, i18n.__("通用目录")),
        ] {
            let mut option = column![
                radio(label, agents, Some(selected_agents), move |_| message(Event::Select(
                    location
                )))
                .size(16)
                .text_size(14)
                .spacing(10)
                .style(style::radio),
            ]
            .spacing(4);
            if current_agents == agents {
                let mut path =
                    row![text(current.root.as_str()).size(12).style(settings_muted_text_style),]
                        .spacing(5)
                        .align_y(Alignment::Center);
                if can_open_directory {
                    path = path.push(
                        button(text(i18n.__("打开")).size(12))
                            .padding([3, 5])
                            .style(|theme, status| settings_tab_style(theme, status, false))
                            .on_press_maybe((!s.busy).then(|| message(Event::OpenDirectory))),
                    );
                }
                option =
                    option.push(container(path.wrap()).padding(Padding::ZERO.left(OPTION_INDENT)));
            }
            if agents {
                option = option.push(
                    container(
                        text(i18n.__(
                            "部分应用会直接读取通用目录，单独停用 Skill 可能不会阻止这些应用读取。",
                        ))
                        .size(12)
                        .style(|theme: &iced::Theme| text::Style {
                            color: Some(theme.extended_palette().warning.strong.color),
                        }),
                    )
                    .padding(Padding::ZERO.left(OPTION_INDENT)),
                );
            }
            locations = locations.push(option);
        }
        let move_hint = skill_count.map_or_else(
            || i18n.__("切换位置会移动现有 Skill。").to_owned(),
            |count| {
                desktop_i18n::__!(i18n, "切换位置会移动 %s 个 Skill。", (count).to_string())
                    .expect("valid extension display template")
            },
        );
        if s.confirm_move {
            locations = locations.push(
                row![
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "确认移动到%s？",
                            (if selected_agents {
                                i18n.__("通用目录")
                            } else {
                                i18n.__("ShiJu 运行时目录")
                            })
                            .to_string()
                        )
                        .expect("valid extension display template")
                    )
                    .size(13)
                    .width(Length::Fill),
                    control(i18n.__("取消"), Event::CancelMove, true),
                    button(text(i18n.__("确认迁移")).size(13))
                        .padding([8, 12])
                        .style(style::primary)
                        .on_press_maybe((!s.busy).then(|| message(Event::ConfirmMove))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            locations = locations.push(
                row![
                    muted(move_hint).width(Length::Fill),
                    control(i18n.__("移动并切换"), Event::Move, selected_agents != current_agents),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        body = body.push(locations).push(settings_divider());

        let selected_mode = match current.mode {
            SkillBindingMode::Auto => 0,
            SkillBindingMode::Symlink => 1,
            SkillBindingMode::Copy => 2,
        };
        let mut modes =
            column![text(i18n.__("同步到应用的方式")).font(HEADING).size(15)].spacing(10);
        for (index, (mode, label, hint)) in [
            (SkillBindingMode::Auto, i18n.__("自动"), i18n.__("优先使用软链接，失败时复制")),
            (
                SkillBindingMode::Symlink,
                i18n.__("只用软链接"),
                i18n.__("Windows 可能需要开启开发者模式"),
            ),
            (SkillBindingMode::Copy, i18n.__("只用复制"), i18n.__("占用更多空间，更新时重新复制")),
        ]
        .into_iter()
        .enumerate()
        {
            modes = modes.push(
                row![
                    radio(label, index, Some(selected_mode), move |_| message(Event::Mode(mode)))
                        .size(16)
                        .text_size(14)
                        .spacing(10)
                        .style(style::radio),
                    text(hint).size(12).style(settings_muted_text_style),
                ]
                .spacing(9)
                .align_y(Alignment::Center)
                .wrap(),
            );
        }
        modes = modes.push(
            row![
                text(i18n.__("新方式用于之后启用或更新的 Skill。已启用的 Skill 可立即重新同步。"))
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
                button(
                    row![
                        icon_svg(Icon::ArrowClockwise, 14.0).style(icon_btn_svg_style),
                        text(i18n.__("立即重新同步")).size(13),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.busy).then(|| message(Event::Resync))),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
        body = body.push(modes);
    } else if s.busy {
        body =
            body.push(text(i18n.__("正在读取存储设置…")).size(13).style(settings_muted_text_style));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    if !s.results.is_empty() {
        let mut feedback = column![text(i18n.__("操作结果")).size(14).font(HEADING)].spacing(5);
        for result in &s.results {
            feedback = feedback.push(
                text(desktop_i18n::settings_extensions::feedback(i18n, result))
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        body = body.push(settings_divider()).push(feedback);
    }
    if s.busy && s.current.is_some() {
        body = body.push(text(i18n.__("正在处理…")).size(12).style(settings_muted_text_style));
    }
    let close = message(Event::Close);
    let header = text(i18n.__("存储与同步")).size(20).font(HEADING);
    let mut footer = row![Space::new().width(Length::Fill)].spacing(8).align_y(Alignment::Center);
    if s.error.is_some() {
        footer = footer.push(control(i18n.__("刷新状态"), Event::Refresh, true));
    }
    let footer = container(
        footer.push(
            button(text(i18n.__("完成")).size(13))
                .padding([8, 16])
                .style(style::primary)
                .on_press_maybe((!s.busy).then_some(close.clone())),
        ),
    );
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .padding([24, 17])
        .style(style::dialog)
        .width(530)
        .height(if s.current.is_some() { 515 } else { 245 });
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
}

#[cfg(test)]
#[path = "storage_modal_tests.rs"]
mod tests;
