use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{
    inventory_api::SkillInventoryResponse,
    repos_api::SkillReposQuery,
    uninstall_api::UninstallSkillResponse,
    updates_api::{CheckSkillUpdatesResponse, SkillUpdateTask},
};
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub uninstall_skill: Option<String>,
    pub busy: bool,
    pub inventory: Option<SkillInventoryResponse>,
    pub updates: Option<CheckSkillUpdatesResponse>,
    pub outcome: Option<String>,
    pub backup: Option<UninstallSkillResponse>,
    pub error: Option<String>,
    pub generation: u64,
    pub context: Option<AgentAppsContext>,
    pub scope: Option<SkillReposQuery>,
    pub catalog: u64,
}
#[derive(Debug, Clone)]
pub enum Operation {
    Update(String),
    Uninstall(String),
}
#[derive(Debug, Clone)]
pub enum Outcome {
    Updated(SkillUpdateTask),
    Uninstalled(String, UninstallSkillResponse),
}
#[derive(Debug, Clone)]
pub enum Event {
    Open,
    OpenForUninstall(String),
    Close,
    Refresh,
    Run(Operation),
    Finished(u64, Result<Outcome, String>),
    Loaded(u64, Result<SkillInventoryResponse, String>, Result<CheckSkillUpdatesResponse, String>),
}

impl State {
    pub fn loaded(
        &mut self,
        inventory: Result<SkillInventoryResponse, String>,
        updates: Result<CheckSkillUpdatesResponse, String>,
    ) {
        self.error = None;
        match inventory {
            Ok(data) if Some(&data.scope) == self.scope.as_ref() => self.inventory = Some(data),
            Ok(_) => self.error = Some("库存响应目标不匹配".into()),
            Err(e) => self.error = Some(format!("库存刷新失败：{e}")),
        }
        match updates {
            Ok(data)
                if Some(&data.scope) == self.scope.as_ref()
                    && self.inventory.as_ref().is_some_and(|i| i.version == data.version) =>
            {
                self.updates = Some(data)
            }
            Ok(_) => self.error = Some("更新响应目标或库存版本不匹配，请刷新".into()),
            Err(e) => self.error = Some(format!("更新检查失败：{e}")),
        }
    }
    pub fn finished(&mut self, result: Result<Outcome, String>) {
        self.outcome = Some(match result {
            Ok(Outcome::Updated(task)) if Some(&task.scope) == self.scope.as_ref() => format!(
                "更新任务：{:?}；需核对：{}；内容哈希：{}",
                task.phase,
                task.requires_reconciliation,
                task.content_hash.as_deref().unwrap_or("未提供")
            ),
            Ok(Outcome::Uninstalled(id, response))
                if Some(&response.scope) == self.scope.as_ref() =>
            {
                if let Some(cache) = &mut self.updates {
                    cache.updates.retain(|u| u.directory != id);
                }
                let result = format!("已卸载；清理未完成：{}", response.cleanup_incomplete);
                self.backup = Some(response);
                result
            }
            Ok(_) => "操作响应目标不匹配，请核对服务端状态".into(),
            Err(e) => format!("操作请求失败：{e}；正在重新查询实际状态"),
        });
        // Any mutation, including an ambiguous transport failure, invalidates revision-bound writes.
        self.inventory = None;
        self.updates = None;
    }
}
use crate::third_party::style;
use desktop_ui_controls::system_settings_common::*;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, alignment::Horizontal};

fn detail<'a>(value: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(value).size(13).wrapping(text::Wrapping::WordOrGlyph).style(settings_muted_text_style)
}

