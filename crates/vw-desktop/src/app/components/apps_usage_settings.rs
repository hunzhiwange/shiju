//! Native usage settings for the selected application provider.

use std::fmt;

use iced::Task;
use vw_gateway_client::{
    AppsUsageCredentialRefs, AppsUsageDisplay, AppsUsageReadError, AppsUsageSettings,
    AppsUsageSigningCredentialRefs, AppsUsageWriteError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        apps_usage::{
            UsageSnapshotQuery,
            settings::{
                MAX_REFRESH_INTERVAL_MINUTES, UsageCodingPlan, UsageCredentialId,
                valid_usage_team_id,
            },
        },
    },
};

use super::{
    apps_usage_cards::{Scope, current_scope, scope},
    system_settings::SystemTab,
};
use crate::app::{
    App, Message,
    message::app_providers::list::{ListMessage, message},
};

mod view;
pub(crate) use view::overlay;

#[cfg(test)]
#[path = "apps_usage_settings_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Balance,
    CodingPlan,
    ZhipuTeam,
    Volcengine,
    OfficialSubscription,
}

impl Kind {
    fn template(self) -> &'static str {
        match self {
            Self::Balance => "balance",
            Self::CodingPlan | Self::ZhipuTeam | Self::Volcengine => "token_plan",
            Self::OfficialSubscription => "official_subscription",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Balance => "供应商余额",
            Self::CodingPlan => "编程套餐（自动识别）",
            Self::ZhipuTeam => "智谱团队套餐",
            Self::Volcengine => "火山引擎套餐",
            Self::OfficialSubscription => "官方订阅额度",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Field {
    Organization,
    Project,
    AccessKey,
    SecretKey,
    Interval,
}

#[derive(Clone, Default)]
pub(super) struct Draft {
    kind: Option<Kind>,
    pub(super) enabled: bool,
    pub(super) display: AppsUsageDisplay,
    organization: String,
    project: String,
    access_key: String,
    secret_key: String,
    interval: String,
}

impl fmt::Debug for Draft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Draft").field("kind", &self.kind).finish_non_exhaustive()
    }
}

impl Draft {
    pub(super) fn new(saved: Option<AppsUsageSettings>) -> Self {
        let mut draft = Self { interval: "5".into(), ..Self::default() };
        if let Some(saved) = saved {
            draft.enabled = saved.enabled;
            draft.interval = saved.refresh_interval_minutes.to_string();
            draft.display = saved.display;
            draft.kind = match (saved.template.as_str(), saved.coding_plan) {
                ("balance", _) => Some(Kind::Balance),
                ("official_subscription", _) => Some(Kind::OfficialSubscription),
                ("token_plan", None) => Some(Kind::CodingPlan),
                (
                    "token_plan",
                    Some(UsageCodingPlan::ZhipuTeam { organization_id, project_id }),
                ) => {
                    draft.organization = organization_id;
                    draft.project = project_id;
                    Some(Kind::ZhipuTeam)
                }
                ("token_plan", Some(UsageCodingPlan::Volcengine)) => {
                    if let Some(signing) = saved.credentials.signing {
                        draft.access_key = signing.access_key_id_ref;
                        draft.secret_key = signing.secret_access_key_ref;
                    }
                    Some(Kind::Volcengine)
                }
                _ => None,
            };
        }
        draft
    }

    pub(super) fn select(&mut self, kind: Kind) {
        if self.kind == Some(kind) {
            return;
        }
        if self.kind.is_none() || kind == Kind::Volcengine {
            self.enabled = kind != Kind::Volcengine;
        }
        self.kind = Some(kind);
    }

    pub(super) fn edit(&mut self, field: Field, value: String) {
        *match field {
            Field::Organization => &mut self.organization,
            Field::Project => &mut self.project,
            Field::AccessKey => &mut self.access_key,
            Field::SecretKey => &mut self.secret_key,
            Field::Interval => &mut self.interval,
        } = value;
    }

    pub(super) fn settings(&self) -> Result<AppsUsageSettings, &'static str> {
        let kind = self.kind.ok_or("请选择查询类型")?;
        let interval = self
            .interval
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|value| *value <= MAX_REFRESH_INTERVAL_MINUTES)
            .ok_or("刷新间隔须为 0–1440 分钟的整数")?;
        let mut credentials = AppsUsageCredentialRefs::default();
        let coding_plan = match kind {
            Kind::ZhipuTeam => {
                let organization = self.organization.trim();
                let project = self.project.trim();
                if !valid_usage_team_id(organization) || !valid_usage_team_id(project) {
                    return Err("请填写有效的团队组织 ID 和项目 ID");
                }
                Some(UsageCodingPlan::ZhipuTeam {
                    organization_id: organization.into(),
                    project_id: project.into(),
                })
            }
            Kind::Volcengine => {
                let access = self.access_key.trim();
                let secret = self.secret_key.trim();
                if self.enabled || !access.is_empty() || !secret.is_empty() {
                    let access = UsageCredentialId::try_from(access.to_owned())
                        .map_err(|_| "请填写管理员提供的 AK 和 SK 引用")?;
                    let secret = UsageCredentialId::try_from(secret.to_owned())
                        .map_err(|_| "请填写管理员提供的 AK 和 SK 引用")?;
                    if access == secret {
                        return Err("AK 和 SK 必须使用各自的引用");
                    }
                    credentials.signing = Some(AppsUsageSigningCredentialRefs {
                        access_key_id_ref: access.into(),
                        secret_access_key_ref: secret.into(),
                    });
                }
                Some(UsageCodingPlan::Volcengine)
            }
            _ => None,
        };
        Ok(AppsUsageSettings {
            template: kind.template().into(),
            enabled: self.enabled,
            refresh_interval_minutes: interval,
            display: self.display,
            credentials,
            coding_plan,
        })
    }
}

