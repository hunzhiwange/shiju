//! 展示识矩商店应用或已安装应用的详情信息。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Font, Length};

use super::{mine, styles};
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{ShijuAppDetailTarget, ShijuAppsLoadState};
use crate::app::{App, Message};

fn field(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    value: String,
    monospace: bool,
) -> Element<'static, Message> {
    let value = if monospace && value.is_ascii() {
        text(value).size(12).font(Font::MONOSPACE)
    } else {
        text(value).size(13)
    };
    column![text(i18n.__(label)).size(11).style(styles::muted_text_style), value,].spacing(4).into()
}

fn shell(
    i18n: crate::app::i18n::I18n,
    title: String,
    description: String,
    fields: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    let mut content = column![
        row![
            button(text(i18n.__("返回")).size(13))
                .padding([7, 12])
                .style(styles::quiet_button_style)
                .on_press(Message::ShijuApps(ShijuAppsMessage::CloseDetail)),
            text(title).size(22),
        ]
        .spacing(12),
        text(description).size(13),
    ]
    .spacing(16);
    for item in fields {
        content =
            content.push(container(item).padding(12).width(Length::Fill).style(styles::card_style));
    }
    scrollable(container(content).padding([0, styles::SCROLLABLE_SIDE_PADDING]).width(Length::Fill))
        .direction(Direction::Vertical(
            Scrollbar::new().width(styles::SCROLLBAR_WIDTH).scroller_width(styles::SCROLLBAR_WIDTH),
        ))
        .height(Length::Fill)
        .into()
}

/// 从目标列表索引读取应用；索引失效时展示可返回的错误状态。
pub(super) fn view(app: &App, target: ShijuAppDetailTarget) -> Element<'static, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    match target {
        ShijuAppDetailTarget::Store(index) => {
            let ShijuAppsLoadState::Loaded(catalog) = &app.expert_hub.shiju_apps.store else {
                return shell(
                    i18n,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("应用详情").to_string(),
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("目录数据不可用")
                        .to_string(),
                    Vec::new(),
                );
            };
            let Some(item) = catalog.apps.get(index) else {
                return shell(
                    i18n,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("应用详情").to_string(),
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("应用条目不存在")
                        .to_string(),
                    Vec::new(),
                );
            };
            shell(
                i18n,
                item.name.clone(),
                item.description.clone().unwrap_or_else(|| {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("暂无应用说明").to_string()
                }),
                vec![
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("应用 ID"),
                        item.app_id.clone(),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("版本"),
                        item.version.clone(),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("安装状态"),
                        if item.installed {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("已安装")
                        } else {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("未安装")
                        }
                        .to_string(),
                        false,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("用户 ID"),
                        item.user_id.clone().unwrap_or_else(|| {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("暂无").to_string()
                        }),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("安装实例 ID"),
                        item.installation_id.clone().unwrap_or_else(|| {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("暂无").to_string()
                        }),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("安装时间"),
                        item.installed_at.clone().unwrap_or_else(|| {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("暂无").to_string()
                        }),
                        false,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("权限"),
                        if item.permissions.is_empty() {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("无").to_string()
                        } else {
                            item.permissions.join(" · ")
                        },
                        true,
                    ),
                ],
            )
        }
        ShijuAppDetailTarget::Mine(index) => {
            let ShijuAppsLoadState::Loaded(apps) = &app.expert_hub.shiju_apps.mine else {
                return shell(
                    i18n,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("我的应用详情").to_string(),
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("应用数据不可用")
                        .to_string(),
                    Vec::new(),
                );
            };
            let Some(item) = apps.apps.get(index) else {
                return shell(
                    i18n,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("我的应用详情").to_string(),
                    crate::app::i18n::I18n::new(app.desktop_locale)
                        .__("应用条目不存在")
                        .to_string(),
                    Vec::new(),
                );
            };
            let readiness = if mine::runtime_readiness(item).is_ok() {
                i18n.__("运行就绪").to_string()
            } else {
                i18n.format(
                    "尚未就绪 · 协调状态：%s；运行状态：%s",
                    &[item.reconcile_status.clone().into(), item.runtime_status.clone().into()],
                )
                .expect("valid readiness")
            };
            shell(
                i18n,
                item.name.clone(),
                item.description.clone().unwrap_or_else(|| {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("暂无应用说明").to_string()
                }),
                vec![
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("应用 ID"),
                        item.app_id.clone(),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("安装实例 ID"),
                        item.installation_id.clone(),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("版本"),
                        item.version.clone(),
                        true,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("来源"),
                        item.source.clone(),
                        false,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("运行状态"),
                        readiness,
                        false,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("创建时间"),
                        item.created_at.clone().unwrap_or_else(|| {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("暂无").to_string()
                        }),
                        false,
                    ),
                    field(
                        i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("更新时间"),
                        item.updated_at.clone().unwrap_or_else(|| {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("暂无").to_string()
                        }),
                        false,
                    ),
                ],
            )
        }
    }
}
