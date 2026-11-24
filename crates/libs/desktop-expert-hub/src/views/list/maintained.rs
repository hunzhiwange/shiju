use super::{
    cards, catalog, category_button, filter_strip, heading, mine_tag_filter, pagination_messages,
    search_box,
};
use crate::message::skill_hub::{SkillCatalogMessage, SkillHubMessage};
use crate::views::{Message, ViewInput};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_skill_hub::skill_hub::{
    LoadState, MaintainedSkillAction, MineFilterMenu, MineSkillStatus, Page, TenantSkill,
};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_select, settings_text_input,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length};

pub(super) fn view(app: &ViewInput<'_>, width: f32) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = app.expert_hub;
    let mut status_tabs = row![].spacing(2);
    for status in std::iter::once(None).chain(MineSkillStatus::ALL.into_iter().map(Some)) {
        let selected = match status {
            None => state.mine_statuses_draft.len() != 1,
            Some(status) => state.mine_statuses_draft.as_slice() == [status],
        };
        status_tabs = status_tabs.push(category_button(
            i18n.__(status.map_or("全部", MineSkillStatus::label)).to_string(),
            selected,
            true,
            SkillHubMessage::Catalog(SkillCatalogMessage::MineStatusSelected(status)),
        ));
    }
    let status_tabs = container(status_tabs.wrap()).padding(4).style(filter_strip);
    let search = search_box(
        i18n,
        &state.mine_filter.keyword_draft,
        (width - 16.0).clamp(1.0, 320.0),
        SkillHubMessage::FilterKeywordChanged,
        Some(SkillHubMessage::FiltersSubmitted),
    );
    let filters = row![
        search,
        mine_tag_filter(
            app,
            MineFilterMenu::Tags,
            &state.mine_tag_ids_draft,
            &state.platform_tag_catalog,
            "选择平台标签",
            SkillHubMessage::ToggleMineTagFilterMenu,
            SkillHubMessage::MineTagToggled,
        ),
        mine_tag_filter(
            app,
            MineFilterMenu::CompanyTags,
            &state.mine_company_tag_ids_draft,
            &state.company_tag_catalog,
            "选择公司标签",
            SkillHubMessage::ToggleMineCompanyTagFilterMenu,
            SkillHubMessage::MineCompanyTagToggled,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let toolbar: Element<'static, Message> = if width >= 1060.0 {
        row![status_tabs, Space::new().width(Length::Fill), filters]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        column![status_tabs, filters.wrap()].spacing(10).into()
    };

    let content = match &state.mine {
        LoadState::Idle | LoadState::Loading { .. } => super::loading_view(),
        LoadState::NeedsLogin => super::needs_login_view(i18n),
        LoadState::Error(error) => super::error_view(i18n, &error.render(i18n)),
        LoadState::Loaded(page) => table(i18n, page, width),
    };
    column![
        heading(i18n, "我创建的技能", "集中管理技能内容、版本、可见范围和发布进度。"),
        toolbar,
        content,
    ]
    .spacing(20)
    .width(Length::Fill)
    .into()
}

fn table(i18n: I18n, page: &Page<TenantSkill>, width: f32) -> Element<'static, Message> {
    let table_width = width.max(900.0);
    let widths = [0.32, 0.095, 0.095, 0.113, 0.103, 0.274]
        .map(|fraction| Length::Fixed(table_width * fraction));
    let labels = ["名称", "平台标签", "公司标签", "可见范围", "状态", "操作"];
    let mut headers = row![].height(40);
    for (index, (label, width)) in labels.into_iter().zip(widths).enumerate() {
        let alignment = if index == 5 { Alignment::End } else { Alignment::Start };
        headers = headers.push(
            container(text(i18n.__(label)).size(12).font(cards::bold()))
                .padding([0, 9])
                .width(width)
                .height(Length::Fill)
                .align_x(alignment)
                .align_y(Alignment::Center),
        );
    }
    let mut rows = column![
        container(headers).style(|theme| container::Style {
            background: Some(
                if cards::dark(theme) {
                    theme.extended_palette().background.strong.color
                } else {
                    Color::from_rgb8(243, 240, 233)
                }
                .into()
            ),
            ..Default::default()
        }),
        cards::line(),
    ];
    for (index, skill) in page.items.iter().enumerate() {
        rows = rows.push(table_row(i18n, skill, widths, index % 2 == 1)).push(cards::line());
    }
    if page.items.is_empty() {
        rows = rows.push(
            container(text(i18n.__("暂无我的技能")).size(14).style(cards::muted_text))
                .height(90)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        );
        rows = rows.push(cards::line());
    }
    let table = scrollable(rows.width(Length::Fixed(table_width)))
        .direction(Direction::Horizontal(Scrollbar::new().width(4).scroller_width(4).spacing(4)))
        .width(Length::Fill);
    container(column![table, table_footer(i18n, page, width)].width(Length::Fill))
        .style(|theme| cards::panel(theme, 8.0))
        .clip(true)
        .into()
}

