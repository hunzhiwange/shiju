//! 详情的概要、应用信息与个人使用设置。
use desktop_app_launch::{
    ShijuAppDetailTarget, ShijuAppsLoadState, ShijuAppsState, ShijuDetailMetadata,
    ShijuMarketplaceAction,
};
use desktop_apps_ui::shiju_apps::operations::DetailAction;
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use gateway_shiju_app::{ShijuAppPresentation, ShijuCatalogAppDto, ShijuInstalledAppDto};
use iced::widget::{Column, button, column, container, row, rule};
use iced::{Alignment, Element, Font, Length};

use super::{
    Event, Message, action, app_icon, date, icon_svg, mine, muted, open_installed, styles, title,
};

struct Detail<'a> {
    name: &'a str,
    description: &'a str,
    presentation: &'a ShijuAppPresentation,
    icon_url: Option<&'a str>,
    status: &'static str,
    fields: Vec<(&'static str, String)>,
    catalog_id: Option<&'a str>,
}

impl<'a> Detail<'a> {
    fn installed(i18n: I18n, app: &'a ShijuInstalledAppDto) -> Self {
        let readiness =
            mine::runtime_readiness(app).map(str::to_owned).unwrap_or_else(|error| error);
        Self {
            name: &app.name,
            description: app.description.as_deref().unwrap_or_default(),
            presentation: &app.presentation,
            icon_url: app.icon_url.as_deref(),
            status: if app.status == "published" {
                "已发布"
            } else if app.runtime_ready() {
                "运行就绪"
            } else {
                "未就绪"
            },
            fields: vec![
                ("创建人", developer(&app.presentation)),
                (
                    "应用来源",
                    i18n.__(if mine::market_source(&app.source) {
                        "市场应用"
                    } else {
                        "企业应用"
                    })
                    .into(),
                ),
                ("分类", category(i18n, &app.presentation)),
                ("当前版本", app.version.clone()),
                ("更新时间", date(i18n, app.updated_at.as_deref(), true)),
                ("应用 ID", app.app_id.clone()),
                ("安装实例 ID", app.installation_id.clone()),
                ("运行状态", readiness),
                ("创建时间", date(i18n, app.created_at.as_deref(), true)),
                ("协调状态", app.reconcile_status.clone()),
                ("运行时状态", app.runtime_status.clone()),
            ],
            catalog_id: None,
        }
    }

    fn catalog(i18n: I18n, app: &'a ShijuCatalogAppDto) -> Self {
        Self {
            name: &app.name,
            description: app.description.as_deref().unwrap_or_default(),
            presentation: &app.presentation,
            icon_url: app.icon_url.as_deref(),
            status: if app.installed { "已安装" } else { "未安装" },
            fields: vec![
                ("创建人", developer(&app.presentation)),
                ("应用来源", i18n.__("市场应用").into()),
                ("分类", category(i18n, &app.presentation)),
                ("当前版本", app.version.clone()),
                ("应用 ID", app.app_id.clone()),
                ("用户 ID", app.user_id.clone().unwrap_or_default()),
                ("安装实例 ID", app.installation_id.clone().unwrap_or_default()),
                ("安装时间", date(i18n, app.installed_at.as_deref(), true)),
                (
                    "权限",
                    if app.permissions.is_empty() {
                        i18n.__("无").into()
                    } else {
                        app.permissions.join(" · ")
                    },
                ),
            ],
            catalog_id: (!app.installed).then_some(&app.app_id),
        }
    }
}

fn developer(info: &ShijuAppPresentation) -> String {
    info.created_by_user_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .or(info.created_by_user_id.as_deref())
        .unwrap_or_default()
        .into()
}

fn category(i18n: I18n, info: &ShijuAppPresentation) -> String {
    let preferred =
        if i18n.__("全部") == "All" { &info.category_name_en } else { &info.category_name };
    preferred
        .as_deref()
        .filter(|name| !name.is_empty())
        .or(info.category_name.as_deref().filter(|name| !name.is_empty()))
        .or(info.category_code.as_deref().filter(|name| !name.is_empty()))
        .unwrap_or(i18n.__("未分类"))
        .into()
}

