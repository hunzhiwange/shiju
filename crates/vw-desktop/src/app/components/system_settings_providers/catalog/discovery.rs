//! Discovery controls and results inside the saved deployment dialog.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
        settings_error_banner, settings_field_label, settings_group_card,
        settings_hover_help_label, settings_muted_text_style,
    },
    i18n::I18n,
    message::settings::providers::{
        catalog_api::{self, Event as CatalogEvent},
        discovery::{self, Event},
    },
    state::app_providers::model_catalog::{CatalogForm, DiscoveryPhase},
};
use iced::{
    Element, Theme,
    widget::{button, checkbox, column, container, row, text, text::Wrapping},
};
use vw_gateway_client::vw_api_types::model_catalog::{ModelDiscoveryFailure, ModelDiscoveryMethod};

fn message(event: Event) -> Message {
    catalog_api::message(CatalogEvent::Discovery(event))
}

pub(super) fn view<'a>(form: &'a CatalogForm, i18n: I18n, theme: &Theme) -> Element<'a, Message> {
    use DiscoveryPhase::*;

    let phase = &form.discovery.phase;
    let ready = !form.busy() && !form.endpoint.trim().is_empty();
    let mut buttons = row![].spacing(8);
    if phase.active() {
        buttons = buttons.push(
            button(text("取消发现"))
                .on_press_maybe(phase.running().then(|| message(Event::Cancel)))
                .padding([8, 12])
                .style(rounded_action_btn_style),
        );
    } else {
        for &method in discovery::methods(form) {
            let retry = form.discovery.method == Some(method);
            let label = match (method, retry) {
                (ModelDiscoveryMethod::Http, false) => "通过 HTTP 发现",
                (ModelDiscoveryMethod::OpenCode, false) => "通过 OpenCode CLI 发现",
                (ModelDiscoveryMethod::Agy, false) => "通过 AGY CLI 发现",
                (ModelDiscoveryMethod::Http, true) => "重试 HTTP 发现",
                (ModelDiscoveryMethod::OpenCode, true) => "重试 OpenCode CLI",
                (ModelDiscoveryMethod::Agy, true) => "重试 AGY CLI",
            };
            buttons = buttons.push(
                button(text(label))
                    .on_press_maybe(ready.then(|| message(Event::Start(method))))
                    .padding([8, 12])
                    .style(primary_action_btn_style),
            );
        }
    }
    let label = i18n.__("发现模型");
    let mut content = column![
        settings_hover_help_label(
            i18n,
            theme,
            label,
            settings_field_label(label),
            i18n.__("使用执行主机上已保存的供应商配置或 CLI。"),
        )
        .wrap(),
        text(i18n.__("勾选结果后可保存到此部署。"))
            .size(13)
            .wrapping(Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        buttons,
    ]
    .spacing(8);

    let label = match phase {
        Idle => None,
        Starting => Some("正在提交发现任务…"),
        Queued => Some("发现任务已排队…"),
        Discovering => Some("正在从供应商或 CLI 发现模型…"),
        Parsing => Some("正在解析发现结果…"),
        Stopping => Some("正在取消发现…"),
        Cancelled => Some("发现已取消，已保存模型保持不变。"),
        Completed(models) if models.is_empty() => Some("发现已完成，未找到模型。"),
        Completed(_) => Some("发现已完成。"),
        _ => None,
    };
    if let Some(label) = label {
        content = content.push(text(label).size(14).style(settings_muted_text_style));
    }
    let failure = match phase {
        CancelUnconfirmed => Some("已停止等待结果，未能确认网关是否取消了任务。"),
        ServiceFailed(reason) => Some(match reason {
            ModelDiscoveryFailure::Unavailable => "发现源不可用，请检查执行主机或供应商后重试。",
            ModelDiscoveryFailure::Unauthorized => "发现授权失败，请检查已保存的凭据或 CLI 登录。",
            ModelDiscoveryFailure::TimedOut => "模型发现超时，请重试。",
            ModelDiscoveryFailure::InvalidResponse => "发现源返回了无法解析的模型，请重试。",
        }),
        Failed(message) => Some(message.as_str()),
        _ => None,
    };
    if let Some(failure) = failure {
        content = content.push(settings_error_banner(failure));
    }

    if !form.discovery.results.is_empty() {
        content = content.push(settings_field_label("最近一次发现的模型"));
        for (index, identity) in form.discovery.results.iter().enumerate() {
            let run = form.discovery.results_run;
            content =
                content.push(
                    checkbox(form.selected.contains(identity))
                        .label(identity.model_id.as_ref())
                        .text_wrapping(Wrapping::WordOrGlyph)
                        .on_toggle_maybe((!form.busy()).then_some(move |checked| {
                            message(Event::Toggle { run, index, checked })
                        }))
                        .style(settings_checkbox_style),
                );
        }
    }
    settings_group_card(container(content).padding([12, 0])).into()
}