fn table_row(
    i18n: I18n,
    skill: &TenantSkill,
    widths: [Length; 6],
    alternate: bool,
) -> Element<'static, Message> {
    let visibility = skill.tenant_visibility.as_ref().map(|scope| scope.effective_mode.as_str());
    let (visibility, visibility_color) = match visibility {
        Some("tenant_open") => ("全员", Color::from_rgb8(74, 109, 52)),
        Some("department") => ("指定部门", Color::from_rgb8(116, 103, 70)),
        Some("user") => ("指定员工", Color::from_rgb8(116, 103, 70)),
        Some("mixed") => ("指定部门和员工", Color::from_rgb8(116, 103, 70)),
        Some("disabled") => (i18n.__("不可见"), Color::from_rgb8(163, 55, 52)),
        Some("unset") => ("未设置", Color::from_rgb8(163, 55, 52)),
        _ => ("—", Color::from_rgb8(95, 107, 130)),
    };
    let (status, status_color) = match skill.status.as_str() {
        "published" => ("已发布", Color::from_rgb8(74, 109, 52)),
        "draft" => ("草稿", Color::from_rgb8(95, 107, 130)),
        "pending_review" => ("待审核", Color::from_rgb8(57, 95, 142)),
        "unpublished" => ("已下架", Color::from_rgb8(116, 103, 70)),
        _ => (skill.status.as_str(), Color::from_rgb8(95, 107, 130)),
    };
    let id = skill.id;
    let mut actions = row![
        cards::icon_button(Icon::Eye, i18n.__("详情"), Some(super::detail_action_message(id))),
        cards::icon_button(
            Icon::Pencil,
            i18n.__("编辑"),
            Some(catalog(SkillCatalogMessage::EditMaintainedSkill(id)))
        ),
        cards::icon_button(
            Icon::Funnel,
            i18n.__("可见范围"),
            Some(catalog(SkillCatalogMessage::OpenMaintainedSkillAction(
                id,
                MaintainedSkillAction::Visibility
            )))
        ),
    ]
    .spacing(8);
    if skill.status == "published" || skill.status == "draft" {
        let published = skill.status == "published";
        let kind = if published {
            MaintainedSkillAction::Unpublish
        } else {
            MaintainedSkillAction::Delete
        };
        actions = actions.push(cards::icon_button(
            if published { Icon::CloudDownload } else { Icon::Trash },
            i18n.__(kind.label()),
            Some(catalog(SkillCatalogMessage::OpenMaintainedSkillAction(id, kind))),
        ));
    }
    let name = row![
        cards::avatar(id, &skill.name, 36.0),
        column![
            Element::from(SingleLineLabel::new(skill.name.clone(), 14.0).font(cards::bold())),
            text(format!("ID {id}")).font(iced::Font::MONOSPACE).size(12).style(cards::muted_text),
        ]
        .spacing(2)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let tags = |tags: &[desktop_skill_hub::skill_hub::SkillTag]| {
        let label = super::tag_summary(tags);
        text(if label.is_empty() { "—".into() } else { label })
            .size(12)
            .style(cards::muted_text)
            .wrapping(text::Wrapping::None)
    };
    let values: [Element<'static, Message>; 6] = [
        name.into(),
        tags(&skill.tags).into(),
        tags(&skill.company_tags).into(),
        pill(i18n.__(visibility).to_string(), visibility_color),
        pill(i18n.__(status).to_string(), status_color),
        actions.into(),
    ];
    let mut cells = row![].height(52);
    for (index, (value, width)) in values.into_iter().zip(widths).enumerate() {
        cells = cells.push(
            container(value)
                .padding([0, 9])
                .width(width)
                .height(Length::Fill)
                .align_x(if index == 5 { Alignment::End } else { Alignment::Start })
                .align_y(Alignment::Center)
                .clip(true),
        );
    }
    container(cells)
        .style(move |theme| container::Style {
            background: Some(
                if alternate && !cards::dark(theme) {
                    Color::from_rgb8(251, 250, 248)
                } else {
                    cards::surface(theme)
                }
                .into(),
            ),
            ..Default::default()
        })
        .into()
}

