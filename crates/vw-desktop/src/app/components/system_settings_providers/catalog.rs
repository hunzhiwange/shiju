//! 搜索模型供应商目录，并用稳定的供应商 ID 打开连接或加入热门。

mod discovery;
mod metadata_fill;

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    provider_logo_svg, rounded_action_btn_style, settings_close_button, settings_control_with_hint,
    settings_divider, settings_error_banner, settings_group_card, settings_hover_help_label,
    settings_modal_card, settings_modal_header, settings_modal_overlay, settings_muted_text_style,
    settings_text_input, settings_text_input_style,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::message::SettingsMessage;
use crate::app::state::{ModelCatalogEntry, ProviderSummary, is_shiju_official_provider_id};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, row, scrollable, text, text::Wrapping, text_input,
};
use iced::{Alignment, Element, Length};
use std::collections::BTreeMap;
use vw_config_types::ui::DesktopLocale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CatalogProviderItem {
    pub(super) provider_id: String,
    pub(super) provider_name: String,
    pub(super) model_count: usize,
    pub(super) is_connected: bool,
    pub(super) in_popular: bool,
}

pub(super) fn visible_catalog_providers(
    catalog_items: &[ModelCatalogEntry],
    configured_providers: &[ProviderSummary],
    popular_patterns: &[String],
    query: &str,
) -> Vec<CatalogProviderItem> {
    let mut providers = BTreeMap::<String, (String, usize)>::new();
    for item in catalog_items {
        providers
            .entry(item.provider_id.clone())
            .and_modify(|entry| entry.1 += 1)
            .or_insert_with(|| (item.provider_name.clone(), 1));
    }

    let query = query.trim().to_ascii_lowercase();
    providers
        .into_iter()
        .filter_map(|(provider_id, (provider_name, model_count))| {
            if !query.is_empty()
                && !provider_id.to_ascii_lowercase().contains(&query)
                && !provider_name.to_ascii_lowercase().contains(&query)
            {
                return None;
            }

            Some(CatalogProviderItem {
                is_connected: configured_providers
                    .iter()
                    .any(|provider| provider.id == provider_id && provider.connected),
                in_popular: is_shiju_official_provider_id(&provider_id)
                    || popular_patterns.iter().any(|pattern| pattern == &provider_id),
                provider_id,
                provider_name,
                model_count,
            })
        })
        .collect()
}

