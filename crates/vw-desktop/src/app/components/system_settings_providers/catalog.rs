//! 搜索模型供应商目录，并用稳定的供应商 ID 打开连接或加入热门。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    provider_logo_svg, rounded_action_btn_style, settings_close_button, settings_control_with_hint,
    settings_divider, settings_error_banner, settings_group_card, settings_modal_card,
    settings_modal_header, settings_modal_overlay, settings_muted_text_style,
    settings_text_input_style,
};
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::message::SettingsMessage;
use crate::app::state::{ModelCatalogEntry, ProviderSummary, is_shiju_official_provider_id};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text::Wrapping, text_input};
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
    let header = settings_modal_header(
        column![
            text(connection_text(locale, "更多模型提供商")).size(18),
            text(connection_text(locale, "浏览模型目录中的 provider，并快速连接或加入热门。"))
                .size(13)
                .style(settings_muted_text_style),
        ]
        .spacing(6),
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

    let search = settings_control_with_hint(
        search,
        connection_text(locale, "按 provider 名称或 provider ID 快速定位可用目录。"),
    );
    let card = settings_modal_card(column![header, search, body].spacing(14).height(Length::Fill))
        .width(Length::Fixed(900.0))
        .height(Length::Fixed(520.0));
    settings_modal_overlay(Some(dialog), close, container(card).padding(12))
}
