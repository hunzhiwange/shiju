use crate::Event as Message;
use crate::browser_widgets::{
    DetailActionStyle, detail_action_button, detail_source_note, empty_state, loading_banner,
};
use crate::catalog::{icon_svg, section_card_style, skill_badge};
use crate::types::SkillsCatalogKind as CatalogSkillKind;
use crate::{BrowserContext, Event};
use desktop_assets::assets::Icon;
use desktop_i18n::settings_extensions::text as tr;
use desktop_ui_controls::system_settings_common::{
    settings_close_button, settings_error_banner, settings_group_heading, settings_modal_card,
    settings_modal_header, settings_modal_overlay, settings_muted_text_style, settings_value_badge,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

pub fn detail_modal_body<'a>(app: BrowserContext<'a>, project_open: bool) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = app.skills_settings;

    if s.detail_loading {
        return loading_banner(tr(locale, "正在通过 gateway 同步技能目录..."));
    }

    if let Some(err) = s.detail_error {
        return settings_error_banner(err);
    }

    if let Some(detail) = s.selected_skill_detail {
        let source_badge = match detail.source.as_str() {
            "workspace" => tr(locale, "项目目录"),
            "ancestor" => tr(locale, "父级目录"),
            "global" => tr(locale, "全局目录"),
            "bundled" => tr(locale, "内置"),
            _ => tr(locale, "来源"),
        };
        let busy = s.loading || s.detail_loading;

        let mut actions = row![].spacing(8).align_y(Alignment::Center);

        if detail.can_install {
            actions = actions.push(detail_action_button(
                locale,
                tr(locale, "安装到项目"),
                Icon::Plus,
                DetailActionStyle::Primary,
                (!busy && project_open)
                    .then_some(Event::SkillsInstallBuiltInRequested(detail.id.clone())),
            ));
        }

        if detail.can_toggle {
            actions = actions.push(detail_action_button(
                locale,
                if detail.enabled { tr(locale, "禁用") } else { tr(locale, "启用") },
                if detail.enabled { Icon::EyeSlash } else { Icon::Eye },
                DetailActionStyle::Secondary,
                (!busy).then_some(Event::SkillsSetEnabledRequested {
                    skill_id: detail.id.clone(),
                    enabled: !detail.enabled,
                }),
            ));
        }

        if detail.can_delete {
            actions = actions.push(detail_action_button(
                locale,
                tr(locale, "删除"),
                Icon::Trash,
                DetailActionStyle::Danger,
                (!busy).then_some(Event::SkillsDeleteRequested(detail.id.clone())),
            ));
        }

        let summary = container(
            column![
                row![
                    column![
                        text(&detail.title).size(17).wrapping(iced::widget::text::Wrapping::Glyph),
                        text(&detail.description)
                            .size(12)
                            .wrapping(iced::widget::text::Wrapping::Glyph)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    skill_badge(
                        match detail.kind {
                            CatalogSkillKind::Recommended => tr(locale, "推荐"),
                            CatalogSkillKind::System => tr(locale, "内置"),
                            CatalogSkillKind::Personal => tr(locale, "本地"),
                        },
                        matches!(detail.kind, CatalogSkillKind::Recommended),
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    settings_value_badge(source_badge),
                    settings_value_badge(if detail.installed {
                        tr(locale, "已安装")
                    } else {
                        tr(locale, "未安装")
                    }),
                    settings_value_badge(if detail.enabled {
                        tr(locale, "已启用")
                    } else {
                        tr(locale, "已禁用")
                    }),
                    skill_badge(detail.document_name.clone(), true),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
                detail_source_note(locale, detail.source_path.as_deref()),
            ]
            .spacing(10),
        )
        .padding([16, 18])
        .width(Length::Fill)
        .style(section_card_style);

        let document = container(
            column![
                row![icon_svg(Icon::FileText, 14.0), text(&detail.document_name).size(13),]
                    .spacing(8)
                    .align_y(Alignment::Center),
                scrollable(
                    container(
                        text(&detail.document_content)
                            .size(12)
                            .width(Length::Fill)
                            .wrapping(iced::widget::text::Wrapping::Glyph),
                    )
                    .padding([0, 7])
                    .width(Length::Fill),
                )
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding([16, 18])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(section_card_style);

        let content = column![summary]
            .push(container(actions.wrap()).width(Length::Fill))
            .push(document)
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        return container(content).width(Length::Fill).height(Length::Fill).into();
    }

    empty_state(
        tr(locale, "加载技能详情中"),
        tr(
            locale,
            "点击左侧技能卡片后，这里会显示 SKILL.md 或 SKILL.toml 内容，并提供启用、禁用、删除操作。",
        ),
    )
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view_overlays<'a, M: Clone + 'static>(
    app: BrowserContext<'a>,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    let locale = app.desktop_locale;
    let s = app.skills_settings;
    if s.selected_skill_id.is_none() {
        return base;
    }

    let close_message = message(Event::SkillsDetailClosed);
    let title = s
        .selected_skill_detail
        .as_ref()
        .map(|detail| detail.title.as_str())
        .unwrap_or(tr(locale, "技能详情"));
    let subtitle = s
        .selected_skill_detail
        .as_ref()
        .map(|detail| match detail.source.as_str() {
            "workspace" => tr(locale, "项目目录技能"),
            "ancestor" => tr(locale, "父级目录技能"),
            "global" => tr(locale, "全局目录技能"),
            "bundled" => tr(locale, "内置技能"),
            _ => tr(locale, "技能"),
        })
        .unwrap_or(tr(locale, "读取技能文档中"));

    let card = settings_modal_card(
        column![
            settings_modal_header(
                settings_group_heading(title, subtitle),
                settings_close_button(close_message.clone()),
            ),
            detail_modal_body(app, app.project_path.is_some()).map(message),
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .style(super::modal_card_style)
    .width(Length::Fixed(860.0))
    .height(Length::Fixed(640.0));

    settings_modal_overlay(Some(base), close_message, container(card).padding(12))
}