fn pill(label: String, tone: Color) -> Element<'static, Message> {
    container(text(label).size(11).font(cards::bold()))
        .padding([5, 8])
        .style(move |theme| {
            let tint = if cards::dark(theme) { Color { a: 1.0, ..tone } } else { tone };
            container::Style {
                background: Some(
                    tint.scale_alpha(if cards::dark(theme) { 0.33 } else { 0.08 }).into(),
                ),
                text_color: Some(if cards::dark(theme) { theme.palette().text } else { tint }),
                border: Border { radius: 20.0.into(), width: 1.0, color: tint.scale_alpha(0.22) },
                ..Default::default()
            }
        })
        .into()
}

fn table_footer(i18n: I18n, page: &Page<TenantSkill>, width: f32) -> Element<'static, Message> {
    let first = u64::from(page.page.saturating_sub(1)) * u64::from(page.page_size) + 1;
    let last = (first + page.items.len().saturating_sub(1) as u64).min(page.total);
    let summary = if page.items.is_empty() {
        i18n.format("共 %s 条", &[page.total.to_string().into()])
    } else {
        i18n.format(
            "共 %s 条，当前显示 %s–%s",
            &[page.total.to_string().into(), first.to_string().into(), last.to_string().into()],
        )
    }
    .expect("valid page summary");
    let options = [10, 20, 50, 100];
    let size = settings_select(options, Some(page.page_size), |size| {
        catalog(SkillCatalogMessage::MinePageSizeSelected(size))
    })
    .width(72)
    .height(32.0);
    let (previous, next) = pagination_messages(page);
    let controls = row![
        text(i18n.__("每页")).size(13).style(cards::muted_text),
        size,
        cards::icon_button(Icon::ChevronLeft, i18n.__("上一页"), previous),
        container(text(page.page.to_string()).size(13))
            .center(Length::Fixed(32.0))
            .style(|theme| cards::panel(theme, 6.0)),
        cards::icon_button(Icon::ChevronRight, i18n.__("下一页"), next),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let summary = text(summary).size(13).font(cards::bold()).style(cards::muted_text);
    let content: Element<'static, Message> = if width >= 560.0 {
        row![summary, Space::new().width(Length::Fill), controls]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        column![summary, controls].spacing(8).into()
    };
    container(content).padding([12, 16]).width(Length::Fill).into()
}

pub(super) fn action_dialog(app: &ViewInput<'_>) -> Option<(Message, Element<'static, Message>)> {
    let action = app.expert_hub.maintained_action.as_ref()?;
    let i18n = I18n::new(app.desktop_locale);
    let pending = action.pending.is_some();
    let close = catalog(SkillCatalogMessage::CloseMaintainedSkillAction);
    let mut form =
        column![text(action.name.clone()).size(14).style(cards::muted_text),].spacing(10);
    if action.kind == MaintainedSkillAction::Visibility {
        form = form.push(
            checkbox(action.all)
                .label(i18n.__("全员"))
                .style(desktop_ui_controls::system_settings_common::settings_checkbox_style)
                .on_toggle_maybe((!pending).then_some(|value| {
                    catalog(SkillCatalogMessage::MaintainedVisibilityAllChanged(value))
                })),
        );
        if !action.all {
            form = form.push(text(i18n.__("部门 ID（逗号分隔）")).size(13)).push(
                settings_text_input(i18n.__("部门 ID"), &action.department_ids).on_input_maybe(
                    (!pending).then_some(|value| {
                        catalog(SkillCatalogMessage::MaintainedVisibilityDepartmentsChanged(value))
                    }),
                ),
            );
            form = form.push(text(i18n.__("员工 ID（逗号分隔）")).size(13)).push(
                settings_text_input(i18n.__("员工 ID"), &action.user_ids).on_input_maybe(
                    (!pending).then_some(|value| {
                        catalog(SkillCatalogMessage::MaintainedVisibilityUsersChanged(value))
                    }),
                ),
            );
        }
    }
    if let Some(error) = &action.error {
        form = form.push(text(error.render(i18n)).size(13).style(text::danger));
    }
    let save = button(text(i18n.__(if pending { "处理中…" } else { "确认" })).size(14))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(
            (!pending).then_some(catalog(SkillCatalogMessage::ConfirmMaintainedSkillAction)),
        );
    let cancel = button(text(i18n.__("取消")).size(14))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe((!pending).then_some(close.clone()));
    let card = settings_modal_card(
        column![
            row![
                text(i18n.__(action.kind.label())).size(18).font(cards::bold()),
                Space::new().width(Length::Fill),
                settings_close_button(close.clone())
            ]
            .align_y(Alignment::Center),
            form,
            row![Space::new().width(Length::Fill), cancel, save].spacing(8),
        ]
        .spacing(16),
    )
    .width(460);
    Some((close, card.into()))
}