fn panel(
    name: &'static str,
    icon: Option<Icon>,
    body: Element<'static, Message>,
) -> Element<'static, Message> {
    let header = match icon {
        Some(icon) => row![icon_svg(icon, 15.0).style(styles::muted_icon), title(name, 16)],
        None => row![title(name, 16)],
    }
    .align_y(Alignment::Center)
    .spacing(8);
    container(column![
        container(header).padding([14, 18]),
        rule::horizontal(1).style(styles::divider_style),
        container(body).padding([16, 18]).width(Length::Fill),
    ])
    .width(Length::Fill)
    .style(styles::panel_style)
    .into()
}

fn notes(i18n: I18n, value: &str) -> Element<'static, Message> {
    muted(
        if value.trim().is_empty() { i18n.__("暂无说明").to_owned() } else { value.to_owned() },
        13,
    )
    .line_height(1.55)
    .into()
}

fn info_panel(i18n: I18n, fields: Vec<(&'static str, String)>) -> Element<'static, Message> {
    let fields = fields.into_iter().fold(column![].spacing(14), |column, (label, value)| {
        let value = if value.is_empty() { "—".to_string() } else { value };
        let is_code = matches!(label, "应用 ID" | "安装实例 ID" | "用户 ID") && value.is_ascii();
        let value = title(value, 13).font(if is_code {
            Font::MONOSPACE
        } else {
            Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }
        });
        column.push(row![muted(i18n.__(label), 13).width(92), value.width(Length::Fill)].spacing(8))
    });
    panel(i18n.__("应用信息"), None, fields.into())
}

fn binding_panel(i18n: I18n, state: &ShijuAppsState) -> Element<'static, Message> {
    let (label, ready, button_label, event) = match &state.binding {
        ShijuAppsLoadState::Loaded(binding) => (
            if binding.personal.configured { "已配置" } else { "待配置" },
            binding.personal.configured,
            if binding.personal.configured { "更换密钥" } else { "选择密钥" },
            binding.personal.can_edit.then_some(Event::ChangeBinding),
        ),
        ShijuAppsLoadState::Error(_) => {
            ("检查失败", false, "重试", Some(Event::RefreshDetailBinding))
        }
        _ => ("正在检查", false, "选择密钥", None),
    };
    panel(
        i18n.__("使用设置"),
        None,
        column![
            row![
                title(i18n.__("API 密钥"), 13).width(Length::Fill),
                container(title(i18n.__(label), 12))
                    .padding([4, 8])
                    .style(move |theme| styles::binding_state_style(theme, !ready)),
            ]
            .align_y(Alignment::Center),
            muted(i18n.__("仅用于当前应用的访问授权。"), 13),
            action(i18n.__(button_label), event).width(Length::Fill),
        ]
        .spacing(12)
        .into(),
    )
}

