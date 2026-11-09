use super::super::system_settings_common::*;
use crate::app::message::app_providers::{
    env::Event as EnvEvent,
    list::{ListMessage, message},
    mode::{self, Event as ModeEvent},
    paths::{self, Event as PathsEvent},
    preview::Event as PreviewEvent,
    snippets::Event as SnippetEvent,
    universal::Event as UniversalEvent,
};
use crate::app::{Message, i18n::I18n, state::app_providers::list::ListState};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{button, column, container, row, text, text_input},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::catalog::{ProviderMembership, ProviderSelection},
};
#[derive(Clone, Copy)]
pub(crate) struct Help<'a> {
    i18n: I18n,
    theme: &'a Theme,
    #[cfg(not(target_arch = "wasm32"))]
    usage_app: Option<&'a crate::app::App>,
}

impl<'a> Help<'a> {
    pub(crate) fn new(i18n: I18n, theme: &'a Theme) -> Self {
        Self {
            i18n,
            theme,
            #[cfg(not(target_arch = "wasm32"))]
            usage_app: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn with_usage(mut self, app: &'a crate::app::App) -> Self {
        self.usage_app = Some(app);
        self
    }

    #[cfg(test)]
    pub(super) fn test() -> Help<'static> {
        Help::new(I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese), &Theme::Light)
    }

    pub(super) fn field(
        self,
        title: &'static str,
        description: &'static str,
    ) -> Element<'static, Message> {
        let title = self.i18n.__(title);
        settings_hover_help_label(
            self.i18n,
            self.theme,
            title,
            settings_field_label(title),
            self.i18n.__(description),
        )
        .wrap()
        .into()
    }

    pub(super) fn heading(
        self,
        title: &'static str,
        description: &'static str,
        size: u32,
    ) -> Element<'static, Message> {
        let title = self.i18n.__(title);
        settings_hover_help_label(
            self.i18n,
            self.theme,
            title,
            text(title).size(size),
            self.i18n.__(description),
        )
        .wrap()
        .into()
    }

    pub(super) fn group(
        self,
        title: &'static str,
        description: &'static str,
    ) -> Element<'static, Message> {
        settings_group_heading_with_help(
            self.i18n,
            self.theme,
            self.i18n.__(title),
            self.i18n.__(description),
        )
    }
}

