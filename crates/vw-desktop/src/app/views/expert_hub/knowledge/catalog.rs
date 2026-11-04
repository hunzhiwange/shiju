//! 两级目录均从服务端 total 计算分页；加载时禁用翻页和重复刷新。
use super::super::styles;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_muted_text_style, settings_text_input_style,
};
use crate::app::message::skill_hub::knowledge::{KnowledgeMessage, message};
use crate::app::{
    App, Message,
    i18n::I18n,
    skill_hub::{LoadState, knowledge::KnowledgePage},
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

fn action(label: &'static str, action: Option<KnowledgeMessage>) -> Element<'static, Message> {
    button(text(label).size(13))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(action.map(message))
        .into()
}
fn paging<T>(app: &App, page: &KnowledgePage<T>, bases: bool) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let busy = matches!(page.status, LoadState::Loading { .. });
    let current = page.query.page;
    let pages = page.pages();
    let navigate =
        |n| if bases { KnowledgeMessage::BasesPage(n) } else { KnowledgeMessage::DocumentsPage(n) };
    row![
        action(i18n.__("上一页"), (!busy && current > 1).then(|| navigate(current - 1))),
        text(format!("{} / {} · {} {}", current, pages, i18n.__("总数"), page.total)).size(13),
        action(i18n.__("下一页"), (!busy && current < pages).then(|| navigate(current + 1))),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .wrap()
    .into()
}
fn panel<'a, T>(
    app: &'a App,
    page: &'a KnowledgePage<T>,
    bases: bool,
    items: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let busy = matches!(page.status, LoadState::Loading { .. });
    let retry =
        if bases { KnowledgeMessage::RefreshBases } else { KnowledgeMessage::RefreshDocuments };
    let empty = if bases { "暂无知识库" } else { "暂无文档" };
    let mut content = column![].spacing(8);
    match &page.status {
        LoadState::Loading { .. } | LoadState::Idle => {
            content = content.push(text(i18n.__("加载中…")));
        }
        LoadState::NeedsLogin => {
            content = content.push(text(i18n.__("请先登录识矩账号")));
        }
        LoadState::Error(_) => {
            content = content.push(
                row![
                    text(i18n.__("加载失败，请重试")),
                    action(i18n.__("重试"), Some(retry.clone()))
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            );
        }
        LoadState::Loaded(()) if items.is_empty() => {
            content = content.push(text(i18n.__(empty)));
        }
        LoadState::Loaded(()) => {
            for item in items {
                content = content.push(item);
            }
        }
    }
    let search = text_input(i18n.__(if bases { "搜索知识库" } else { "搜索文档" }), &page.query.q)
        .on_input(move |q| {
            message(if bases {
                KnowledgeMessage::SearchBases(q)
            } else {
                KnowledgeMessage::SearchDocuments(q)
            })
        })
        .style(settings_text_input_style)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .padding(SETTINGS_CONTROL_PADDING);
    column![
        row![
            text(i18n.__(if bases { "知识库" } else { "文档" })).size(16),
            search,
            action(i18n.__("刷新"), (!busy).then_some(retry))
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        scrollable(container(content).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
        paging(app, page, bases),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}
pub(super) fn bases(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.knowledge;
    let items = state
        .bases
        .items
        .iter()
        .map(|base| {
            let selected = state.selected_base.as_deref() == Some(&base.kb_zid);
            let content =
                column![text(&base.name).size(15), text(&base.description).size(12)].spacing(4);
            button(content)
                .padding(10)
                .width(Length::Fill)
                .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
                .on_press(message(KnowledgeMessage::SelectBase(base.kb_zid.clone())))
                .into()
        })
        .collect();
    panel(app, &state.bases, true, items)
}
pub(super) fn documents(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.knowledge;
    if state.selected_base.is_none() {
        return container(text(I18n::new(app.desktop_locale).__("请选择知识库")))
            .center(Length::Fill)
            .into();
    }
    let items = state
        .documents
        .items
        .iter()
        .map(|doc| {
            let mut content = column![
                text(&doc.title).size(15),
                text(&doc.original_filename).size(12).style(settings_muted_text_style)
            ]
            .spacing(4);
            if doc.lifecycle_status != "ready" && doc.lifecycle_status != "active" {
                content = content.push(
                    text(format!(
                        "{}：{}",
                        I18n::new(app.desktop_locale).__("状态"),
                        doc.lifecycle_status
                    ))
                    .size(12)
                    .style(settings_muted_text_style),
                );
            }
            container(
                row![
                    container(content).width(Length::Fill),
                    button(text(I18n::new(app.desktop_locale).__("阅读")))
                        .padding([6, 10])
                        .style(primary_action_btn_style)
                        .on_press_maybe(
                            (!matches!(state.documents.status, LoadState::Loading { .. })).then(
                                || message(KnowledgeMessage::OpenReader {
                                    kb: doc.kb_zid.clone(),
                                    doc: doc.doc_zid.clone()
                                })
                            )
                        ),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .style(styles::card_style)
            .into()
        })
        .collect();
    panel(app, &state.documents, false, items)
}
