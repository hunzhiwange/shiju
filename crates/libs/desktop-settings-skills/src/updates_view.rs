use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{inventory_api::SkillInventoryResponse, repos_api::SkillReposQuery, updates_api::{CheckSkillUpdatesResponse, SkillUpdateTask}, uninstall_api::UninstallSkillResponse};
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
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
pub enum Operation { Update(String), Uninstall(String) }
#[derive(Debug, Clone)]
pub enum Outcome { Updated(SkillUpdateTask), Uninstalled(String, UninstallSkillResponse) }
#[derive(Debug, Clone)]
pub enum Event {
    Open, Close, Refresh, Run(Operation),
    Finished(u64, Result<Outcome, String>),
    Loaded(u64, Result<SkillInventoryResponse, String>, Result<CheckSkillUpdatesResponse, String>),
}

impl State {
    pub fn loaded(&mut self, inventory: Result<SkillInventoryResponse, String>, updates: Result<CheckSkillUpdatesResponse, String>) {
        self.error = None;
        match inventory {
            Ok(data) if Some(&data.scope) == self.scope.as_ref() => self.inventory = Some(data),
            Ok(_) => self.error = Some("库存响应目标不匹配".into()),
            Err(e) => self.error = Some(format!("库存刷新失败：{e}")),
        }
        match updates {
            Ok(data) if Some(&data.scope) == self.scope.as_ref()
                && self.inventory.as_ref().is_some_and(|i| i.version == data.version) => self.updates = Some(data),
            Ok(_) => self.error = Some("更新响应目标或库存版本不匹配，请刷新".into()),
            Err(e) => self.error = Some(format!("更新检查失败：{e}")),
        }
    }
    pub fn finished(&mut self, result: Result<Outcome, String>) {
        self.outcome = Some(match result {
            Ok(Outcome::Updated(task)) if Some(&task.scope) == self.scope.as_ref() =>
                format!("更新任务：{:?}；需核对：{}；内容哈希：{}", task.phase, task.requires_reconciliation, task.content_hash.as_deref().unwrap_or("未提供")),
            Ok(Outcome::Uninstalled(id, response)) if Some(&response.scope) == self.scope.as_ref() => {
                if let Some(cache) = &mut self.updates { cache.updates.retain(|u| u.directory != id); }
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
use desktop_ui_controls::system_settings_common::*;

use iced::{Element, Length};
use iced::widget::{button, column, container, row, text};

pub fn view<'a, M: Clone + 'static>(s: &'a State, host: &str, base: Element<'a, M>, message: fn(Event) -> M) -> Element<'a, M> {
    if !s.open { return base; }
    let mut body = column![text(format!("目标主机：{}", host)),
        text("更新校验已检查的当前哈希。卸载将备份受管内容并保留外部路径。")].spacing(10).width(Length::Fill);
    if let Some(outcome) = &s.outcome { body = body.push(text(outcome.clone())); }
    if let Some(backup) = &s.backup {
        body = body.push(text(format!("备份：{}", backup.backup_id)))
            .push(text(format!("备份路径：{}", backup.backup_path.display())));
        for path in &backup.preserved_paths { body = body.push(text(format!("保留外部路径：{}", path.display()))); }
    }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    if s.busy { body = body.push(text("正在操作或刷新，请等待…")); }
    if let Some(inventory) = &s.inventory {
        body = body.push(text(format!("库存版本：{}", inventory.version)));
        for skill in &inventory.skills {
            let mut card = column![text(skill.name.clone()),
                text(format!("可用版本：{}", skill.version.as_deref().unwrap_or("未提供"))),
                text(format!("当前哈希：{}", skill.content_hash.as_deref().unwrap_or("未提供")))].spacing(6);
            if let Some(info) = s.updates.as_ref().and_then(|u| u.updates.iter().find(|u| u.directory == skill.id)) {
                card = card.push(text(format!("远端哈希：{}", info.remote_hash)))
                    .push(button("确认更新").padding([8, 10]).style(primary_action_btn_style)
                        .on_press_maybe((!s.busy).then(|| message(Event::Run(Operation::Update(skill.id.clone()))))));
            }
            card = card.push(button("确认卸载并备份").padding([8, 10]).style(rounded_action_btn_style)
                .on_press_maybe((!s.busy).then(|| message(Event::Run(Operation::Uninstall(skill.id.clone()))))));
            body = body.push(settings_group_card(card));
        }
    }
    if let Some(updates) = &s.updates {
        for failure in &updates.failures { body = body.push(text(format!("失败仓库：{}/{} ({})：{:?}", failure.owner, failure.name, failure.branch, failure.error))); }
        for failure in &updates.skill_failures { body = body.push(text(format!("技能检查失败：{}：{:?}", failure.directory, failure.error))); }
        for id in &updates.without_repository { body = body.push(text(format!("无仓库来源：{id}"))); }
    }
    let close = message(Event::Close);
    let header = settings_modal_header(text("技能更新与卸载").size(20), settings_close_button(close.clone()));
    let footer = row![button("刷新状态").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe((!s.busy).then(|| message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe((!s.busy).then_some(close.clone()))].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).style(super::modal_card_style).width(760).height(600);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "updates_view_tests.rs"]
mod tests;
