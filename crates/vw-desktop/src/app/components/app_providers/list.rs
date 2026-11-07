use super::super::system_settings_common::*;
use crate::app::message::app_providers::list::{ListMessage, message};
use crate::app::{Message, state::app_providers::list::ListState};
use iced::{
    Element, Length,
    widget::{button, column, container, row, scrollable, text, text_input},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::catalog::{ProviderMembership, ProviderSelection},
};
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
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(scope) = &state.scope else {
        return base;
    };
    let mut body = column![].spacing(12);
    if state.loading {
        body = body.push(text("正在从执行主机读取…"));
    }
    if let Some(error) = &state.error {
        body = body.push(text(error));
    }
    if let Some(snapshot) = &state.snapshot {
        body = body.push(text(format!(
            "直连：{}　原生当前：{}",
            selection(&snapshot.catalog.direct),
            selection(&snapshot.catalog.in_use)
        )));
        if let Some(reason) = snapshot.catalog.native_read {
            body = body.push(text(format!("原生状态不可用：{reason:?}")));
        }
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
            body = body.push(text("没有已保存的供应商"));
        }
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
            body = body.push(settings_panel(
                column![
                    row![
                        column![
                            text(&entry.provider.name).size(18),
                            text(if entry.provider.configured {
                                "凭据已配置"
                            } else {
                                "凭据未配置"
                            })
                            .size(12),
                            text(format!(
                                "{} · {}",
                                p.category.unwrap_or_else(|| "未分类".into()),
                                member
                            ))
                        ]
                        .width(Length::Fill),
                        action(
                            "图标",
                            ListMessage::Icons(
                                crate::app::message::app_providers::icons::Event::Open(id.clone())
                            ),
                            enabled,
                        ),
                        action(
                            "排序与备注",
                            ListMessage::Edit(entry.provider.id.clone()),
                            state.editor.is_none() && state.form.is_none() && !state.loading
                        )
                    ]
                    .spacing(12),
                    text(p.notes.unwrap_or_default()),
                    actions
                ]
                .spacing(8),
            ));
        }
    }
    let content = scrollable(container(body).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill);
    let card = settings_modal_card(
        column![
            row![
                text(format!("应用供应商 · {:?}", scope.app)).size(22).width(Length::Fill),
                settings_close_button(message(ListMessage::Close))
            ]
            .spacing(12),
            content,
            row![
                button(text("新增"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        (!state.loading && state.form.is_none() && state.editor.is_none())
                            .then_some(if scope.app == AppId::ClaudeDesktop {
                                crate::app::message::app_providers::desktop::message(
                                    crate::app::message::app_providers::desktop::Event::Open(None),
                                )
                            } else {
                                crate::app::message::app_providers::edit::message(
                                    crate::app::message::app_providers::edit::EditMessage::Create,
                                )
                            })
                    ),
                action(
                    "刷新",
                    ListMessage::Refresh,
                    !state.loading && state.editor.is_none() && state.form.is_none()
                ),
                action("关闭", ListMessage::Close, true)
            ]
            .spacing(10)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(800.))
    .height(Length::Fixed(650.));
    let list = settings_modal_overlay(Some(base), message(ListMessage::Close), card);
    if let Some(form) = &state.icons {
        return super::icons::overlay(form, list);
    }
    if let Some(form) = &state.endpoints {
        return super::endpoints::overlay(form, scope.app, list);
    }
    if let Some(form) = &state.desktop {
        return super::desktop::overlay(form, list);
    }
    if let Some(form) = &state.form {
        return super::edit::overlay(form, list);
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
    let fields = column![
        text("排序（非负整数，留空清除）"),
        input("排序", &form.sort, ListMessage::Sort),
        text("分类"),
        input("分类", &form.category, ListMessage::Category),
        text("备注"),
        input("备注", &form.notes, ListMessage::Notes)
    ]
    .spacing(10);
    let card = settings_modal_card(
        column![
            row![
                text("排序、分类与备注").size(22).width(Length::Fill),
                settings_close_button(message(ListMessage::Cancel))
            ],
            scrollable(container(fields).padding([0, 7]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            row![
                action("取消", ListMessage::Cancel, !form.saving),
                button(text(if form.saving { "保存中…" } else { "保存" }))
                    .padding([8, 16])
                    .style(primary_action_btn_style)
                    .on_press_maybe((!form.saving).then_some(message(ListMessage::Save)))
            ]
            .spacing(12)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(560.))
    .height(Length::Fixed(480.));
    settings_modal_overlay(Some(list), message(ListMessage::Cancel), card)
}
