use desktop_ui_controls::system_settings_common::*;
use crate::{Scope, state::list::Snapshot};
use iced::{Alignment,Element,Length,widget::{button,column,container,row,text,text_input}};
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::{ProviderMembership,ProviderSelection};
#[derive(Clone)]
pub enum ListMessage {
 Create, OpenEdit { id: String, copy: bool, delete: bool },
 Close, Refresh, QueryInput(String), Edit(String),
 Endpoints(String), ModelDiagnostics(String), FailoverHealth(String), Preview(String), DesktopAggregation(String), Desktop(Option<String>), Icons(String), ModelCatalog(String),
 Universal, Snippets, Env, Aggregation, Failover, Mode, Paths, Omo(bool), CodexSubagent, PiDirectory, OmpDirectory,
 HermesMember { id:String,add:bool }, PiMember { id:String,add:bool }, KimiMember { id:String,add:bool }, McodeMember { id:String,add:bool }, DshMember { id:String,add:bool }, OmpMember { id:String,add:bool }, OpenClawMember { id:String,add:bool }, OpenCodeMember { id:String,add:bool },
}

pub struct View<'a> {
 pub scope: Option<&'a Scope>, pub query: &'a str, pub loading: bool, pub error: Option<&'a String>, pub snapshot: Option<&'a Snapshot>, pub editing: bool,
}
fn selection(value: &ProviderSelection) -> String {
    match value {
        ProviderSelection::Selected { id } => id.clone(),
        ProviderSelection::Unconfigured => "未配置".into(),
        ProviderSelection::NotApplicable => "不适用".into(),
        ProviderSelection::Unavailable { reason } => format!("不可读取：{reason:?}"),
    }
}
fn action<M: Clone + 'static>(message: fn(ListMessage)->M, label: &str, event: ListMessage, enabled: bool) -> Element<'_, M> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event)))
        .into()
}
fn edit_action<'a, M: Clone + 'static>(
    message: fn(ListMessage)->M,
    label: &'a str,
    id: &str,
    copy: bool,
    delete: bool,
    enabled: bool,
) -> Element<'a, M> {
    button(text(label))
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| {
            message(ListMessage::OpenEdit { id: id.into(), copy, delete })
        }))
        .into()
}
pub fn card<'a, M: Clone + 'static>(
    message: fn(ListMessage)->M,
    state: View<'a>,
    mut supplements: Vec<(&'a str, Option<Element<'a, M>>, Element<'a, M>)>,
) -> Element<'a, M> {
    let Some(scope) = state.scope else {
        return iced::widget::Space::new().into();
    };
    let search = settings_text_input("搜索供应商名称或标识…", state.query)
        .on_input(move |query| message(ListMessage::QueryInput(query)))
        .width(Length::Fill);
    let mut body = column![
        row![search, action(message, "清空搜索", ListMessage::QueryInput(String::new()), !state.query.is_empty())]
            .spacing(10)
            .align_y(Alignment::Center),
    ]
    .spacing(12)
    .width(Length::Fill);
    if state.loading {
        body =
            body.push(settings_panel(text("正在从执行主机读取…").style(settings_muted_text_style)));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    if let Some(snapshot) = state.snapshot {
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
        let query = state.query.trim().to_lowercase();
        let rows = snapshot.catalog.rows.iter().filter(|row| {
            row.provider.app_id == scope.app
                && (query.is_empty()
                    || row.provider.name.to_lowercase().contains(&query)
                    || row.provider.id.to_lowercase().contains(&query))
        }).collect::<Vec<_>>();
        if rows.is_empty() {
            body = body
                .push(settings_panel(text(if query.is_empty() { "没有已保存的供应商" } else { "没有匹配的供应商，请修改或清空搜索" }).style(settings_muted_text_style)));
        }
        for entry in rows {
            let p = state.snapshot.and_then(|s| s.presentation.iter().find(|p| p.id == entry.provider.id)).map(|p| p.presentation.clone()).unwrap_or_default();
            let member = if scope.app == AppId::Agy {
                "成员操作不适用"
            } else {
                match entry.native_member {
                    ProviderMembership::Present => "已添加",
                    ProviderMembership::Absent => "未添加",
                    ProviderMembership::Unknown => "成员状态未知",
                }
            };
            let enabled = !state.editing && !state.loading;
            let id = &entry.provider.id;
            let mut actions = row![
                edit_action(message, "编辑", id, false, false, enabled),
                edit_action(message, "复制", id, true, false, enabled && scope.app != AppId::Agy),
                edit_action(message, "删除", id, false, true, enabled),
            ]
            .spacing(8);
            actions = actions.push(action(
        message,
                "端点与请求选项",
                ListMessage::Endpoints(id.clone()),
                enabled,
            ));
            if matches!(scope.app, AppId::Claude | AppId::Codex | AppId::Gemini) {
                actions = actions.push(action(
        message,
                    "模型诊断",
                    ListMessage::ModelDiagnostics(id.clone()),
                    enabled,
                ));
            }
            if matches!(scope.app, AppId::Claude | AppId::Codex | AppId::Gemini) {
                actions = actions.push(action(message, "健康与实际路由", ListMessage::FailoverHealth(id.clone()), enabled));
            }
            if crate::state::preview::available(scope.app) {
                actions = actions.push(action(
        message,
                    "配置预览",
                    ListMessage::Preview(id.clone()),
                    enabled,
                ));
            }
            if scope.app == AppId::ClaudeDesktop {
                actions = actions.push(action(
        message,
                    "聚合映射与应用",
                    ListMessage::DesktopAggregation(id.clone()),
                    enabled,
                ));
                actions = actions.push(action(
        message,
                    "直连与映射",
                    ListMessage::Desktop(Some(id.clone())),
                    enabled,
                ));
            }
            if scope.app == AppId::Hermes && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 Hermes" } else { "从 Hermes 移除" },
                    ListMessage::HermesMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Pi && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 Pi" } else { "从 Pi 移除" },
                    ListMessage::PiMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Kimi && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 Kimi" } else { "从 Kimi 移除" },
                    ListMessage::KimiMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Mcode && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 Mcode" } else { "从 Mcode 移除" },
                    ListMessage::McodeMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Dsh && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 Dsh" } else { "从 Dsh 移除" },
                    ListMessage::DshMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::Omp && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 OMP" } else { "从 OMP 移除" },
                    ListMessage::OmpMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::OpenClaw && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 OpenClaw" } else { "从 OpenClaw 移除" },
                    ListMessage::OpenClawMember { id: id.clone(), add },
                    enabled,
                ));
            }
            if scope.app == AppId::OpenCode && entry.native_member != ProviderMembership::Unknown {
                let add = entry.native_member == ProviderMembership::Absent;
                actions = actions.push(action(
        message,
                    if add { "添加到 OpenCode" } else { "从 OpenCode 移除" },
                    ListMessage::OpenCodeMember { id: id.clone(), add },
                    enabled,
                ));
            }
            actions = actions
                .push(action(
        message,
                    "图标",
                    ListMessage::Icons(id.clone()),
                    enabled,
                ))
                .push(action(message, "排序与备注", ListMessage::Edit(id.clone()), enabled))
                .push(action(
        message,
                    "模型目录",
                    ListMessage::ModelCatalog(id.clone()),
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
            let extra = supplements.iter().position(|(key, _, _)| *key == id)
                .map(|index| supplements.remove(index));
            let usage = if let Some((_, health, usage)) = extra {
                if let Some(health) = health { details = details.push(health); }
                Some(usage)
            } else { None };
            if let Some(notes) = p.notes.filter(|notes| !notes.is_empty()) {
                details = details.push(text(notes).size(13).style(settings_muted_text_style));
            }
            if let Some(usage) = usage { details = details.push(usage); }
            body =
                body.push(settings_panel(details.push(actions.align_y(Alignment::Center).wrap())));
        }
    }

    let close = message(ListMessage::Close);
    let header = settings_modal_header(
        text(format!("应用供应商 · {:?}", scope.app)).size(20),
        settings_close_button(close.clone()),
    );
    let ready = !state.loading && !state.editing;
    let create = message(if scope.app == AppId::ClaudeDesktop { ListMessage::Desktop(None) } else { ListMessage::Create });
    let mut actions = row![
        button(text("新增"))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(ready.then_some(create)),
        action(message, "刷新", ListMessage::Refresh, ready),
        action(message, "统一供应商投影", ListMessage::Universal, ready),
        action(
        message,
            "公共配置片段",
            ListMessage::Snippets,
            ready && crate::state::snippets::available(scope.app),
        ),
    ];
    if crate::state::env::available(scope.app) {
        actions = actions.push(action(message, "环境诊断", ListMessage::Env, ready));
    }
    actions = actions.push(action(
        message,
        "聚合目录",
        ListMessage::Aggregation,
        ready,
    ));
    actions = actions.push(action(message, "备用队列", ListMessage::Failover, ready));
    if cfg!(target_os = "macos") && scope.app == AppId::Claude {
        actions = actions.push(action(message, "供应商模式", ListMessage::Mode, ready));
    }
    if cfg!(target_os = "macos") && scope.app == AppId::Claude {
        actions = actions.push(action(message, "配置目录", ListMessage::Paths, ready));
    }
    if scope.app == AppId::OpenCode {
        actions = actions
            .push(action(message, "OMO 标准目录", ListMessage::Omo(false), ready))
            .push(action(message, "OMO Slim 目录", ListMessage::Omo(true), ready));
    }
    if scope.app == AppId::Codex {
        actions = actions.push(action(
        message,
            "Codex 子代理设置",
            ListMessage::CodexSubagent,
            ready,
        ));
    }
    if scope.app == AppId::Pi {
        actions = actions.push(action(
        message,
            "Pi 模型条目",
            ListMessage::PiDirectory,
            ready,
        ));
    }
    if scope.app == AppId::Omp {
        actions = actions.push(action(
        message,
            "OMP 独立模型条目",
            ListMessage::OmpDirectory,
            ready,
        ));
    }
    let actions = actions
        .push(action(message, "关闭", ListMessage::Close, true))
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap();
    let card = settings_modal_layout(header, body, Some(actions.into())).width(800).height(650);
    container(card).padding(12).into()
}