fn selection(value: &ProviderSelection) -> String {
    match value {
        ProviderSelection::Selected { id } => id.clone(),
        ProviderSelection::Unconfigured => "未配置".into(),
        ProviderSelection::NotApplicable => "不适用".into(),
        ProviderSelection::Unavailable { reason } => format!("不可读取：{reason:?}"),
    }
}
fn action(label: &str, event: ListMessage, enabled: bool) -> Element<'_, Message> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event)))
        .into()
}
fn edit_action<'a>(
    label: &'a str,
    id: &str,
    copy: bool,
    delete: bool,
    enabled: bool,
) -> Element<'a, Message> {
    button(text(label))
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| {
            super::super::super::message::app_providers::edit::message(
                super::super::super::message::app_providers::edit::EditMessage::Open {
                    id: id.into(),
                    copy,
                    delete,
                },
            )
        }))
        .into()
}
pub(crate) fn overlay<'a>(
    state: &'a ListState,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(scope) = &state.scope else {
        return base;
    };
    let mut body = column![].spacing(12).width(Length::Fill);
    if state.loading {
        body =
            body.push(settings_panel(text("正在从执行主机读取…").style(settings_muted_text_style)));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    if let Some(snapshot) = &state.snapshot {
        let mut status = column![
            settings_field_label("直连"),
            text(selection(&snapshot.catalog.direct)).size(13),
            settings_field_label("原生当前"),
            text(selection(&snapshot.catalog.in_use)).size(13),
        ]
        .spacing(6);
        if let Some(reason) = snapshot.catalog.native_read {
            status = status.push(
                text(format!("原生状态不可用：{reason:?}"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
        }
        body = body.push(settings_panel(status));
        let mut rows = snapshot.catalog.rows.iter().collect::<Vec<_>>();
        rows.sort_by_key(|r| {
            let p = state.presentation(&r.provider.id);
            (
                p.category.unwrap_or_default(),
                p.sort_index.unwrap_or(u32::MAX),
                r.provider.id.clone(),
            )
        });
        if rows.is_empty() {
            body = body
                .push(settings_panel(text("没有已保存的供应商").style(settings_muted_text_style)));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let usage_scope = help
            .usage_app
            .and_then(|app| super::super::apps_usage_cards::current_scope(app, false));
        for entry in rows {
            let p = state.presentation(&entry.provider.id);
            let member = if scope.app == AppId::Agy {
                "成员操作不适用"
            } else {
                match entry.native_member {
                    ProviderMembership::Present => "已添加",
                    ProviderMembership::Absent => "未添加",
                    ProviderMembership::Unknown => "成员状态未知",
                }
            };
            let enabled = state.form.is_none() && state.editor.is_none() && !state.loading;
            let id = &entry.provider.id;
            let mut actions = row![
                edit_action("编辑", id, false, false, enabled),
                edit_action("复制", id, true, false, enabled && scope.app != AppId::Agy),
                edit_action("删除", id, false, true, enabled),
            ]
            .spacing(8);
            actions = actions.push(action(
                "端点与请求选项",
                ListMessage::Endpoints(crate::app::message::app_providers::endpoints::Event::Open(
                    id.clone(),
                )),
                enabled,
            ));
            if crate::app::state::app_providers::preview::available(scope.app) {
                actions = actions.push(action(
                    "配置预览",
                    ListMessage::Preview(PreviewEvent::Open(id.clone())),
                    enabled,
                ));
            }
            if scope.app == AppId::ClaudeDesktop {
                actions = actions.push(action(
                    "直连与映射",
                    ListMessage::Desktop(crate::app::message::app_providers::desktop::Event::Open(
                        Some(id.clone()),
                    )),
                    enabled,
                ));
            }
            if scope.app == AppId::Hermes && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 Hermes" } else { "从 Hermes 移除" },
                    ListMessage::HermesMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Pi && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 Pi" } else { "从 Pi 移除" },
                    ListMessage::PiMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Kimi && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 Kimi" } else { "从 Kimi 移除" },
                    ListMessage::KimiMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Mcode && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 Mcode" } else { "从 Mcode 移除" },
                    ListMessage::McodeMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Dsh && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 Dsh" } else { "从 Dsh 移除" },
                    ListMessage::DshMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Omp && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 OMP" } else { "从 OMP 移除" },
                    ListMessage::OmpMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::OpenClaw && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 OpenClaw" } else { "从 OpenClaw 移除" },
                    ListMessage::OpenClawMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::OpenCode && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
                    if add { "添加到 OpenCode" } else { "从 OpenCode 移除" },
                    ListMessage::OpenCodeMember { id: id.clone(), add },
                    enabled,
                ));
            }
            actions = actions
                .push(action(
                    "图标",
                    ListMessage::Icons(crate::app::message::app_providers::icons::Event::Open(
                        id.clone(),
                    )),
                    enabled,
                ))
                .push(action("排序与备注", ListMessage::Edit(id.clone()), enabled))
                .push(action(
                    "模型目录",
                    ListMessage::ModelCatalog(
                        crate::app::message::settings::providers::catalog_api::Event::Open(
                            id.clone(),
                        ),
                    ),
                    enabled,
                ));
            let mut details = column![
                text(&entry.provider.name).size(17),
                row![
                    settings_value_badge(if entry.provider.configured {
                        "凭据已配置"
                    } else {
                        "凭据未配置"
                    }),
                    text(format!("{} · {member}", p.category.unwrap_or_else(|| "未分类".into())))
                        .size(13)
                        .style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
            ]
            .spacing(8);
            if let Some(notes) = p.notes.filter(|notes| !notes.is_empty()) {
                details = details.push(text(notes).size(13).style(settings_muted_text_style));
            }
            #[cfg(not(target_arch = "wasm32"))]
            if help.usage_app.is_some() {
                details = details.push(super::super::apps_usage_cards::provider_view(
                    usage_scope.as_ref(),
                    &state.usage,
                    scope.app,
                    id,
                ));
            }
            body =
                body.push(settings_panel(details.push(actions.align_y(Alignment::Center).wrap())));
        }
    }

    let close = message(ListMessage::Close);
    let header = settings_modal_header(
        text(format!("应用供应商 · {:?}", scope.app)).size(20),
        settings_close_button(close.clone()),
    );
    let ready = !state.loading && state.form.is_none() && state.editor.is_none();
    let create = if scope.app == AppId::ClaudeDesktop {
        super::super::super::message::app_providers::desktop::message(
            super::super::super::message::app_providers::desktop::Event::Open(None),
        )
    } else {
        super::super::super::message::app_providers::edit::message(
            super::super::super::message::app_providers::edit::EditMessage::Create,
        )
    };
    let mut actions = row![
        button(text("新增"))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(ready.then_some(create)),
        action("刷新", ListMessage::Refresh, ready),
        action("统一供应商投影", ListMessage::Universal(UniversalEvent::Open), ready),
        action(
            "公共配置片段",
            ListMessage::Snippets(SnippetEvent::Open),
            ready && crate::app::state::app_providers::snippets::available(scope.app),
        ),
    ];
    if crate::app::state::app_providers::env::available(scope.app) {
        actions = actions.push(action("环境诊断", ListMessage::Env(EnvEvent::Open), ready));
    }
    if mode::available(scope.app) {
        actions = actions.push(action("供应商模式", ListMessage::Mode(ModeEvent::Open), ready));
    }
    if paths::available(scope.app) {
        actions = actions.push(action("配置目录", ListMessage::Paths(PathsEvent::Open), ready));
    }
    let actions = actions
        .push(action("关闭", ListMessage::Close, true))
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap();
    let card = settings_modal_layout(header, body, Some(actions.into())).width(800).height(650);
    let list = settings_modal_overlay(Some(base), close, container(card).padding(12));
    if let Some(form) = &state.model_catalog {
        return crate::app::components::system_settings_providers::catalog::deployments(
            form,
            state.generation,
            help.i18n,
            help.theme,
            list,
        );
    }
    if let Some(form) = &state.paths {
        return super::paths::overlay(form, help, list);
    }
    if let Some(form) = &state.mode {
        return super::mode::overlay(state, form, help, list);
    }
    if let Some(form) = &state.env {
        return super::env::overlay(form, help, list);
    }
    if let Some(form) = &state.preview {
        return super::preview::overlay(form, help, list);
    }
    if let Some(form) = &state.snippets {
        return super::snippets::overlay(state, form, scope.app, help, list);
    }
    if let Some(form) = &state.universal {
        return super::universal::overlay(form, help, list);
    }
    if let Some(form) = &state.icons {
        return super::icons::overlay(form, list);
    }
    if let Some(form) = &state.endpoints {
        return super::endpoints::overlay(form, scope.app, list);
    }
    if let Some(form) = &state.desktop {
        return super::desktop::overlay(form, help, list);
    }
    if let Some(form) = &state.form {
        #[cfg(not(target_arch = "wasm32"))]
        let binding =
            crate::app::message::agent_apps_auth::binding::entry(form, scope.app, state.generation);
        #[cfg(target_arch = "wasm32")]
        let binding = None;
        return super::edit::overlay_with_binding(form, help, list, binding);
    }
    let Some(form) = &state.editor else {
        return list;
    };
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> ListMessage| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!form.saving).then_some(move |v| message(event(v))))
    };
    let fields = settings_panel(
        column![
            settings_field_label("排序"),
            settings_control_with_hint(
                input("排序", &form.sort, ListMessage::Sort),
                "非负整数，留空清除",
            ),
            settings_field_label("分类"),
            input("分类", &form.category, ListMessage::Category),
            settings_field_label("备注"),
            input("备注", &form.notes, ListMessage::Notes),
        ]
        .spacing(8),
    );
    let cancel = message(ListMessage::Cancel);
    let header = settings_modal_header(
        text("排序、分类与备注").size(20),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        action("取消", ListMessage::Cancel, !form.saving),
        button(text(if form.saving { "保存中…" } else { "保存" }))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe((!form.saving).then_some(message(ListMessage::Save))),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(actions.into())).width(560).height(480);
    settings_modal_overlay(Some(list), cancel, container(card).padding(12))
}