pub fn view<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    s: &'a State,
    _host: &str,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    if let Some(id) = &s.uninstall_skill {
        return uninstall_view(i18n, s, id, base, message);
    }
    let mut body =
        column![detail(i18n.__("更新校验已检查的当前哈希。卸载将备份受管内容并保留外部路径。"))]
            .spacing(12)
            .width(Length::Fill);
    if let Some(outcome) = &s.outcome {
        body = body.push(text(outcome).size(14).wrapping(text::Wrapping::WordOrGlyph));
    }
    if let Some(backup) = &s.backup {
        body = body
            .push(detail(
                desktop_i18n::__!(i18n, "备份：%s", (backup.backup_id).to_string())
                    .expect("valid extension display template"),
            ))
            .push(detail(
                desktop_i18n::__!(i18n, "备份路径：%s", (backup.backup_path.display()).to_string())
                    .expect("valid extension display template"),
            ));
        for path in &backup.preserved_paths {
            body = body.push(detail(
                desktop_i18n::__!(i18n, "保留外部路径：%s", (path.display()).to_string())
                    .expect("valid extension display template"),
            ));
        }
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    if s.busy {
        body = body.push(detail(i18n.__("正在操作或刷新，请等待…")));
    }
    let updates = s.updates.as_ref();
    if let Some(inventory) = &s.inventory {
        body = body.push(
            row![
                text(i18n.__("已安装技能")).size(15).width(Length::Fill),
                detail(
                    desktop_i18n::__!(i18n, "库存版本：%s", (inventory.version).to_string())
                        .expect("valid extension display template"),
                ),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        );
        if inventory.skills.is_empty() {
            body = body.push(settings_panel(detail(i18n.__("当前范围尚未安装可管理技能"))));
        }
        for skill in &inventory.skills {
            let mut card = column![
                text(&skill.name).size(16).wrapping(text::Wrapping::WordOrGlyph),
                detail(
                    desktop_i18n::__!(
                        i18n,
                        "可用版本：%s",
                        (skill.version.as_deref().unwrap_or(i18n.__("未提供"))).to_string()
                    )
                    .expect("valid extension display template")
                ),
                detail(
                    desktop_i18n::__!(
                        i18n,
                        "当前哈希：%s",
                        (skill.content_hash.as_deref().unwrap_or(i18n.__("未提供"))).to_string()
                    )
                    .expect("valid extension display template")
                )
            ]
            .spacing(5)
            .width(Length::Fill);
            let mut actions = row![
                button(text(i18n.__("确认卸载并备份")).size(13))
                    .padding([8, 12])
                    .style(danger_action_btn_style)
                    .on_press_maybe(
                        (!s.busy)
                            .then(|| message(Event::Run(Operation::Uninstall(skill.id.clone())))),
                    ),
            ];
            if let Some(info) =
                updates.and_then(|u| u.updates.iter().find(|u| u.directory == skill.id))
            {
                card = card.push(detail(
                    desktop_i18n::__!(i18n, "远端哈希：%s", (info.remote_hash).to_string())
                        .expect("valid extension display template"),
                ));
                actions = actions.push(
                    button(text(i18n.__("确认更新")).size(13))
                        .padding([8, 12])
                        .style(primary_action_btn_style)
                        .on_press_maybe(
                            (!s.busy)
                                .then(|| message(Event::Run(Operation::Update(skill.id.clone())))),
                        ),
                );
            }
            if updates.is_some_and(|u| u.without_repository.contains(&skill.id)) {
                card = card.push(detail(i18n.__("无仓库来源")));
            }
            body = body.push(
                settings_panel(
                    column![
                        card,
                        container(actions.spacing(8).wrap().align_x(Horizontal::Right))
                            .align_right(Length::Fill),
                    ]
                    .spacing(12),
                )
                .padding([12, 14]),
            );
        }
    }
    if let Some(updates) = updates {
        for failure in &updates.failures {
            body = body.push(detail(
                desktop_i18n::__!(
                    i18n,
                    "失败仓库：%s/%s (%s)：%s",
                    (failure.owner).to_string(),
                    (failure.name).to_string(),
                    (failure.branch).to_string(),
                    (format!("{:?}", failure.error)).to_string()
                )
                .expect("valid extension display template"),
            ));
        }
        for failure in &updates.skill_failures {
            body = body.push(detail(
                desktop_i18n::__!(
                    i18n,
                    "技能检查失败：%s：%s",
                    (failure.directory).to_string(),
                    (format!("{:?}", failure.error)).to_string()
                )
                .expect("valid extension display template"),
            ));
        }
        for id in &updates.without_repository {
            if s.inventory
                .as_ref()
                .is_some_and(|inventory| inventory.skills.iter().any(|skill| &skill.id == id))
            {
                continue;
            }
            body = body.push(detail(
                desktop_i18n::__!(i18n, "无仓库来源：%s", (id).to_string())
                    .expect("valid extension display template"),
            ));
        }
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("技能更新与卸载")).size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        button(text(i18n.__("刷新状态")).size(13))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then(|| message(Event::Refresh))),
        button(text(i18n.__("关闭")).size(13))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone()))
    ]
    .spacing(8);
    let footer = column![
        settings_divider(),
        container(footer.wrap().align_x(Horizontal::Right)).align_right(Length::Fill),
    ]
    .spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(style::dialog)
        .width(620)
        .height(380);
    settings_modal_overlay(Some(base), close, card)
}

fn uninstall_view<'a, M: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    s: &'a State,
    id: &str,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    let skill = s.inventory.as_ref().and_then(|inventory| {
        inventory.skills.iter().find(|skill| skill.id == id && skill.present)
    });
    let mut body = column![].spacing(10);
    if let Some(outcome) = &s.outcome {
        body = body.push(text(outcome).size(14));
    } else if let Some(skill) = skill {
        body = body
            .push(
                text(
                    desktop_i18n::__!(i18n, "卸载「%s」？", (skill.name).to_string())
                        .expect("valid extension display template"),
                )
                .size(16),
            )
            .push(
                text(i18n.__("卸载会先创建备份，之后可从备份恢复。"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
    } else if s.busy {
        body = body.push(text(i18n.__("正在读取技能…")).size(13));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
            i18n,
            &error.clone(),
        )));
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("卸载 Skill")).size(20),
        settings_close_button(close.clone()),
    );
    let mut footer = row![
        iced::widget::Space::new().width(Length::Fill),
        button(if s.outcome.is_some() { i18n.__("关闭") } else { i18n.__("取消") })
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ]
    .spacing(8);
    if s.outcome.is_none() {
        footer = footer.push(
            button(i18n.__("卸载并备份"))
                .padding([8, 10])
                .style(danger_action_btn_style)
                .on_press_maybe(
                    (!s.busy && skill.is_some())
                        .then(|| message(Event::Run(Operation::Uninstall(id.to_owned())))),
                ),
        );
    }
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(style::dialog)
        .width(440)
        .height(245);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
}

#[cfg(test)]
#[path = "updates_view_tests.rs"]
mod tests;
