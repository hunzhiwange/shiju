//! Transient script trials for the provider usage settings modal.

use std::fmt;

use iced::{
    Task,
    task::Handle,
    widget::text_editor::{Action, Content},
};
use vw_gateway_client::{
    AppsUsageCredentialRefs, AppsUsageScriptTrial, AppsUsageTemplate, AppsUsageWriteError,
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps_auth::AuthErrorCode,
        apps_usage::{
            UsageQueryErrorCode, UsageScriptPackage, UsageSnapshotQuery,
            settings::UsageCredentialId,
        },
    },
};

use super::apps_usage_settings::{self, Event as SettingsEvent};
use crate::app::Message;

#[cfg(test)]
#[path = "apps_usage_script_tests.rs"]
mod tests;
mod view;
pub(super) use view::modal_view;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Custom,
    General,
    Sub2Api,
}

impl Kind {
    const ALL: [Self; 3] = [Self::Custom, Self::General, Self::Sub2Api];

    fn id(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::General => "general",
            Self::Sub2Api => "sub2api",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Custom => "自定义",
            Self::General => "通用余额",
            Self::Sub2Api => "Sub2API",
        })
    }
}

#[derive(Clone)]
pub(super) struct Draft {
    templates: Vec<AppsUsageTemplate>,
    selected: Option<Kind>,
    pub(super) source: Content,
    preview: Content,
    pub(super) timeout: String,
    pub(super) key_ref: String,
}

impl Draft {
    pub(super) fn new(templates: Vec<AppsUsageTemplate>) -> Self {
        let mut draft = Self {
            source: Content::with_text(
                templates
                    .iter()
                    .find(|template| template.template == Kind::Custom.id())
                    .and_then(|template| template.source.as_deref())
                    .unwrap_or_default(),
            ),
            templates,
            selected: None,
            preview: Content::new(),
            timeout: "10".into(),
            key_ref: String::new(),
        };
        let first = draft.kinds().next();
        if let Some(kind) = first {
            draft.select(kind);
        }
        draft
    }

    fn preset(&self, kind: Kind) -> Option<&str> {
        self.templates.iter().find(|template| template.template == kind.id())?.source.as_deref()
    }

    fn kinds(&self) -> impl Iterator<Item = Kind> + '_ {
        Kind::ALL.into_iter().filter(|kind| self.preset(*kind).is_some())
    }

    pub(super) fn select(&mut self, kind: Kind) {
        if self.selected != Some(kind)
            && let Some(source) = self.preset(kind)
        {
            if kind != Kind::Custom {
                self.preview = Content::with_text(source);
            }
            self.selected = Some(kind);
        }
    }

    pub(super) fn request(
        &self,
        selection: &UsageSnapshotQuery,
    ) -> Result<AppsUsageScriptTrial, &'static str> {
        let kind = self.selected.ok_or("执行主机未提供可试运行的脚本模板")?;
        let timeout = self
            .timeout
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|value| (2..=30).contains(value))
            .ok_or("超时须为 2–30 秒的整数")?;
        let mut credentials = AppsUsageCredentialRefs::default();
        let source = if kind == Kind::Custom {
            let source = self.source.text();
            if source.trim().is_empty() || source.len() > 64 * 1024 {
                return Err("脚本须包含内容且不超过 64 KiB");
            }
            Some(source)
        } else {
            let key_ref = UsageCredentialId::try_from(self.key_ref.trim().to_owned())
                .map_err(|_| "请填写管理员提供的 API Key 引用")?;
            credentials.api_key_ref = Some(key_ref.into());
            None
        };
        Ok(AppsUsageScriptTrial {
            selection: selection.clone(),
            template: kind.id().into(),
            source,
            timeout_seconds: Some(timeout),
            credentials,
        })
    }
}

#[derive(Clone)]
pub enum Event {
    Loaded(u64, Option<Vec<AppsUsageTemplate>>),
    Select(Kind),
    Edit(Action),
    Timeout(String),
    KeyRef(String),
    Run,
    Cancel,
    Finished(u64, TrialResult),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidInput,
    Denied,
    Changed,
    Unsupported,
    Timeout,
    Credential,
    InvalidResponse,
    Unavailable,
}

