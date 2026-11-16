use super::*;
use iced::widget::column;

pub(super) fn build_documents_table(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let filtered_page = filtered_documents_page(app);
    let all_visible_selected = !filtered_page.documents.is_empty()
        && filtered_page
            .documents
            .iter()
            .all(|document| app.knowledge.selected_document_ids.contains(&document.id));
    let selected_dataset_chunking = app
        .knowledge
        .selected_dataset()
        .map(|dataset| &dataset.chunking_mode)
        .unwrap_or(&KnowledgeChunkingMode::General);
    let header = row![
        container(document_selection_button(
            all_visible_selected,
            desktop_i18n::I18n::new(app.desktop_locale).__("全选"),
            Some(Message::Knowledge(KnowledgeToolMessage::ToggleSelectAllVisibleDocuments)),
        ))
        .center_x(Length::Fixed(28.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("序号"))
            .size(12)
            .width(Length::Fixed(42.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("名称"))
            .size(12)
            .width(Length::Fill),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("分段模式"))
            .size(12)
            .width(Length::Fixed(86.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("字符数"))
            .size(12)
            .width(Length::Fixed(76.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("召回次数"))
            .size(12)
            .width(Length::Fixed(76.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("更新时间"))
            .size(12)
            .width(Length::Fixed(150.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("状态"))
            .size(12)
            .width(Length::Fixed(74.0)),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("操作"))
            .size(12)
            .width(Length::Fixed(DOCUMENT_ACTIONS_WIDTH)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut rows = column![].spacing(6);
    for (index, document) in filtered_page.documents.iter().enumerate() {
        rows = rows.push(document_row_with_index(
            app,
            document,
            filtered_page.start_index + index + 1,
            selected_dataset_chunking,
        ));
    }
    if app.knowledge.loading_documents {
        rows = rows
            .push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("正在加载文档")));
    } else if app.knowledge.documents.is_empty() {
        rows =
            rows.push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("暂无文档")));
    } else if filtered_page.total_matches == 0 {
        rows = rows
            .push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("没有匹配文档")));
    }

    let mut content = column![
        build_documents_toolbar(app),
        container(header).padding([8, 8]).style(chrome_style),
        knowledge_scrollable(rows).height(Length::Fill)
    ]
    .spacing(10)
    .height(Length::Fill);
    if !app.knowledge.selected_document_ids.is_empty() {
        content = content.push(build_document_bulk_toolbar(app, &filtered_page.documents));
    }
    if !app.knowledge.loading_documents && filtered_page.total_matches > 0 {
        content = content.push(build_document_pagination(
            i18n,
            filtered_page.current_page,
            filtered_page.total_pages,
            filtered_page.total_matches,
            filtered_page.page_size,
        ));
    }

    let table: Element<'_, Message> =
        container(content).padding([18, 0]).width(Length::Fill).into();

    if app.knowledge.document_status_menu_open {
        let dismiss = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseDocumentStatusMenu));
        let menu = container(build_document_status_menu(app))
            .padding(iced::Padding { top: 70.0, right: 0.0, bottom: 0.0, left: 36.0 })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        iced::widget::stack![table, dismiss, menu].width(Length::Fill).height(Length::Fill).into()
    } else {
        table
    }
}

