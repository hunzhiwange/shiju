use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::desktop::{Event, message},
    state::app_providers::desktop::Form,
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use vw_gateway_client::vw_api_types::app_providers::desktop_mapping::DesktopMappingMode;
pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let active = !form.busy;
    let input = |label, value, event: fn(String) -> Event| {
        text_input(label, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(active.then_some(move |v| message(event(v))))
    };
    let action = |label: &'static str, event: Event, enabled: bool| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![
        text("目录保存与主机应用分别执行。应用成功后，请完全退出并重新启动 Claude Desktop。")
            .size(13),
        text("路由运行时及实时配置诊断尚不可用；不能由保存成功推断模型已可用。").size(13)
    ]
    .spacing(12);
    if let Some(notice) = &form.notice {
        body = body.push(text(notice).size(14));
    }
    if form.busy {
        body = body.push(text("正在请求执行主机…"));
    }
    if form.create {
        body = body.push(text("供应商标识")).push(input("新的唯一标识", &form.id, Event::Id));
    }
    if form.direct_editable {
        body = body
            .push(text("直连配置").size(18))
            .push(text("名称"))
            .push(input("供应商名称", &form.name, Event::Name))
            .push(
                checkbox(form.official)
                    .label("使用 Desktop 已有官方登录")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(active.then_some(|v| message(Event::Official(v)))),
            );
        if !form.official {
            body = body
                .push(text("API 地址"))
                .push(input("https://…", &form.base_url, Event::BaseUrl))
                .push(text("完整密钥（保存直连配置时需重新输入；读取不会返回密钥）").size(13))
                .push(input("输入完整密钥", &form.key, Event::Key).secure(true));
        }
        body = body
            .push(
                checkbox(form.apply)
                    .label(if form.create {
                        "仅在尚无直连供应商时同时激活"
                    } else {
                        "若当前正在使用此档案，同时应用"
                    })
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(active.then_some(|v| message(Event::Apply(v)))),
            )
            .push(
                text("保存后关闭并刷新，再打开可编辑模型映射。官方授权与模型发现暂不可用。")
                    .size(12),
            );
        if !form.create {
            body = body.push(action("应用已保存的直连配置", Event::Activate, active));
        }
    }
    if form.mapping_editable {
        body=body.push(settings_divider()).push(text("模型路由").size(18))
            .push(checkbox(form.mapping.mode==DesktopMappingMode::Proxy).label("模型映射模式（仅保存目录，不启动代理）").style(settings_checkbox_style).on_toggle_maybe(active.then_some(|v|message(Event::Proxy(v)))))
            .push(text("直连要求路由与请求模型相同；映射模式需兼容上游及路由运行时。服务端校验失败会保留草稿。").size(13))
            .push(action("载入默认路由",Event::Defaults,active))
            .push(text("添加模型路由（由执行端校验标识兼容性）").size(13))
            .push(input("claude-sonnet-…", &form.new_route, Event::NewRoute))
            .push(action("添加模型", Event::AddRoute, active && !form.new_route.is_empty()));
        for (id, route) in &form.mapping.routes {
            let control = |label, value, event: fn(String, String) -> Event| {
                let id = id.clone();
                text_input(label, value)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input_maybe(active.then_some(move |v| message(event(id.clone(), v))))
            };
            let one_id = id.clone();
            body = body.push(settings_panel(
                column![
                    text(id),
                    text("实际请求模型").size(13),
                    control("请求模型", &route.model, Event::Model),
                    text("菜单显示名").size(13),
                    control(
                        "可留空",
                        route.label_override.as_deref().unwrap_or_default(),
                        Event::Label
                    ),
                    row![
                        checkbox(route.supports_1m)
                            .label("声明支持 1M")
                            .style(settings_checkbox_style)
                            .on_toggle_maybe(
                                active.then_some(move |v| message(Event::OneM(one_id.clone(), v)))
                            ),
                        action("移除", Event::Remove(id.clone()), active)
                    ]
                    .spacing(12)
                ]
                .spacing(8),
            ));
        }
    } else if !form.create && !form.direct_editable {
        body = body.push(text("此档案使用不支持的凭据或配置形态，无法在此表单修改。"));
    }
    let content = scrollable(container(body).padding([0, 7]))
        .id(iced::widget::Id::new("desktop-provider-form"))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill);
    let card = settings_modal_card(
        column![
            row![
                text("Claude Desktop · 直连与映射").size(22).width(Length::Fill),
                settings_close_button(message(Event::Cancel))
            ]
            .spacing(12),
            content,
            row![
                action("从 Claude 导入兼容档案", Event::Import, active),
                button(text("保存直连"))
                    .padding([8, 14])
                    .style(primary_action_btn_style)
                    .on_press_maybe(
                        (active
                            && form.direct_editable
                            && !form.id.is_empty()
                            && !form.name.is_empty())
                        .then_some(message(Event::SaveDirect))
                    ),
                button(text("保存映射"))
                    .padding([8, 14])
                    .style(primary_action_btn_style)
                    .on_press_maybe(
                        (active && form.mapping_editable).then_some(message(Event::SaveMapping))
                    ),
                action("取消 / 关闭", Event::Cancel, active)
            ]
            .spacing(10)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(800.))
    .height(Length::Fixed(680.));
    settings_modal_overlay(Some(base), message(Event::Cancel), card)
}
