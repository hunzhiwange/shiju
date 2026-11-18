use super::{BillingModelSource, Event, ImportView, ModelPricingInfo, Outcome, State, message};
use crate::{Identity, models};
use desktop_assets::assets::Icon;
use desktop_ui_controls::{overlays::BelowOverlay, system_settings_common::*};
use iced::{
    Element, Length, Theme,
    widget::{button, column, container, row, rule, text},
};

type Message = Outcome<Event>;

fn action<'a>(
    label: &'static str,
    event: Event,
    enabled: bool,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event)))
}

fn icon_action<'a>(
    icon: Icon,
    label: &'static str,
    event: Event,
    enabled: bool,
) -> Element<'a, Message> {
    icon_btn(icon, label, enabled.then_some(message(event)))
}

fn catalog_action<'a>(enabled: bool) -> iced::widget::Button<'a, Message> {
    action("从 models.dev 导入", Event::ShowImport(Some(ImportView::Catalog)), enabled)
}

pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    current: Option<Identity>,
    base: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    if !state.open {
        return base;
    }

    let enabled = state.scope.is_some()
        && state.scope == current
        && !state.busy
        && !state.billing_busy
        && !state.storage_busy;
    let (title, back, body, footer) = if let Some(import) = state.import_view {
        let (body, footer) = import_view(state, import, enabled);
        ("导入定价", Event::ShowImport(None), body, Some(footer))
    } else if let Some(price) = &state.editor {
        let (body, footer) = price_form(price, state.creating, enabled);
        (
            if state.creating { "新增定价" } else { "编辑定价" },
            Event::CancelEdit,
            body,
            Some(footer),
        )
    } else {
        ("定价", Event::Close, price_list(state, enabled), None)
    };

    let can_go_back = !state.busy && !state.billing_busy && !state.storage_busy;
    let mut header = column![
        row![
            icon_action(Icon::ChevronLeft, "返回", back.clone(), can_go_back),
            text(title).size(22),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        rule::horizontal(1),
    ]
    .spacing(10);
    if state.busy {
        header = header.push(text("处理中…").size(12).style(text::secondary));
    }
    if let Some(error) = &state.error {
        header = header.push(settings_error_banner(error));
    }
    let card = settings_modal_layout(header, body, footer).width(1060).height(730);
    let panel =
        settings_modal_overlay(Some(base), map(message(back)), Element::from(card).map(map));
    let panel = billing_overlay(state, panel, map);
    models::picker(&state.models, panel, move |event| map(message(Event::Models(event))))
}