fn detail_for<'a>(
    i18n: I18n,
    state: &'a ShijuAppsState,
    target: ShijuAppDetailTarget,
    empty: &'a ShijuAppPresentation,
) -> Option<Detail<'a>> {
    match target {
        ShijuAppDetailTarget::Mine(index) => match &state.mine {
            ShijuAppsLoadState::Loaded(mine) => {
                mine.apps.get(index).map(|app| Detail::installed(i18n, app))
            }
            _ => None,
        },
        ShijuAppDetailTarget::Store(index) => match &state.store {
            ShijuAppsLoadState::Loaded(store) => {
                store.apps.get(index).map(|app| Detail::catalog(i18n, app))
            }
            _ => None,
        },
        ShijuAppDetailTarget::Featured(index) => match &state.featured {
            ShijuAppsLoadState::Loaded(featured) => {
                let entry = featured.get(index)?;
                let mut detail = Detail::installed(i18n, &entry.app);
                if entry.open_catalog_detail && entry.app.installation_id.is_empty() {
                    detail.catalog_id = Some(&entry.app.app_id);
                    detail.status = "未安装";
                }
                Some(detail)
            }
            _ => None,
        },
        ShijuAppDetailTarget::Created(index) => match &state.created {
            ShijuAppsLoadState::Loaded(created) => {
                let app = created.get(index)?;
                let installed = match &state.mine {
                    ShijuAppsLoadState::Loaded(mine) => mine.apps.iter().find(|item| {
                        Some(item.installation_id.as_str()) == app.installation_id.as_deref()
                    }),
                    _ => None,
                };
                let mut detail =
                    installed.map(|item| Detail::installed(i18n, item)).unwrap_or_else(|| Detail {
                        name: &app.title,
                        description: app.description.as_deref().unwrap_or_default(),
                        presentation: empty,
                        icon_url: app.icon_url.as_deref(),
                        status: app.lifecycle(),
                        fields: vec![
                            (
                                "创建人",
                                app.created_by_user_name
                                    .clone()
                                    .or_else(|| app.created_by_user_id.clone())
                                    .unwrap_or_default(),
                            ),
                            ("应用 ID", app.app_id.clone()),
                            ("当前版本", app.active_version.clone()),
                            ("更新时间", date(i18n, app.updated_at.as_deref(), true)),
                        ],
                        catalog_id: None,
                    });
                detail.status = app.lifecycle();
                if let Some(scope) = app.access_scope.as_deref() {
                    detail.fields.push((
                        "访问权限",
                        i18n.__(if scope == "company" { "全员" } else { "指定成员" }).into(),
                    ));
                }
                if let Some(version) = &app.published_version {
                    detail.fields.push(("发布版本", version.clone()));
                }
                Some(detail)
            }
            _ => None,
        },
    }
}