impl Failure {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "脚本试运行输入无效，请检查脚本和参数",
            Self::Denied => "脚本试运行权限不足，或目标地址未获授权",
            Self::Changed => "供应商或试运行授权已变化，请重新打开设置",
            Self::Unsupported => "执行主机不支持此模板的试运行",
            Self::Timeout => "脚本试运行超时",
            Self::Credential => "试运行所需凭据不可用或已失效",
            Self::InvalidResponse => "脚本未返回有效的套餐结果",
            Self::Unavailable => "脚本试运行暂不可用，请重试",
        }
    }

    pub(super) fn from_gateway(error: AppsUsageWriteError) -> Self {
        use AppsUsageWriteError::*;
        match error {
            Http { status, error } => match (status, error.map(|error| error.code)) {
                (_, Some(AuthErrorCode::InvalidInput)) | (400, _) => Self::InvalidInput,
                (_, Some(AuthErrorCode::ReauthRequired)) => Self::Credential,
                (401 | 403, _) => Self::Denied,
                (404 | 409, _) => Self::Changed,
                (_, Some(AuthErrorCode::Unsupported)) | (501, _) => Self::Unsupported,
                _ => Self::Unavailable,
            },
            ScriptTrial { code, .. } => match code {
                UsageQueryErrorCode::Timeout => Self::Timeout,
                UsageQueryErrorCode::CredentialUnavailable
                | UsageQueryErrorCode::UpstreamUnauthorized => Self::Credential,
                UsageQueryErrorCode::Unsupported => Self::Unsupported,
                UsageQueryErrorCode::InvalidResponse => Self::InvalidResponse,
                _ => Self::Unavailable,
            },
            Decode => Self::InvalidResponse,
            InvalidPrincipalToken => Self::Denied,
            InvalidOperationId | Transport => Self::Unavailable,
        }
    }
}

type TrialResult = Result<Vec<UsageScriptPackage>, Failure>;

pub(super) async fn run_trial(
    client: GatewayClient,
    request: AppsUsageScriptTrial,
    token: String,
) -> TrialResult {
    let result =
        client.apps_usage_script_test(&request, &token).await.map_err(Failure::from_gateway)?;
    if result.is_empty() { Err(Failure::InvalidResponse) } else { Ok(result) }
}

fn send(event: Event) -> Message {
    apps_usage_settings::send(SettingsEvent::Script(event))
}

#[derive(Clone)]
pub(super) enum Editor {
    Loading(u64),
    LoadFailed,
    Ready(Box<Ready>),
}

impl fmt::Debug for Editor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UsageScriptEditor")
    }
}

#[derive(Clone)]
pub(super) struct Ready {
    draft: Draft,
    pending: Option<(u64, Handle)>,
    packages: Vec<UsageScriptPackage>,
}

impl Ready {
    fn cancel(&mut self) {
        if let Some((_, handle)) = self.pending.take() {
            handle.abort();
        }
    }
}

impl Editor {
    pub(super) fn load(
        client: GatewayClient,
        selection: UsageSnapshotQuery,
        token: String,
        ticket: u64,
    ) -> Task<Message> {
        Task::perform(
            async move { client.apps_usage_templates(&selection, &token).await.ok() },
            move |result| send(Event::Loaded(ticket, result)),
        )
    }

    pub(super) fn cancel(&mut self) {
        if let Self::Ready(ready) = self {
            ready.cancel();
        }
    }

    pub(super) fn update(
        &mut self,
        event: Event,
        selection: &UsageSnapshotQuery,
        endpoint: GatewayEndpoint,
        token: &str,
        sequence: &mut u64,
    ) -> Result<Task<Message>, &'static str> {
        if let Event::Loaded(ticket, result) = event {
            if matches!(self, Self::Loading(expected) if *expected == ticket) {
                *self = if let Some(templates) = result {
                    Self::Ready(Box::new(Ready {
                        draft: Draft::new(templates),
                        pending: None,
                        packages: Vec::new(),
                    }))
                } else {
                    Self::LoadFailed
                };
            }
            return Ok(Task::none());
        }

        let Self::Ready(ready) = self else { return Ok(Task::none()) };
        match event {
            Event::Cancel => ready.cancel(),
            Event::Finished(ticket, result)
                if ready.pending.as_ref().is_some_and(|(expected, _)| *expected == ticket) =>
            {
                ready.pending = None;
                ready.packages = result.map_err(Failure::label)?;
            }
            event if ready.pending.is_none() => {
                match event {
                    Event::Select(kind) => ready.draft.select(kind),
                    Event::Edit(action) if ready.draft.selected == Some(Kind::Custom) => {
                        let edited = action.is_edit();
                        ready.draft.source.perform(action);
                        if !edited {
                            return Ok(Task::none());
                        }
                    }
                    Event::Timeout(value) => ready.draft.timeout = value,
                    Event::KeyRef(value) => ready.draft.key_ref = value,
                    Event::Run => {
                        let trial = ready.draft.request(selection)?;
                        if token.trim().is_empty() {
                            return Err("请先连接执行主机");
                        }
                        let client = GatewayClient::new(endpoint)
                            .map_err(|_| Failure::Unavailable.label())?;
                        ready.packages.clear();
                        *sequence = sequence.wrapping_add(1);
                        let ticket = *sequence;
                        let (task, handle) = Task::perform(
                            run_trial(client, trial, token.trim().to_owned()),
                            move |result| send(Event::Finished(ticket, result)),
                        )
                        .abortable();
                        ready.pending = Some((ticket, handle.abort_on_drop()));
                        return Ok(task);
                    }
                    _ => return Ok(Task::none()),
                }
                ready.packages.clear();
            }
            _ => {}
        }
        Ok(Task::none())
    }
}