pub(super) fn build_document_status_menu(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let mut list = column![].spacing(2);
    for option in document_status_filter_options(i18n) {
        let checked = app.knowledge.document_status_filter.contains(&option.status);
        let mark = if checked { Icon::CheckSquare } else { Icon::Square };
        let entry = button(
            row![themed_inline_icon(mark, 14.0), text(option.label).size(13).width(Length::Fill),]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press(Message::Knowledge(KnowledgeToolMessage::ToggleDocumentStatusFilter(
            option.status.clone(),
        )))
        .padding([8, 10])
        .width(Length::Fill)
        .style(knowledge_status_menu_entry_style);
        list = list.push(entry);
    }

    let clear = button(
        text(desktop_i18n::I18n::new(app.desktop_locale).__("清除筛选"))
            .size(12)
            .width(Length::Fill),
    )
    .on_press(Message::Knowledge(KnowledgeToolMessage::ClearDocumentStatusFilter))
    .padding([8, 10])
    .width(Length::Fill)
    .style(knowledge_status_menu_entry_style);

    container(column![list, clear].spacing(4))
        .padding(8)
        .width(Length::Fixed(180.0))
        .style(knowledge_status_menu_style)
        .into()
}

pub(super) fn knowledge_status_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_palette(theme) {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::WHITE.scale_alpha(0.98)
        })),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_palette(theme) {
                0.56
            } else {
                0.18
            }),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_palette(theme) { 0.32 } else { 0.16 }),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub(super) fn knowledge_status_menu_entry_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    iced::widget::button::Style {
        background: hovered.then(|| palette.background.weak.color.scale_alpha(0.6).into()),
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub(super) fn build_documents_toolbar(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected_count = app.knowledge.document_status_filter.len();
    let summary = match selected_count {
        0 => desktop_i18n::I18n::new(app.desktop_locale).__("全部状态").to_string(),
        1 => document_status_filter_options(i18n)
            .into_iter()
            .find(|option| app.knowledge.document_status_filter.contains(&option.status))
            .map(|option| option.label.to_string())
            .unwrap_or_else(|| {
                desktop_i18n::I18n::new(app.desktop_locale).__("全部状态").to_string()
            }),
        count => desktop_i18n::I18n::new(app.desktop_locale)
            .format("已选 %s 项", &[(count).to_string().into()])
            .expect("valid display template"),
    };
    let menu_open = app.knowledge.document_status_menu_open;
    let status_filter = button(
        row![
            text(summary).size(12).width(Length::Fixed(96.0)),
            themed_inline_icon(if menu_open { Icon::ChevronUp } else { Icon::ChevronDown }, 11.0),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(Message::Knowledge(KnowledgeToolMessage::ToggleDocumentStatusMenu))
    .padding([7, 10])
    .style(knowledge_toolbar_button_style);

    let search = text_input(
        desktop_i18n::I18n::new(app.desktop_locale).__("搜索文档"),
        &app.knowledge.document_search_query,
    )
    .on_input(|value| Message::Knowledge(KnowledgeToolMessage::DocumentSearchChanged(value)))
    .padding([9, 10])
    .size(13)
    .style(settings_text_input_style)
    .width(Length::Fill);

    container(
        row![
            container(
                row![themed_inline_icon(Icon::Sliders, 13.0), status_filter]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding([0, 2]),
            container(search).width(Length::Fill),
            button(
                row![
                    themed_inline_icon(Icon::SquareHalf, 13.0),
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("反选")).size(13)
                ]
                .spacing(8)
            )
            .on_press(Message::Knowledge(KnowledgeToolMessage::InvertVisibleDocumentSelection))
            .padding([9, 12])
            .style(knowledge_toolbar_button_style),
            button(
                row![
                    themed_inline_icon(Icon::ArrowRepeat, 13.0),
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("刷新")).size(13)
                ]
                .spacing(8)
            )
            .on_press_maybe(
                (!app.knowledge.is_busy())
                    .then_some(Message::Knowledge(KnowledgeToolMessage::Refresh))
            )
            .padding([9, 12])
            .style(knowledge_toolbar_button_style),
            button(
                row![
                    primary_action_icon(Icon::FileEarmarkPlus, 13.0),
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("新增文件")).size(13)
                ]
                .spacing(8)
            )
            .on_press(Message::Knowledge(KnowledgeToolMessage::SelectTab(
                KnowledgeDetailTab::AddDocument,
            )))
            .padding([9, 12])
            .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([12, 12])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

pub(super) fn filtered_documents_page(app: ViewInput<'_>) -> FilteredDocumentsPage<'_> {
    let filtered_ids = app.knowledge.filtered_document_ids();
    let visible_ids = app.knowledge.visible_document_ids();
    let total_matches = filtered_ids.len();
    let page_size = document_page_size(app);
    let total_pages = document_total_pages(total_matches, page_size);
    let current_page = app.knowledge.document_page.clamp(1, total_pages);
    let start_index = (current_page - 1) * page_size;
    let documents = visible_ids
        .iter()
        .filter_map(|document_id| {
            app.knowledge.documents.iter().find(|document| &document.id == document_id)
        })
        .collect::<Vec<_>>();
    FilteredDocumentsPage {
        documents,
        total_matches,
        current_page,
        total_pages,
        page_size,
        start_index,
    }
}

pub(super) fn document_page_size(app: ViewInput<'_>) -> usize {
    match app.knowledge.document_page_size {
        10 | 20 | 30 | 40 | 50 | 100 => app.knowledge.document_page_size,
        _ => DOCUMENTS_PER_PAGE,
    }
}

pub(super) fn document_total_pages(total_items: usize, page_size: usize) -> usize {
    total_items.div_ceil(page_size.max(1)).max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DocumentPageItem {
    Page(usize),
    Ellipsis,
}

pub(super) fn document_pagination_items(current_page: usize, total_pages: usize) -> Vec<DocumentPageItem> {
    if total_pages <= 7 {
        return (1..=total_pages).map(DocumentPageItem::Page).collect();
    }

    if current_page <= 3 {
        return vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Page(2),
            DocumentPageItem::Page(3),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(total_pages - 2),
            DocumentPageItem::Page(total_pages - 1),
            DocumentPageItem::Page(total_pages),
        ];
    }

    if current_page >= total_pages.saturating_sub(2) {
        return vec![
            DocumentPageItem::Page(1),
            DocumentPageItem::Page(2),
            DocumentPageItem::Page(3),
            DocumentPageItem::Ellipsis,
            DocumentPageItem::Page(total_pages - 2),
            DocumentPageItem::Page(total_pages - 1),
            DocumentPageItem::Page(total_pages),
        ];
    }

    vec![
        DocumentPageItem::Page(1),
        DocumentPageItem::Ellipsis,
        DocumentPageItem::Page(current_page - 1),
        DocumentPageItem::Page(current_page),
        DocumentPageItem::Page(current_page + 1),
        DocumentPageItem::Ellipsis,
        DocumentPageItem::Page(total_pages),
    ]
}

pub(super) fn build_document_pagination(
    i18n: desktop_i18n::I18n,
    current_page: usize,
    total_pages: usize,
    total_matches: usize,
    page_size: usize,
) -> Element<'static, Message> {
    let mut pages = row![document_page_nav_button(i18n.__("上一页"), current_page.checked_sub(1)),]
        .spacing(6)
        .align_y(Alignment::Center);

    for item in document_pagination_items(current_page, total_pages) {
        pages = pages.push(match item {
            DocumentPageItem::Page(page) => document_page_button(page, page == current_page),
            DocumentPageItem::Ellipsis => {
                container(text("...").size(12).style(settings_muted_text_style))
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(32.0))
                    .center_x(Length::Fixed(28.0))
                    .center_y(Length::Fixed(32.0))
                    .into()
            }
        });
    }

    let next_page = (current_page < total_pages).then_some(current_page + 1);
    pages = pages.push(document_page_nav_button(i18n.__("下一页"), next_page));

    container(
        row![
            row![
                text(
                    i18n.format("共 %s 个文档", &[(total_matches).to_string().into()])
                        .expect("valid display template")
                )
                .size(12)
                .style(settings_muted_text_style),
                text(i18n.__("每页")).size(12).style(settings_muted_text_style),
                document_page_size_selector(page_size),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
            pages.wrap(),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap(),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn document_page_size_selector(page_size: usize) -> Element<'static, Message> {
    settings_select(DOCUMENT_PAGE_SIZE_OPTIONS, Some(page_size), |size| {
        Message::Knowledge(KnowledgeToolMessage::DocumentPageSizeChanged(size))
    })
    .width(Length::Fixed(86.0))
    .into()
}

pub(super) fn document_page_button(page: usize, active: bool) -> Element<'static, Message> {
    button(
        container(text(page.to_string()).size(12).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press_maybe(
        (!active).then_some(Message::Knowledge(KnowledgeToolMessage::DocumentPageChanged(page))),
    )
    .padding(0)
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .style(move |theme: &Theme, status| segment_style(theme, status, active))
    .into()
}

pub(super) fn document_page_nav_button(label: &'static str, page: Option<usize>) -> Element<'static, Message> {
    button(
        container(text(label).size(12).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press_maybe(
        page.map(|page| Message::Knowledge(KnowledgeToolMessage::DocumentPageChanged(page))),
    )
    .padding(0)
    .width(Length::Fixed(62.0))
    .height(Length::Fixed(32.0))
    .style(knowledge_toolbar_button_style)
    .into()
}