fn price_list(state: &State, enabled: bool) -> Element<'_, Message> {
    let sync = models::sync_card(&state.models, storage_menu(state, enabled), |event| {
        message(Event::Models(event))
    });
    let title = column![
        text(format!("模型定价 · {}", state.prices.len())).size(17),
        text("USD / 每百万 Tokens").size(12).style(text::secondary),
    ]
    .spacing(4)
    .width(Length::Fill);
    let controls = row![
        title,
        catalog_action(enabled && state.models.catalog_ready()),
        action("新增定价", Event::New, enabled).style(primary_action_btn_style),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let headings = row![
        text("模型").size(13).width(Length::FillPortion(4)),
        text("输入").size(13).width(Length::Fill),
        text("输出").size(13).width(Length::Fill),
        text("缓存读取").size(13).width(Length::Fill),
        text("缓存写入").size(13).width(Length::Fill),
        text("操作").size(13).width(74),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    let mut table =
        column![container(headings).padding([9, 12]).width(Length::Fill).style(|theme: &Theme| {
            container::Style::default()
                .background(theme.extended_palette().background.weak.color)
                .color(theme.palette().text.scale_alpha(0.8))
                .border(iced::border::rounded(iced::border::top(8)))
        })];
    if state.prices.is_empty() {
        let status = if state.busy { "正在读取模型定价…" } else { "暂无模型定价" };
        table = table.push(container(text(status).size(13).style(text::secondary)).padding(14));
    }
    for (index, stored) in state.prices.iter().enumerate().skip(state.page * 20).take(20) {
        let price = &stored.price;
        let id = price.model_id.as_ref();
        let name =
            if price.display_name.trim().is_empty() { id } else { price.display_name.as_str() };
        let source = if stored.is_custom { "自定义" } else { "自动定价" };
        let subtitle = if name == id { source.into() } else { format!("{id} · {source}") };
        let model = column![text(name).size(14), text(subtitle).size(12).style(text::secondary)]
            .spacing(3)
            .width(Length::FillPortion(4));
        let amount = |value: &str| text(format!("${value}")).size(13).width(Length::Fill);
        let actions = row![
            icon_action(Icon::Pencil, "编辑定价", Event::Edit(index), enabled),
            icon_action(Icon::Trash, "删除定价", Event::Delete(id.to_owned()), enabled),
        ]
        .spacing(4)
        .width(74)
        .align_y(iced::Alignment::Center);
        table = table
            .push(
                container(
                    row![
                        model,
                        amount(&price.input_cost_per_million),
                        amount(&price.output_cost_per_million),
                        amount(&price.cache_read_cost_per_million),
                        amount(&price.cache_creation_cost_per_million),
                        actions,
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .padding([7, 12]),
            )
            .push(rule::horizontal(1));
        if state.pending_delete.as_deref() == Some(id) {
            let confirm = row![
                text(format!("删除 {name} 的定价？")).size(13).width(Length::Fill),
                action("取消", Event::CancelDelete, enabled),
                action("确认删除", Event::ConfirmDelete(id.to_owned()), enabled),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            table = table.push(container(confirm).padding([7, 12])).push(rule::horizontal(1));
        }
    }
    let mut body = column![sync, billing_card(state, enabled), controls, table].spacing(14);
    if state.prices.len() > 20 {
        let page = row![
            action("上一页", Event::Page(state.page.saturating_sub(1)), enabled && state.page > 0),
            text(format!("{} / {}", state.page + 1, state.prices.len().div_ceil(20))).size(13),
            action(
                "下一页",
                Event::Page(state.page + 1),
                enabled && (state.page + 1) * 20 < state.prices.len()
            ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        body = body.push(container(page).align_right(Length::Fill));
    }
    body.into()
}

fn storage_menu(state: &State, enabled: bool) -> Element<'_, Message> {
    let trigger = icon_action(Icon::DotsThreeVertical, "定价存储", Event::Files, enabled);
    let mut content = column![text("主机定价数据库").size(13)].spacing(8);
    if let Some(storage) = &state.storage {
        content = content.push(text(&storage.path).size(12).style(text::secondary));
        if let Some(reason) = &storage.reason {
            content = content.push(text(reason).size(12).style(text::secondary));
        }
        content = content.push(action("打开目录", Event::StorageOpen, enabled && storage.can_open));
    } else if state.storage_busy {
        content = content.push(text("正在读取存储位置…").size(12).style(text::secondary));
    } else {
        content = content.push(action("重试读取位置", Event::StorageReload, enabled));
    }
    content = content
        .push(rule::horizontal(1))
        .push(action("重新加载定价", Event::Reload, enabled))
        .push(action("导入 JSON", Event::ShowImport(Some(ImportView::Json)), enabled));
    BelowOverlay::new(trigger, settings_modal_card(content).padding(12).width(270))
        .show(state.files_open)
        .gap(6.0)
        .on_close(message(Event::CloseFiles))
        .into()
}

fn price_form(
    price: &ModelPricingInfo,
    creating: bool,
    enabled: bool,
) -> (Element<'_, Message>, Element<'_, Message>) {
    let mut body = column![].spacing(16);
    if creating {
        body = body.push(
            container(catalog_action(enabled))
                .padding([8, 12])
                .width(Length::Fill)
                .style(super::super::view::panel_style),
        );
    }
    for (index, (label, hint, value)) in [
        ("模型 ID", "例如 gpt-5.4", price.model_id.as_ref()),
        ("显示名称", "例如 GPT-5.4", price.display_name.as_str()),
        ("输入成本（USD / 每百万 Tokens）", "0", price.input_cost_per_million.as_str()),
        ("输出成本（USD / 每百万 Tokens）", "0", price.output_cost_per_million.as_str()),
        ("缓存读取成本（USD / 每百万 Tokens）", "0", price.cache_read_cost_per_million.as_str()),
        (
            "缓存写入成本（USD / 每百万 Tokens）",
            "0",
            price.cache_creation_cost_per_million.as_str(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let input = settings_text_input(hint, value).width(Length::Fill).on_input_maybe(
            (enabled && (index != 0 || creating))
                .then_some(move |value| message(Event::Field(index, value))),
        );
        body = body.push(column![text(label).size(14), input].spacing(6));
    }
    let footer = container(
        row![
            action("取消", Event::CancelEdit, enabled),
            action(if creating { "添加定价" } else { "保存定价" }, Event::Save, enabled)
                .style(primary_action_btn_style),
        ]
        .spacing(8),
    )
    .align_right(Length::Fill);
    (body.into(), footer.into())
}

fn import_view(
    state: &State,
    view: ImportView,
    enabled: bool,
) -> (Element<'_, Message>, Element<'_, Message>) {
    let mut body = column![].spacing(10);
    let cancel = action("返回", Event::ShowImport(None), enabled);
    let mut footer = row![cancel].spacing(8);
    match view {
        ImportView::Catalog => {
            let search = state.import_search.trim().to_lowercase();
            body = body.push(
                settings_text_input("搜索模型或供应商", &state.import_search)
                    .on_input(move |value| message(Event::ImportSearch(value))),
            );
            if !state.models.catalog_ready() {
                body = body.push(text("models.dev 目录暂不可用").size(13).style(text::secondary));
            } else {
                let mut matches = state
                    .models
                    .candidates()
                    .iter()
                    .filter(|entry| {
                        format!(
                            "{} {} {}",
                            entry.model_name, entry.provider_name, entry.normalized_id
                        )
                        .to_lowercase()
                        .contains(&search)
                    })
                    .take(100)
                    .peekable();
                if matches.peek().is_none() {
                    body = body.push(text("没有匹配的模型").size(13).style(text::secondary));
                }
                for entry in matches {
                    let label = row![
                        column![
                            text(&entry.model_name).size(14),
                            text(format!("{} · {}", entry.provider_name, entry.normalized_id))
                                .size(12)
                                .style(text::secondary),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        text("使用此定价").size(12),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center);
                    body = body.push(
                        button(label)
                            .padding([8, 12])
                            .width(Length::Fill)
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                enabled.then(|| message(Event::ImportModel(entry.key.clone()))),
                            ),
                    );
                }
            }
        }
        ImportView::Json => {
            body = body.push(text("粘贴 1–256 条定价记录的 JSON 数组。").size(13))
                .push(settings_text_input("导入来源", &state.source)
                    .on_input_maybe(enabled.then_some(|value| message(Event::Source(value)))))
                .push(text("字段：model_id、display_name、input_cost_per_million、output_cost_per_million、cache_read_cost_per_million、cache_creation_cost_per_million。成本填十进制字符串。").size(12).style(text::secondary))
                .push(settings_text_editor(&state.import).on_action(|change| message(Event::ImportEdit(change))).height(320));
            footer = footer
                .push(action("批量导入", Event::Import, enabled).style(primary_action_btn_style));
        }
    }
    (body.into(), container(footer).align_right(Length::Fill).into())
}

fn billing_card(state: &State, enabled: bool) -> Element<'_, Message> {
    let summary = state.billing.as_ref().map_or_else(
        || if state.billing_busy { "正在读取计费规则…" } else { "计费规则不可用" }.to_owned(),
        |saved| {
            if saved.values().all(|source| *source == BillingModelSource::Response) {
                "按返回模型计价".into()
            } else if saved.values().all(|source| *source == BillingModelSource::Request) {
                "按请求模型计价".into()
            } else {
                "按应用配置计价模型".into()
            }
        },
    );
    container(
        row![
            column![text("计费默认配置").size(16), text(summary).size(12).style(text::secondary)]
                .spacing(4)
                .width(Length::Fill),
            action("修改", Event::BillingEdit, enabled && state.billing.is_some()),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(super::super::view::panel_style)
    .into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BillingChoice(BillingModelSource);

impl std::fmt::Display for BillingChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            BillingModelSource::Response => "返回模型",
            BillingModelSource::Request => "请求模型",
        })
    }
}

fn billing_overlay<'a, M: Clone + 'static>(
    state: &'a State,
    panel: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let Some(draft) = &state.billing_draft else { return panel };
    let enabled = !state.busy && !state.billing_busy;
    let mut body = column![
        text("按请求模型或返回模型匹配定价，仅对后续本地路由请求生效。")
            .size(13)
            .style(text::secondary)
    ]
    .spacing(10);
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error));
    }
    for (app, label) in [
        ("claude", "Claude Code"),
        ("codex", "Codex"),
        ("gemini", "Gemini CLI"),
        ("grokbuild", "Grok Build"),
    ] {
        let Some(source) = draft.get(app) else { continue };
        let select = settings_select(
            [BillingModelSource::Response, BillingModelSource::Request].map(BillingChoice),
            Some(BillingChoice(*source)),
            move |choice| message(Event::BillingField(app.into(), choice.0)),
        )
        .width(152)
        .disabled(!enabled);
        body = body
            .push(
                row![
                    provider_logo_svg(app, 18.0),
                    text(label).size(14).width(Length::Fill),
                    select
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .push(rule::horizontal(1));
    }
    let close = message(Event::BillingCancel);
    let header =
        settings_modal_header(text("计费默认配置").size(20), settings_close_button(close.clone()));
    let changed = state.billing.as_ref().is_some_and(|saved| saved != draft);
    let footer = container(
        row![
            action("取消", Event::BillingCancel, enabled),
            action("保存配置", Event::BillingSave, enabled && changed)
                .style(primary_action_btn_style),
        ]
        .spacing(8),
    )
    .align_right(Length::Fill);
    let dialog = settings_modal_layout(header, body, Some(footer.into())).width(510).height(390);
    settings_modal_overlay(Some(panel), map(close), Element::from(dialog).map(map))
}