#[derive(Clone)]
pub enum Event {
    Open(AppId, String),
    Close,
    Select(Kind),
    Edit(Field, String),
    Enabled(bool),
    DisplayUsed(bool),
    Save,
    Loaded(u64, Result<Option<AppsUsageSettings>, AppsUsageReadError>),
    Saved(u64, Result<(), AppsUsageWriteError>),
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UsageSettingsEvent")
    }
}

#[derive(Clone, Debug)]
enum Phase {
    Loading,
    LoadFailed,
    Editing { draft: Draft, saving: bool },
}

#[derive(Clone, Debug)]
struct Modal {
    scope: Scope,
    selection: UsageSnapshotQuery,
    ticket: u64,
    phase: Phase,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Editor {
    sequence: u64,
    modal: Option<Modal>,
}

impl Editor {
    pub(super) fn clear(&mut self) {
        self.modal = None;
    }
}

fn send(event: Event) -> Message {
    message(ListMessage::UsageSettings(event))
}

fn live_scope(app: &App) -> Option<Scope> {
    (app.show_system_settings && app.system_settings_tab == SystemTab::Apps)
        .then(|| current_scope(app, false))
        .flatten()
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let completed = matches!(event, Event::Loaded(..) | Event::Saved(..));
    if !completed && (!app.show_system_settings || app.system_settings_tab != SystemTab::Apps) {
        return Task::none();
    }
    let endpoint = crate::app::gateway_client_endpoint();
    let Some(scope) = scope(app, &endpoint, false) else {
        return Task::none();
    };

    if let Event::Open(app_id, provider_id) = event {
        if app.app_providers.usage.settings.modal.as_ref().is_some_and(|modal| {
            modal.scope == scope && matches!(modal.phase, Phase::Editing { saving: true, .. })
        }) {
            return Task::none();
        }
        if !app.app_providers.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot
                .catalog
                .rows
                .iter()
                .any(|row| row.provider.app_id == app_id && row.provider.id == provider_id)
        }) {
            return Task::none();
        }
        let token = app.apps_catalog.token.trim().to_owned();
        let Ok(client) = GatewayClient::new(endpoint) else {
            return app.show_error_toast("无法连接额度设置网关");
        };
        if token.is_empty() {
            return app.show_error_toast("请先连接执行主机");
        }
        let editor = &mut app.app_providers.usage.settings;
        editor.sequence = editor.sequence.wrapping_add(1);
        let ticket = editor.sequence;
        let selection = UsageSnapshotQuery {
            project_id: scope.project.clone(),
            target_host_id: scope.host.clone(),
            app_id,
            provider_id,
            account_id: None,
            kind: None,
        };
        editor.modal =
            Some(Modal { scope, selection: selection.clone(), ticket, phase: Phase::Loading });
        return Task::perform(
            async move { client.apps_usage_settings(&selection, &token).await },
            move |result| send(Event::Loaded(ticket, result)),
        );
    }

    let editor = &mut app.app_providers.usage.settings;
    let Some(modal) = editor.modal.as_mut().filter(|modal| modal.scope == scope) else {
        return Task::none();
    };
    match event {
        Event::Close if !matches!(modal.phase, Phase::Editing { saving: true, .. }) => {
            editor.clear();
        }
        Event::Loaded(ticket, result)
            if ticket == modal.ticket && matches!(modal.phase, Phase::Loading) =>
        {
            match result {
                Ok(saved) => {
                    modal.phase = Phase::Editing { draft: Draft::new(saved), saving: false }
                }
                Err(error) => {
                    modal.phase = Phase::LoadFailed;
                    return app.show_error_toast(format!("读取额度设置失败 · {error}"));
                }
            }
        }
        Event::Saved(ticket, result) if ticket == modal.ticket => {
            if let Phase::Editing { saving, .. } = &mut modal.phase
                && *saving
            {
                *saving = false;
                return match result {
                    Ok(()) => {
                        editor.clear();
                        app.show_success_toast("额度设置已保存")
                    }
                    Err(error) => app.show_error_toast(format!("保存额度设置失败 · {error}")),
                };
            }
        }
        event => {
            if let Phase::Editing { draft, saving } = &mut modal.phase
                && !*saving
            {
                match event {
                    Event::Select(kind) => draft.select(kind),
                    Event::Edit(field, value) => draft.edit(field, value),
                    Event::Enabled(value) => draft.enabled = value,
                    Event::DisplayUsed(value) => {
                        draft.display =
                            if value { AppsUsageDisplay::Used } else { AppsUsageDisplay::Left };
                    }
                    Event::Save => {
                        let Ok(settings) = draft.settings() else { return Task::none() };
                        let token = app.apps_catalog.token.trim().to_owned();
                        let Ok(client) = GatewayClient::new(endpoint) else {
                            return app.show_error_toast("无法连接额度设置网关");
                        };
                        if token.is_empty() {
                            return app.show_error_toast("请先连接执行主机");
                        }
                        let selection = modal.selection.clone();
                        editor.sequence = editor.sequence.wrapping_add(1);
                        let ticket = editor.sequence;
                        modal.ticket = ticket;
                        *saving = true;
                        return Task::perform(
                            async move {
                                client.apps_usage_save_settings(&selection, &settings, &token).await
                            },
                            move |result| send(Event::Saved(ticket, result)),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    Task::none()
}
