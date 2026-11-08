//! Model metadata already saved and fields proposed by the service.

use crate::app::{
    Message,
    components::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
        settings_field_label, settings_muted_text_style,
    },
    message::settings::providers::metadata_fill::{Event, message},
    state::app_providers::model_catalog::{CatalogForm, FillState},
};
use iced::{
    Element, Length,
    widget::{Column, button, checkbox, column, row, text, text::Wrapping},
};
use vw_gateway_client::vw_api_types::model_catalog::{
    ModelCapabilityState, ModelCatalogCacheStatus, ModelMetadataDto, ModelMetadataField,
    ModelMetadataSource,
};

fn field_rows(metadata: &ModelMetadataDto) -> Column<'_, Message> {
    use ModelMetadataField::*;

    let capability = |value| match value {
        ModelCapabilityState::Unknown => None,
        ModelCapabilityState::Supported => Some("支持".into()),
        ModelCapabilityState::Unsupported => Some("不支持".into()),
    };
    let list = |value: &Option<Vec<String>>| value.as_ref().map(|values| values.join("、"));
    let cost = metadata.cost.as_ref().map(|cost| {
        let mut value = format!("输入 {} / 输出 {}", cost.input, cost.output);
        if let Some(read) = cost.cache_read {
            value.push_str(&format!(" / 缓存读取 {read}"));
        }
        if let Some(write) = cost.cache_write {
            value.push_str(&format!(" / 缓存写入 {write}"));
        }
        value
    });

    let fields = [
        (ContextWindow, "上下文窗口", metadata.context_window.map(|value| value.to_string())),
        (MaxOutputTokens, "最大输出", metadata.max_output_tokens.map(|value| value.to_string())),
        (Reasoning, "推理能力", capability(metadata.reasoning)),
        (ToolCalls, "工具调用", capability(metadata.tool_calls)),
        (ReasoningEfforts, "推理档位", list(&metadata.reasoning_efforts)),
        (InputModalities, "输入模态", list(&metadata.input_modalities)),
        (OutputModalities, "输出模态", list(&metadata.output_modalities)),
        (Cost, "每百万 token 价格（USD）", cost),
    ];
    let mut rows = column![].spacing(6).width(Length::Fill);
    for (field, label, value) in fields {
        let Some(value) = value else { continue };
        let provenance = metadata.provenance.get(&field);
        let source = match provenance.map(|p| p.source) {
            Some(ModelMetadataSource::Manual) => "手工设置",
            Some(ModelMetadataSource::Preset) => "预设",
            Some(ModelMetadataSource::ModelsDev) => "models.dev",
            Some(ModelMetadataSource::Discovery) => "服务发现",
            None => "未记录来源",
        };
        let fetched = provenance
            .and_then(|p| p.fetched_at)
            .and_then(|time| chrono::DateTime::from_timestamp_millis(time.0))
            .map(|time| format!(" · 获取时间 {}", time.format("%Y-%m-%d %H:%M:%S UTC")))
            .unwrap_or_default();
        rows = rows.push(
            column![
                text(format!("{label}：{value}")).size(14).wrapping(Wrapping::WordOrGlyph),
                hint(format!("来源：{source}{fetched}")),
            ]
            .spacing(2),
        );
    }
    rows
}

fn hint<'a>(value: impl Into<String>) -> Element<'a, Message> {
    text(value.into())
        .size(13)
        .style(settings_muted_text_style)
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

pub(super) fn view(form: &CatalogForm, editable: bool) -> Element<'_, Message> {
    let mut body = column![settings_field_label("模型参数补全")].spacing(8).width(Length::Fill);
    if let Some(entry) = form.catalog.as_ref().and_then(|catalog| {
        catalog.items.iter().find(|entry| Some(&entry.identity.model_id) == form.edit_id.as_ref())
    }) && entry.metadata != ModelMetadataDto::default()
    {
        body = body.push(hint("已保存参数")).push(field_rows(&entry.metadata));
    }

    let can_preview = match &form.fill {
        FillState::Idle => true,
        FillState::Loading(_) => {
            body = body.push(hint("正在预览空白参数…"));
            false
        }
        FillState::Committing(_) => {
            body = body.push(hint("正在应用补全…"));
            false
        }
        FillState::Notice(notice) => {
            body = body.push(hint(*notice));
            true
        }
        FillState::Preview(preview) => {
            let status = match preview.models_dev_cache {
                Some(ModelCatalogCacheStatus::Fresh) => "models.dev 缓存未过期",
                Some(ModelCatalogCacheStatus::Stale) => "models.dev 缓存已陈旧，离线仍可使用",
                Some(ModelCatalogCacheStatus::Undated) => {
                    "没有带获取时间的 models.dev 缓存，可离线使用预设或内置数据"
                }
                None => "网关未报告 models.dev 缓存状态",
            };
            body = body.push(hint(status));
            if preview.proposed == ModelMetadataDto::default() {
                body = body.push(hint("没有可补全的空白参数。"));
                true
            } else {
                body = body
                    .push(hint("将填入以下空白字段，保留已设置的值"))
                    .push(field_rows(&preview.proposed))
                    .push(
                        row![
                            button(text("应用补全"))
                                .on_press_maybe(editable.then(|| message(Event::Commit)))
                                .padding([8, 12])
                                .style(primary_action_btn_style),
                            checkbox(form.fill_silent)
                                .label("只在此处显示结果")
                                .on_toggle_maybe(
                                    editable.then_some(|value| message(Event::Silent(value)))
                                )
                                .style(settings_checkbox_style),
                        ]
                        .spacing(12)
                        .align_y(iced::Alignment::Center),
                    );
                false
            }
        }
    };
    if can_preview {
        body = body.push(
            button(text("预览补全"))
                .on_press_maybe(editable.then(|| message(Event::Preview)))
                .padding([8, 12])
                .style(rounded_action_btn_style),
        );
    }
    body.into()
}
