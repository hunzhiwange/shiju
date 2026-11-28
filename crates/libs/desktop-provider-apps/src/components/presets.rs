//! Search and version selection for the application-provider picker.
use crate::presets::{self, Availability, Category, Event, Picker, Preset};
use app_model_catalog::preset_search::{
    PresetMatch, PresetSearchEntry, PresetSearchHit, search_presets,
};
use desktop_assets::assets::{Icon, find_provider_icon};
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{
        button, column, container, row, scrollable,
        scrollable::{Direction, Scrollbar},
        svg, text,
    },
};
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::ModelDevProviderPreset;

pub const SEARCH: &str = "app-provider-preset-search";
pub const RESULTS: &str = "app-provider-preset-results";
pub const VERSIONS: &str = "app-provider-preset-versions";
pub const MODEL_SEARCH: &str = "app-provider-preset-model-search";

pub fn card<'a>(
    i18n: I18n,
    app: AppId,
    picker: &'a Picker,
    has_provider: impl Fn(&str) -> bool,
) -> Element<'a, Event> {
    let catalog = picker.catalog.as_ref().and_then(|result| result.as_ref().ok());
    if let Some(provider) = catalog.and_then(|rows| rows.get(picker.catalog_provider?)) {
        return catalog_models(i18n, provider, &picker.catalog_model_query);
    }
    let entries = presets::entries(app);
    let all = search_presets(&entries, "", |_| None);
    let query = domain(picker.query.trim()).unwrap_or_else(|| picker.query.clone());
    let mut results = search_presets(&entries, &query, |key| {
        preset_label(key).map(|label| i18n.__(label).to_owned())
    });
    if let Some(hit) = results.iter().find(|hit| {
        hit.group.family.is_some_and(|family| picker.family.as_deref() == Some(family.id.as_str()))
    }) {
        return versions_card(i18n, hit, has_provider);
    }
    match picker.category {
        Category::Cloud | Category::Local => results.sort_by_cached_key(|hit| {
            hit.group
                .family
                .map_or(hit.group.versions[0].name, |family| &family.name)
                .to_lowercase()
        }),
        Category::Vendor => results.sort_by_cached_key(|hit| group_name(i18n, hit).to_lowercase()),
        _ => {
            results.sort_by_key(|hit| hit.group.versions[0].preset.category() != Category::Account)
        }
    }
    let catalog_query = picker.query.trim().to_lowercase();
    let catalog_hits: Vec<_> = catalog
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, row)| {
            catalog_query.is_empty()
                || [&row.name, &row.provider_id, &row.base_url, &row.deployment]
                    .into_iter()
                    .any(|value| value.to_lowercase().contains(&catalog_query))
                || row.models.iter().any(|model| {
                    model.id.to_lowercase().contains(&catalog_query)
                        || model.name.to_lowercase().contains(&catalog_query)
                })
        })
        .collect();
    let categories = Category::ORDER.into_iter().filter(|category| {
        matches!(category, Category::All | Category::ThirdParty)
            || all.iter().any(|hit| hit.group.versions[0].preset.category() == *category)
    });
    let mut filters = row![].spacing(7);
    for category in categories {
        let count = results
            .iter()
            .filter(|hit| {
                category == Category::All || hit.group.versions[0].preset.category() == category
            })
            .count();
        let count = if matches!(category, Category::All | Category::ThirdParty) {
            count + catalog_hits.len()
        } else {
            count
        };
        let active = picker.category == category;
        filters = filters.push(
            button(
                row![
                    text(i18n.__(category.label())).size(13),
                    text(count.to_string()).size(12).style(settings_muted_text_style),
                ]
                .spacing(8),
            )
            .padding([7, 11])
            .style(move |theme, status| settings_tab_style(theme, status, active))
            .on_press(Event::Filter(category)),
        );
    }

    let mut cards = column![].spacing(12).width(Length::Fill);
    let show_custom = picker.query.trim().is_empty();
    if picker.category == Category::Account {
        cards = cards.push(
            text(i18n.__(
                "账号登录使用已有账号，无需填写 API Key。标有「需要路由」的供应商须通过路由使用。",
            ))
            .size(13)
            .style(settings_muted_text_style),
        );
    }
    if picker.category == Category::Vendor {
        cards = cards.push(
            text(i18n.__("按厂商浏览官方 API 和编程套餐，选中后再挑选地区或套餐。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if picker.category == Category::ThirdParty {
        cards = cards.push(
            text(i18n.__("从识矩模型目录选择与当前应用兼容的连接。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if picker.category == Category::Cloud {
        cards = cards.push(
            text(i18n.__("使用云服务商的账号和 API 凭据调用模型。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if picker.category == Category::Local {
        let hint = if app == AppId::ClaudeDesktop {
            "请在执行主机启动推理服务并加载模型。Claude Desktop 的本地预设需要路由。"
        } else {
            "请在执行主机启动推理服务并加载模型。服务未启用鉴权时，必填的 API Key 可使用占位值。"
        };
        cards = cards.push(text(i18n.__(hint)).size(13).style(settings_muted_text_style));
    }
    if picker.account_help {
        let detail = text(i18n.__("此入口暂不能创建路由供应商。")).size(13).width(Length::Fill);
        #[cfg(not(target_arch = "wasm32"))]
        let detail = row![
            detail,
            button(text(i18n.__("前往账号授权")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(Event::OpenAuthorization),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        cards = cards.push(settings_panel(detail));
    }
    let mut tiles = Vec::new();
    if show_custom {
        tiles.push(tile(
            i18n.__("手动配置").into(),
            i18n.__("填写请求地址和 API Key").into(),
            icon_svg(Icon::Sliders, 20.0).style(icons::icon_btn_svg_style).into(),
            None,
            Some(Event::Custom),
        ));
    }
    let shown = results.into_iter().filter(|hit| {
        picker.category == Category::All
            || picker.category == hit.group.versions[0].preset.category()
    });
    for hit in shown {
        let versions = &hit.group.versions;
        let first = versions[0];
        let Some(family) = hit.group.family else {
            tiles.push(version_card(i18n, first, false, &has_provider));
            continue;
        };
        let name = group_name(i18n, &hit);
        let count = i18n
            .format("%s 个版本", &[versions.len().to_string().into()])
            .expect("valid version count");
        tiles.push(tile(
            name.into(),
            [first.website_url.and_then(domain), Some(count.to_string())]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · "),
            preset_icon(first.preset, name),
            versions
                .iter()
                .all(|version| version.preset.availability() == Availability::Routing)
                .then(|| i18n.__("需要路由")),
            Some(Event::OpenVersions(family.id.clone())),
        ));
    }
    if matches!(picker.category, Category::All | Category::ThirdParty) {
        let mut duplicates = std::collections::HashSet::new();
        let mut seen = std::collections::HashSet::new();
        for (_, provider) in catalog_hits.iter().copied() {
            let identity = (provider.name.as_str(), provider.base_url.as_str());
            if !seen.insert(identity) {
                duplicates.insert(identity);
            }
        }
        for (index, provider) in catalog_hits {
            let mut detail = vec![provider.base_url.clone()];
            if duplicates.contains(&(provider.name.as_str(), provider.base_url.as_str())) {
                detail.push(provider.deployment.clone());
                if app == AppId::Claude {
                    detail
                        .push(if provider.api_key_header { "x-api-key" } else { "Bearer" }.into());
                }
            }
            if app != AppId::ClaudeDesktop {
                let count = i18n
                    .format("%s 个模型", &[provider.models.len().to_string().into()])
                    .expect("valid model count");
                detail.push(count.to_string());
            }
            tiles.push(tile(
                provider.name.clone(),
                detail.join(" · "),
                logo(Some(&provider.provider_id), &provider.name),
                None,
                Some(Event::ChooseCatalogProvider(index)),
            ));
        }
        let status: Option<Element<'a, Event>> = match &picker.catalog {
            None => Some(text(i18n.__("正在读取识矩模型目录…")).size(13).into()),
            Some(Err(())) => Some(
                row![
                    text(i18n.__("无法读取识矩模型目录")).size(13).width(Length::Fill),
                    button(text(i18n.__("重试")).size(13))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press(Event::RetryCatalog),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ),
            Some(Ok(_)) => None,
        };
        if let Some(status) = status {
            cards = cards.push(settings_panel(status));
        }
    }
    if tiles.is_empty()
        && (!matches!(picker.category, Category::All | Category::ThirdParty)
            || picker.catalog.as_ref().is_some_and(Result::is_ok))
    {
        cards = cards.push(settings_panel(text(i18n.__("没有匹配的供应商预设")).size(14)));
    }
    let mut tiles = tiles.into_iter();
    while let Some(first) = tiles.next() {
        let second = tiles.next().unwrap_or_else(|| iced::widget::Space::new().into());
        cards = cards.push(
            row![
                container(first).width(Length::FillPortion(1)),
                container(second).width(Length::FillPortion(1)),
            ]
            .spacing(12),
        );
    }

    let title = row![
        text(i18n.__("添加供应商")).size(21),
        text(presets::app_name(app).unwrap_or_default()).size(14).style(settings_muted_text_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let search = settings_text_input(i18n.__("搜索供应商名称或网址"), &picker.query)
        .id(SEARCH)
        .on_input(Event::Query)
        .width(Length::Fill);
    let clear = button(text(i18n.__("清空搜索")).size(13))
        .padding([8, 11])
        .style(rounded_action_btn_style)
        .on_press_maybe((!picker.query.is_empty()).then(|| Event::Query(String::new())));
    let mut controls = row![search, clear].spacing(8).align_y(Alignment::Center);
    let custom = button(text(i18n.__("手动配置")).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(Event::Custom);
    if !show_custom {
        controls = controls.push(custom);
    }
    let results = scrollable(container(cards).padding([0, 7]).width(Length::Fill))
        .id(RESULTS)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill);
    container(
        settings_modal_card(
            column![
                settings_modal_header(title, settings_close_button(Event::Close)),
                controls,
                filters.wrap(),
                results,
            ]
            .spacing(14),
        )
        .width(880)
        .height(670),
    )
    .padding(12)
    .into()
}

fn catalog_models<'a>(
    i18n: I18n,
    provider: &ModelDevProviderPreset,
    query: &str,
) -> Element<'a, Event> {
    let back = button(text(i18n.__("返回预设")).size(13))
        .padding([8, 11])
        .style(rounded_action_btn_style)
        .on_press(Event::BackToCatalog);
    let title =
        row![back, text(provider.name.clone()).size(21)].spacing(10).align_y(Alignment::Center);
    let search = settings_text_input(i18n.__("搜索模型"), query)
        .id(MODEL_SEARCH)
        .on_input(Event::CatalogModelQuery)
        .width(Length::Fill);
    let needle = query.trim().to_lowercase();
    let mut models = column![].spacing(8).width(Length::Fill);
    let mut matches = provider
        .models
        .iter()
        .enumerate()
        .filter(|(_, model)| {
            model.id.to_lowercase().contains(&needle) || model.name.to_lowercase().contains(&needle)
        })
        .peekable();
    if matches.peek().is_none() {
        models =
            models.push(text(i18n.__("没有匹配的模型")).size(13).style(settings_muted_text_style));
    }
    for (index, model) in matches {
        models = models.push(
            button(
                column![
                    text(model.name.clone()).size(14),
                    text(model.id.clone()).size(12).style(settings_muted_text_style),
                ]
                .spacing(4),
            )
            .padding([9, 12])
            .width(Length::Fill)
            .style(rounded_action_btn_style)
            .on_press(Event::ChooseCatalogModel(index)),
        );
    }
    let results = scrollable(container(models).padding([0, 7]).width(Length::Fill))
        .id(VERSIONS)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill);
    container(
        settings_modal_card(
            column![
                settings_modal_header(title, settings_close_button(Event::Close)),
                text(provider.base_url.clone()).size(13).style(settings_muted_text_style),
                search,
                results,
            ]
            .spacing(12),
        )
        .width(880)
        .height(670),
    )
    .padding(12)
    .into()
}

fn group_name<'a>(i18n: I18n, hit: &PresetSearchHit<'a, Preset>) -> &'a str {
    hit.group.family.map_or_else(
        || entry_name(i18n, hit.group.versions[0]),
        |family| {
            family
                .name_key
                .as_deref()
                .and_then(preset_label)
                .map_or(family.name.as_str(), |label| i18n.__(label))
        },
    )
}

fn entry_name<'a>(i18n: I18n, entry: &PresetSearchEntry<'a, Preset>) -> &'a str {
    entry.name_key.and_then(preset_label).map_or(entry.name, |label| i18n.__(label))
}

fn versions_card<'a>(
    i18n: I18n,
    hit: &PresetSearchHit<'_, Preset>,
    has_provider: impl Fn(&str) -> bool,
) -> Element<'a, Event> {
    let back = button(text(i18n.__("返回预设")).size(13))
        .padding([8, 11])
        .style(rounded_action_btn_style)
        .on_press(Event::BackToCatalog);
    let header = settings_modal_header(
        row![
            back,
            text(group_name(i18n, hit).to_owned()).size(21),
            text(i18n.__("选择版本")).size(14).style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        settings_close_button(Event::Close),
    );
    let mut versions = column![].spacing(8).width(Length::Fill);
    if hit.group.versions.iter().all(|entry| entry.preset.availability() == Availability::Routing) {
        versions = versions.push(
            text(i18n.__("此入口暂不能创建路由供应商。")).size(13).style(settings_muted_text_style),
        );
    }
    let mut indices: Vec<_> = (0..hit.group.versions.len()).collect();
    if let PresetMatch::Versions(matched) = &hit.matched {
        indices.sort_by_key(|index| !matched.contains(index));
    }
    for index in indices {
        versions =
            versions.push(version_card(i18n, hit.group.versions[index], true, &has_provider));
    }
    let results = scrollable(container(versions).padding([0, 7]).width(Length::Fill))
        .id(VERSIONS)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill);
    container(settings_modal_card(column![header, results].spacing(14)).width(880).height(670))
        .padding(12)
        .into()
}

fn preset_label(key: &str) -> Option<&'static str> {
    Some(match key {
        "providerPreset.family.tencent" => "腾讯云",
        "providerPreset.family.volcengine" => "火山引擎",
        "providerPreset.region.cn" => "中国大陆",
        "providerPreset.region.intl" => "国际",
        "providerPreset.plan.payg" => "按量付费",
        "providerPreset.plan.coding" | "providerPreset.plan.codingPlan" => "编程套餐",
        "providerPreset.plan.agentPlan" => "Agent Plan",
        "providerPreset.plan.tokenPlan" => "Token Plan",
        "providerPreset.plan.enterpriseLite" => "企业轻量版",
        "providerPreset.plan.enterprisePro" => "企业专业版",
        "providerPreset.plan.stepPlan" => "Step Plan",
        "providerPreset.plan.aksk" => "AK/SK",
        "providerPreset.plan.apiKey" => "API Key",
        _ => return None,
    })
}

fn version_card<'a>(
    i18n: I18n,
    entry: &PresetSearchEntry<'_, Preset>,
    show_version: bool,
    has_provider: impl Fn(&str) -> bool,
) -> Element<'a, Event> {
    let (badge, enabled) = match entry.preset.availability() {
        Availability::Official(id) if has_provider(id) => (Some(i18n.__("官方")), true),
        Availability::Official(_) => (Some(i18n.__("官方供应商未保存")), false),
        Availability::Direct if entry.preset.desktop_endpoint_needs_account_id() => {
            (Some(i18n.__("需补全地址")), true)
        }
        Availability::Direct => (None, true),
        Availability::Routing => {
            (Some(i18n.__("需要路由")), entry.preset.account_label().is_some())
        }
        Availability::Unavailable => (Some(i18n.__("此预设不可直连")), false),
    };
    let detail = if let Some(label) = entry.preset.account_label() {
        i18n.__(label).to_owned()
    } else {
        let plan = show_version
            .then_some(entry.plan)
            .flatten()
            .and_then(|plan| preset_label(plan.label_key()));
        let region = show_version
            .then_some(entry.region)
            .flatten()
            .and_then(|region| preset_label(region.label_key()));
        plan.into_iter()
            .chain(region)
            .map(|label| i18n.__(label).to_owned())
            .chain(entry.preset.local_endpoint().map(str::to_owned))
            .chain(entry.website_url.and_then(domain))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let name = entry_name(i18n, entry);
    tile(
        name.to_owned(),
        detail,
        preset_icon(entry.preset, name),
        badge,
        enabled.then(|| Event::Select(entry.name.to_owned())),
    )
}

fn domain(site: &str) -> Option<String> {
    let url = url::Url::parse(site).ok()?;
    let host = url.host_str()?;
    Some(host.strip_prefix("www.").unwrap_or(host).to_owned())
}

fn preset_icon<'a>(preset: Preset, name: &str) -> Element<'a, Event> {
    logo(preset.icon(), name)
}

fn logo<'a>(icon: Option<&str>, name: &str) -> Element<'a, Event> {
    match icon.and_then(find_provider_icon) {
        Some(handle) => {
            svg(handle).width(23).height(23).style(icons::provider_logo_svg_style).into()
        }
        None => text(name.chars().take(2).collect::<String>())
            .size(13)
            .style(settings_muted_text_style)
            .into(),
    }
}

fn tile<'a>(
    name: String,
    detail: String,
    icon: Element<'a, Event>,
    badge: Option<&'static str>,
    event: Option<Event>,
) -> Element<'a, Event> {
    let body =
        column![text(name).size(14), text(detail).size(12).style(settings_muted_text_style),]
            .spacing(4)
            .width(Length::Fill);
    let mut content = row![container(icon).width(30), body].spacing(9).align_y(Alignment::Center);
    if let Some(badge) = badge {
        content = content.push(settings_value_badge(badge));
    }
    button(content)
        .padding([10, 12])
        .width(Length::Fill)
        .style(rounded_action_btn_style)
        .on_press_maybe(event)
        .into()
}
