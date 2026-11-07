use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::endpoints::{Event, message},
    state::app_providers::endpoints::Form,
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId, app_providers::endpoints::AppProviderEndpointKind,
};
fn action(label: &str, event: Event, enabled: bool) -> Element<'_, Message> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event)))
        .into()
}
pub(crate) fn overlay<'a>(
    form: &'a Form,
    app: AppId,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> Event| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe((!form.busy).then_some(move |v| message(event(v))))
    };
    let mut body = column![
        text("端点地址簿：保存地址，不切换当前端点；标记使用不代表请求成功。"),
        input("https://example.test/v1/messages", &form.url, Event::Url),
        checkbox(form.full_url)
            .label("完整请求 URL（保留路径及末尾斜杠）")
            .style(settings_checkbox_style)
            .on_toggle_maybe((!form.busy).then_some(|v| message(Event::FullUrl(v)))),
        action("添加端点", Event::Add, !form.busy),
    ]
    .spacing(10);
    for endpoint in &form.rows {
        body = body.push(settings_panel(
            column![
                text(&endpoint.url),
                text(format!(
                    "{} · 最后使用：{}",
                    if endpoint.kind == AppProviderEndpointKind::FullUrl {
                        "完整 URL"
                    } else {
                        "基础 URL"
                    },
                    endpoint.last_used.map(|v| v.to_string()).unwrap_or_else(|| "未使用".into())
                )),
                row![
                    action(
                        "标记使用",
                        Event::MarkUsed(endpoint.url.clone(), endpoint.kind),
                        !form.busy
                    ),
                    action("删除", Event::Remove(endpoint.url.clone(), endpoint.kind), !form.busy)
                ]
                .spacing(8)
            ]
            .spacing(8),
        ));
    }
    if matches!(app, AppId::Claude | AppId::Codex) {
        body = body
            .push(text("请求选项（只写）").size(18))
            .push(text("留空保留；输入 {} 清除对应组。刷新只回读配置状态，不回填敏感值。"));
        if let Some(view) = &form.options {
            body = body.push(text(format!(
                "服务状态：普通头{}，代理覆盖{}；未应用",
                if view.headers_configured { "已配置" } else { "未配置" },
                if view.overrides_configured { "已配置" } else { "未配置" }
            )));
        }
        body = body
            .push(text("普通请求头：供原生适配消费；此入口仅保存，尚未投影到原生配置。"))
            .push(input("{\"X-Title\":\"示例\"}", &form.headers, Event::Headers))
            .push(text("代理覆盖：仅适用于路由/聚合代理；生产代理未接入，保存不代表生效。"))
            .push(input(
                "{\"headers\":{},\"body\":{\"temperature\":0.2}}",
                &form.overrides,
                Event::Overrides,
            ));
    }
    if let Some(notice) = &form.notice {
        body = body.push(text(notice));
    }
    if form.busy {
        body = body.push(text("正在等待服务响应…"));
    }
    let mut footer =
        row![action("刷新", Event::Refresh, !form.busy), action("取消", Event::Cancel, !form.busy)]
            .spacing(10);
    if matches!(app, AppId::Claude | AppId::Codex) {
        footer = footer.push(
            button(text("保存请求选项"))
                .padding([8, 16])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (!form.busy && form.options.is_some()).then_some(message(Event::SaveOptions)),
                ),
        );
    }
    let card = settings_modal_card(
        column![
            row![
                text("端点与请求选项").size(22).width(Length::Fill),
                settings_close_button(message(Event::Cancel))
            ],
            scrollable(container(body).padding([0, 7]))
                .id("cc04-endpoints-scroll")
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            footer
        ]
        .spacing(16),
    )
    .width(Length::Fixed(760.))
    .height(Length::Fixed(650.));
    settings_modal_overlay(Some(base), message(Event::Cancel), card)
}