#[derive(Clone)]
pub enum EditorMessage { Sort(String), Category(String), Notes(String), Cancel, Save }
fn message(event: EditorMessage) -> crate::Message { crate::Message::ListEditor(event) }
fn editor_action(label: &str, event: EditorMessage, enabled: bool) -> Element<'_, crate::Message> {
 button(text(label)).padding([8,14]).style(rounded_action_btn_style).on_press_maybe(enabled.then_some(message(event))).into()
}
pub fn editor_card<'a>(sort: &'a str, category: &'a str, notes: &'a str, saving: bool) -> Element<'a, crate::Message> {
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> EditorMessage| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!saving).then_some(move |v| message(event(v))))
    };
    let fields = settings_panel(
        column![
            settings_field_label("排序"),
            settings_control_with_hint(
                input("排序", sort, EditorMessage::Sort),
                "非负整数，留空清除",
            ),
            settings_field_label("分类"),
            input("分类", category, EditorMessage::Category),
            settings_field_label("备注"),
            input("备注", notes, EditorMessage::Notes),
        ]
        .spacing(8),
    );
    let cancel = message(EditorMessage::Cancel);
    let header = settings_modal_header(
        text("排序、分类与备注").size(20),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        editor_action("取消", EditorMessage::Cancel, !saving),
        button(text(if saving { "保存中…" } else { "保存" }))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe((!saving).then_some(message(EditorMessage::Save))),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, fields, Some(actions.into())).width(560).height(480);
    container(card).padding(12).into()
}