fn provider_row(locale: DesktopLocale, provider: CatalogProviderItem) -> Element<'static, Message> {
    let CatalogProviderItem { provider_id, provider_name, model_count, is_connected, in_popular } =
        provider;

    let connect = button(
        text(connection_text(locale, if is_connected { "已连接" } else { "连接" }))
            .size(SETTINGS_CONTROL_TEXT_SIZE),
    )
    .on_press_maybe(
        (!is_connected)
            .then(|| Message::Settings(SettingsMessage::ProviderConnectOpen(provider_id.clone()))),
    )
    .padding([8, 12])
    .style(if is_connected { rounded_action_btn_style } else { primary_action_btn_style });
    let add = button(
        text(connection_text(locale, if in_popular { "已在热门" } else { "添加到热门" }))
            .size(SETTINGS_CONTROL_TEXT_SIZE),
    )
    .on_press_maybe((!in_popular).then(|| {
        Message::Settings(SettingsMessage::ProviderCatalogAddToPopular(provider_id.clone()))
    }))
    .padding([8, 12])
    .style(rounded_action_btn_style);

    let model_count = crate::app::i18n::settings_models::model_count(locale, model_count);
    container(
        column![
            row![
                container(provider_logo_svg(&provider_id, 18.0)).center_x(32).center_y(32),
                column![
                    text(provider_name).size(15).wrapping(Wrapping::WordOrGlyph),
                    text(provider_id)
                        .size(12)
                        .wrapping(Wrapping::WordOrGlyph)
                        .style(settings_muted_text_style),
                    text(model_count).size(12).style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            container(row![connect, add].spacing(8).align_y(Alignment::Center))
                .align_right(Length::Fill),
        ]
        .spacing(8),
    )
    .padding([12, 0])
    .width(Length::Fill)
    .into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    if !s.catalog_open {
        return dialog;
    }

    let locale = app.desktop_locale;
    let close = Message::Settings(SettingsMessage::ProviderCatalogClose);
    let title = connection_text(locale, "更多模型提供商");
    let description = connection_text(locale, "浏览模型目录中的 provider，并快速连接或加入热门。");
    let search_hint = connection_text(locale, "按 provider 名称或 provider ID 快速定位可用目录。");
    let header = settings_modal_header(
        settings_hover_help_label(
            I18n::new(locale),
            &app.app_theme,
            title,
            text(title).size(18),
            format!("{description}\n\n{search_hint}"),
        )
        .wrap(),
        settings_close_button(close.clone()),
    );
    let search = text_input(connection_text(locale, "搜索模型提供商"), &s.catalog_query)
        .on_input(|query| Message::Settings(SettingsMessage::ProviderCatalogQueryChanged(query)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);

    let providers = visible_catalog_providers(
        &s.catalog_items,
        &s.providers,
        &s.popular_patterns,
        &s.catalog_query,
    );
    let empty_label = if s.catalog_loading {
        Some("加载中…")
    } else if providers.is_empty() && s.save_error.is_none() {
        Some(if s.catalog_items.is_empty() {
            "暂无可用模型提供商"
        } else {
            "未找到匹配的模型提供商"
        })
    } else {
        None
    };
    let body: Element<'_, Message> = if let Some(label) = empty_label {
        container(text(connection_text(locale, label)).size(14).style(settings_muted_text_style))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        let mut rows = column![];
        if !s.catalog_loading
            && let Some(error) = &s.save_error
        {
            rows = rows.push(settings_error_banner(error.display(locale)));
        }
        if !providers.is_empty() {
            let mut list = column![];
            for (index, provider) in providers.into_iter().enumerate() {
                if index != 0 {
                    list = list.push(settings_divider());
                }
                list = list.push(provider_row(locale, provider));
            }
            rows = rows.push(settings_group_card(list));
        }
        scrollable(container(rows.spacing(10)).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    };

    let card = settings_modal_card(column![header, search, body].spacing(14).height(Length::Fill))
        .width(Length::Fixed(900.0))
        .height(Length::Fixed(520.0));
    settings_modal_overlay(Some(dialog), close, container(card).padding(12))
}

/// Show the service catalog for a saved application provider.
pub(crate) fn deployments<'a>(
    form: &'a crate::app::state::app_providers::model_catalog::CatalogForm,
    generation: u64,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::{
        settings_checkbox_style, settings_field_label, settings_modal_layout,
        settings_responsive_row, settings_value_badge,
    };
    use crate::app::message::settings::providers::catalog_api::{Event, message};

    let close = message(Event::Close);
    let editable = !form.busy();
    let deployment = form.deployment();
    let search = text_input("搜索预设名称、别名或端点", &form.query)
        .on_input_maybe(editable.then_some(|query| message(Event::Query(query))))
        .on_submit_maybe(editable.then(|| message(Event::Search)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let header = column![
        settings_modal_header(
            column![
                text(format!("服务模型目录 · {:?}", form.context.scope.app)).size(20),
                text(format!("供应商 {} · 执行主机 {}", form.provider_id, form.context.scope.host))
                    .size(13)
                    .style(settings_muted_text_style),
            ]
            .spacing(6),
            settings_close_button(close.clone()),
        ),
        row![
            search,
            button(text(if form.notice.is_some() && !form.searched { "重试" } else { "搜索" }))
                .on_press_maybe(editable.then(|| message(Event::Search)))
                .padding([8, 14])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12);

    let endpoint = settings_text_input("例如 https://api.example.com/v1", &form.endpoint)
        .on_input_maybe(editable.then_some(|value| message(Event::Endpoint(value))))
        .on_submit_maybe(
            (editable && !deployment.endpoint.is_empty()).then(|| message(Event::LoadCatalog)),
        );
    let id_label =
        if form.edit_id.is_some() { "编辑模型 · 请求 ID" } else { "手工添加模型" };
    let id = settings_responsive_row(id_label, "", move || -> Element<'a, Message> {
        if let Some(id) = &form.edit_id {
            text(id.as_ref()).wrapping(Wrapping::WordOrGlyph).into()
        } else {
            settings_text_input("填写供应商使用的原始模型 ID", &form.manual_id)
                .on_input_maybe(editable.then_some(|value| message(Event::ManualId(value))))
                .into()
        }
    });
    let has_model = form.edit_id.is_some() || !form.manual_id.trim().is_empty();
    let alias = settings_responsive_row(
        "显示别名",
        "别名仅用于显示，发送请求时使用原始模型 ID。",
        move || {
            settings_text_input("可留空", &form.alias).on_input_maybe(
                (editable && has_model).then_some(|value| message(Event::Alias(value))),
            )
        },
    );
    let mut fields = column![
        settings_field_label("目标部署端点"),
        settings_control_with_hint(
            row![
                endpoint,
                button(text(if form.catalog_loading {
                    "正在读取…"
                } else {
                    "刷新已保存模型"
                }))
                .on_press_maybe(
                    (editable && !deployment.endpoint.is_empty())
                        .then(|| message(Event::LoadCatalog))
                )
                .padding([8, 12])
                .style(rounded_action_btn_style),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
            "选择下方预设端点，也可以直接输入。切换端点会清空当前草稿与已选模型。"
        ),
        id,
        alias,
        settings_responsive_row("默认模型", "", move || {
            checkbox(form.make_default())
                .label("设为此部署的默认模型")
                .on_toggle_maybe(
                    (editable && has_model).then_some(|value| message(Event::MakeDefault(value))),
                )
                .style(settings_checkbox_style)
        }),
    ]
    .spacing(8);
    if form.edit_id.is_some() {
        fields = fields
            .push(
                button(text("取消编辑"))
                    .on_press_maybe(editable.then(|| message(Event::CancelEdit)))
                    .padding([8, 12])
                    .style(rounded_action_btn_style),
            )
            .push(metadata_fill::view(form, editable));
    }
    let mut body = column![settings_group_card(container(fields).padding([12, 0]))].spacing(10);

    body = body.push(discovery::view(form));

    if let Some(notice) = &form.notice {
        body = body.push(settings_error_banner(notice.as_str()));
    }
    if let Some(catalog) = &form.catalog {
        body = body.push(settings_field_label("已保存模型"));
        if catalog.items.is_empty() {
            body = body.push(text("此部署尚未保存模型").size(14).style(settings_muted_text_style));
        }
        for entry in &catalog.items {
            let id = &entry.identity.model_id;
            let mut title = row![
                text(entry.display_alias.as_deref().unwrap_or(id.as_ref()))
                    .size(16)
                    .wrapping(Wrapping::WordOrGlyph)
                    .width(Length::Fill)
            ]
            .align_y(Alignment::Center)
            .spacing(8);
            if catalog.default_model_id.as_ref() == Some(id) {
                title = title.push(settings_value_badge("默认"));
            }
            title =
                title.push(
                    button(text("编辑"))
                        .on_press_maybe(editable.then(|| {
                            message(Event::EditModel { generation, model_id: id.clone() })
                        }))
                        .padding([8, 12])
                        .style(rounded_action_btn_style),
                );
            let mut details = column![title].spacing(4);
            if entry.display_alias.is_some() {
                details = details.push(
                    text(format!("请求 ID：{}", id.as_ref()))
                        .size(13)
                        .wrapping(Wrapping::WordOrGlyph)
                        .style(settings_muted_text_style),
                );
            }
            body = body.push(settings_group_card(container(details).padding([10, 0])));
        }
    }
    let status = if form.loading {
        Some("正在从服务搜索…")
    } else if form.items.is_empty() && form.searched {
        Some(form.empty_label())
    } else if !form.searched && form.notice.is_none() {
        Some("按回车或点击搜索查看预设")
    } else {
        None
    };
    if let Some(status) = status {
        body = body.push(text(status).size(14).style(settings_muted_text_style));
    }

    let mut family = None;
    for (preset_index, preset) in form.items.iter().enumerate() {
        if family != preset.family.as_deref() {
            family = preset.family.as_deref();
            if let Some(family) = family {
                body = body.push(settings_field_label(family));
            }
        }
        let endpoint = preset.endpoint.as_deref().filter(|endpoint| !endpoint.is_empty());
        let active = endpoint == Some(deployment.endpoint.as_str());
        let mut details = column![
            text(&preset.name).size(16).wrapping(Wrapping::WordOrGlyph),
            text(endpoint.unwrap_or("此预设没有部署端点"))
                .size(13)
                .wrapping(Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
        ]
        .spacing(6);
        if endpoint.is_some() {
            details = details.push(
                button(text(if active { "当前部署" } else { "选择此部署" }))
                    .on_press_maybe((!active && editable).then(|| {
                        message(Event::ChooseEndpoint { generation, preset: preset_index })
                    }))
                    .padding([8, 12])
                    .style(rounded_action_btn_style),
            );
        }
        if active {
            for (model_index, id) in preset.model_ids.iter().enumerate() {
                let checked = form
                    .selected
                    .iter()
                    .any(|item| item.deployment == deployment && &item.model_id == id);
                details = details.push(
                    checkbox(checked)
                        .label(id.as_ref())
                        .on_toggle_maybe(editable.then_some(move |checked| {
                            message(Event::Toggle {
                                generation,
                                preset: preset_index,
                                model: model_index,
                                checked,
                            })
                        }))
                        .style(settings_checkbox_style),
                );
            }
        }
        body = body.push(settings_group_card(container(details).padding([12, 0])));
    }

    let ready =
        editable && !deployment.endpoint.is_empty() && (has_model || !form.selected.is_empty());
    let actions = row![
        button(text("关闭"))
            .on_press_maybe((!form.saving).then_some(close.clone()))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text(if form.saving { "正在保存…" } else { "保存模型目录" }))
            .on_press_maybe(ready.then(|| message(Event::Save)))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(super::custom_model_modal::custom_modal_card_style)
        .width(800)
        .height(650);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