pub(super) fn view(
    i18n: I18n,
    state: &ShijuAppsState,
    target: ShijuAppDetailTarget,
    width: f32,
) -> Element<'static, Message> {
    let back = super::tab_specs()
        .iter()
        .find(|(_, tab)| *tab == state.tab)
        .map(|(label, _)| *label)
        .unwrap_or("返回");
    let back_button = button(
        row![icon_svg(Icon::ArrowLeft, 14.0).style(styles::muted_icon), title(i18n.__(back), 14)]
            .align_y(Alignment::Center)
            .spacing(8),
    )
    .on_press(Event::CloseDetail.into())
    .style(styles::text_button_style)
    .padding([6, 0]);
    let empty = ShijuAppPresentation::default();
    let Some(mut detail) = detail_for(i18n, state, target, &empty) else {
        return column![back_button, muted(i18n.__("应用条目不存在"), 14)].spacing(16).into();
    };
    if let ShijuAppsLoadState::Loaded(metadata) = &state.detail_metadata {
        match metadata {
            ShijuDetailMetadata::Development(app) => {
                detail.name = &app.title;
                detail.description = app.description.as_deref().unwrap_or(detail.description);
                detail.presentation = &app.presentation;
                detail.icon_url = app.icon_url.as_deref().or(detail.icon_url);
                if let Some((_, current)) =
                    detail.fields.iter_mut().find(|(label, _)| *label == "当前版本")
                {
                    current.clone_from(&app.active_version);
                }
            }
            ShijuDetailMetadata::Installation(app) => {
                detail.name = &app.name;
                detail.description = app.description.as_deref().unwrap_or(detail.description);
                detail.presentation = &app.presentation;
                detail.icon_url = app.icon_url.as_deref().or(detail.icon_url);
            }
        }
        for (label, value) in &mut detail.fields {
            match *label {
                "创建人" if !developer(detail.presentation).is_empty() => {
                    *value = developer(detail.presentation)
                }
                "分类" => *value = category(i18n, detail.presentation),
                _ => {}
            }
        }
    }
    let installation = state.detail_installation();
    let installed = state.installed_detail();
    let busy = state.action.is_some();
    let mut actions = row![].spacing(8);
    if installed.is_some_and(|app| app.presentation.can_copy) {
        actions = actions.push(action(
            i18n.__("复制应用"),
            (!busy).then_some(Event::DetailAction(DetailAction::Copy)),
        ));
    }
    if installation.is_some_and(gateway_shiju_app::valid_installation_id) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            actions = actions.push(action(
                i18n.__("下载技能包"),
                (!busy).then_some(Event::DetailAction(DetailAction::Download)),
            ));
        }
        actions = actions
            .push(action(i18n.__("分享应用"), Some(Event::DetailAction(DetailAction::Share))));
    }
    if let Some(app) = installed {
        let favorite = app.presentation.is_favorite;
        actions = actions.push(action(
            i18n.__(if favorite { "取消收藏" } else { "收藏应用" }),
            (!busy).then(|| {
                Event::MarketplaceAction(ShijuMarketplaceAction::Favorite(
                    app.installation_id.clone(),
                    !favorite,
                ))
            }),
        ));
    }
    let primary = if let Some(id) = detail.catalog_id {
        Some((
            "安装",
            (!busy)
                .then(|| Event::MarketplaceAction(ShijuMarketplaceAction::Install(id.to_owned()))),
        ))
    } else {
        installed.map(|item| ("打开", open_installed(&item.installation_id)))
    };
    if let Some((label, event)) = primary {
        actions = actions.push(
            action(i18n.__(label), event).style(styles::primary_button_style).padding([9, 14]),
        );
    }
    let intro = row![
        app_icon(64.0, state.artwork(detail.icon_url)),
        column![
            row![
                title(detail.name.to_owned(), 26),
                container(title(i18n.__(detail.status), 12))
                    .padding([3, 8])
                    .style(move |theme| styles::lifecycle_style(theme, detail.status)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            muted(detail.description.to_owned(), 14).line_height(1.55),
        ]
        .spacing(8)
        .width(Length::Fill),
    ]
    .spacing(16)
    .width(Length::Fill);
    let actions = actions.wrap();
    let intro: Element<'static, Message> = if width >= 1050.0 {
        row![intro, actions].spacing(16).into()
    } else {
        column![intro, actions].spacing(12).into()
    };
    let overview = detail.presentation.overview.as_ref();
    let features = overview.map(|value| &value.features).filter(|features| !features.is_empty());
    let scenarios = overview.map(|value| value.scenarios.join("\n")).unwrap_or_default();
    let permissions = overview
        .map(|value| value.data_notes.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if detail.presentation.can_manage {
                i18n.__("当前用户可使用并管理此应用。")
            } else if installed.is_some() {
                i18n.__("当前用户可使用此应用。")
            } else {
                ""
            }
        });
    let feature_body = match features {
        Some(features) => features
            .iter()
            .fold(Column::new().spacing(12), |column, feature| {
                column.push(
                    column![title(feature.title.clone(), 13), notes(i18n, &feature.description)]
                        .spacing(4),
                )
            })
            .into(),
        None => notes(i18n, detail.description),
    };
    let main = column![
        panel(i18n.__("这个应用能做什么"), None, feature_body),
        panel(i18n.__("适用场景"), None, notes(i18n, &scenarios)),
        panel(
            i18n.__("使用前需要了解"),
            Some(Icon::Circle),
            notes(i18n, overview.map(|value| value.before_use.as_str()).unwrap_or_default())
        ),
        panel(i18n.__("数据与权限"), Some(Icon::CheckSquare), notes(i18n, permissions)),
    ]
    .spacing(18)
    .width(Length::Fill);
    let mut sidebar = column![info_panel(i18n, detail.fields)].spacing(18);
    if installation.is_some() {
        sidebar = sidebar.push(binding_panel(i18n, state));
    }
    let sections: Element<'static, Message> = if width >= 720.0 {
        row![main, sidebar.width(316)].spacing(18).into()
    } else {
        column![main, sidebar.width(Length::Fill)].spacing(18).into()
    };
    column![back_button, intro, sections].spacing(22).width(Length::Fill).into()
}
