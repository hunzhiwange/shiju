use super::*;
use iced::widget::column;

pub(super) fn build_detail<'a>(app: ViewInput<'a>, size: Size) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let Some(dataset) = app.knowledge.selected_dataset() else {
        return settings_panel(
            column![
                text(desktop_i18n::I18n::new(app.desktop_locale).__("选择或创建一个知识库"))
                    .size(18),
                text(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .__("左侧列表会显示全部知识库。")
                )
                .size(13)
                .style(settings_muted_text_style),
            ]
            .spacing(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center),
        )
        .into();
    };

    let header = row![
        column![
            text(&dataset.name).size(18),
            row![
                settings_value_badge(chunking_label(&dataset.chunking_mode)),
                settings_value_badge(indexing_label(i18n, &dataset.indexing_mode)),
                settings_value_badge(retrieval_label(i18n, &dataset.retrieval_mode)),
                settings_value_badge(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .format("%s 文档", &[(dataset.document_count).to_string().into()])
                        .expect("valid display template")
                ),
                settings_value_badge(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .format("%s 分段", &[(dataset.chunk_count).to_string().into()])
                        .expect("valid display template")
                ),
            ]
            .spacing(6),
        ]
        .spacing(8)
        .width(Length::Fill),
        button(centered_header_action_content(
            Icon::Back,
            desktop_i18n::I18n::new(app.desktop_locale).__("返回列表")
        ))
        .on_press(Message::Knowledge(KnowledgeToolMessage::ShowDatasetList))
        .padding(0)
        .width(Length::Fixed(124.0))
        .height(Length::Fixed(40.0))
        .style(rounded_action_btn_style),
        button(centered_header_action_content(
            Icon::Trash,
            desktop_i18n::I18n::new(app.desktop_locale).__("删除")
        ))
        .on_press_maybe(
            (!app.knowledge.deleting)
                .then_some(Message::Knowledge(KnowledgeToolMessage::OpenDeleteDatasetConfirm))
        )
        .padding(0)
        .width(Length::Fixed(96.0))
        .height(Length::Fixed(40.0))
        .style(danger_action_btn_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let tabs = build_tabs(app);
    let body = match app.knowledge.active_tab {
        KnowledgeDetailTab::Documents => build_documents_tab(app, size),
        KnowledgeDetailTab::AddDocument => build_add_document_tab(app),
        KnowledgeDetailTab::Retrieval => build_retrieval_tab(app),
        KnowledgeDetailTab::Settings => build_settings_tab(app, dataset),
    };

    let detail: Element<'_, Message> = column![settings_panel(header), tabs, body]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    if app.knowledge.active_tab == KnowledgeDetailTab::Documents
        && app.knowledge.document_preview_open
    {
        iced::widget::stack![detail, build_document_preview_drawer(app)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        detail
    }
}

pub(super) fn build_tabs(app: ViewInput<'_>) -> Element<'_, Message> {
    let mut tabs = row![].spacing(8).align_y(Alignment::Center);
    for tab in KnowledgeDetailTab::ALL {
        tabs = tabs.push(segment_button(
            tab.title(),
            app.knowledge.active_tab == tab,
            Message::Knowledge(KnowledgeToolMessage::SelectTab(tab)),
        ));
    }
    settings_panel(tabs).into()
}

pub(super) fn build_documents_tab<'a>(app: ViewInput<'a>, _size: Size) -> Element<'a, Message> {
    build_documents_table(app)
}

pub(super) fn build_add_document_tab(app: ViewInput<'_>) -> Element<'_, Message> {
    build_upload_panel(app)
}

pub(super) fn build_upload_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let body = match app.knowledge.document_input_tab {
        KnowledgeDocumentInputTab::Files => build_file_import_panel(app),
        KnowledgeDocumentInputTab::Text => build_text_document_panel(app),
    };

    settings_panel(
        knowledge_scrollable(
            column![build_upload_heading(app), build_document_input_tabs(app), body,]
                .spacing(14)
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}

pub(super) fn build_upload_heading(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let title = i18n.__("添加文档");
    responsive(move |size| {
        let heading = settings_hover_help_label(
            i18n,
            app.app_theme,
            title,
            text(title).size(14),
            i18n.__("支持拖拽文件上传，或直接粘贴长文本入库。"),
        );
        let formats = row![
            settings_value_badge("TXT"),
            settings_value_badge("MD"),
            settings_value_badge("MDX"),
            settings_value_badge("PDF"),
        ]
        .spacing(6);
        if size.width < 360.0 {
            column![heading.wrap(), formats.wrap()].spacing(6).into()
        } else {
            row![heading.width(Length::Fill).wrap(), formats]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
        }
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn build_document_input_tabs(app: ViewInput<'_>) -> Element<'_, Message> {
    let mut tabs = row![].spacing(8).align_y(Alignment::Center);
    for tab in KnowledgeDocumentInputTab::ALL {
        tabs = tabs.push(segment_button(
            tab.title(),
            app.knowledge.document_input_tab == tab,
            Message::Knowledge(KnowledgeToolMessage::SelectDocumentInputTab(tab)),
        ));
    }
    tabs.into()
}

pub(super) fn build_file_import_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected = app.knowledge.file_imports.as_slice();
    let selected_count = selected.len();
    let has_selection = selected_count > 0;
    let dragging_any_file =
        app.files_dragging;
    let drop_hovered = app.knowledge.file_drop_hovered && dragging_any_file;
    let drop_title = match selected {
        [] if drop_hovered => {
            desktop_i18n::I18n::new(app.desktop_locale).__("释放鼠标，开始准备上传").to_string()
        }
        [] => desktop_i18n::I18n::new(app.desktop_locale).__("拖拽文件到这里上传").to_string(),
        [file] => file.filename.clone(),
        _ => desktop_i18n::I18n::new(app.desktop_locale)
            .format("已选择 %s 个文件", &[(selected_count).to_string().into()])
            .expect("valid display template"),
    };
    let drop_meta = match selected {
        [] if drop_hovered => desktop_i18n::I18n::new(app.desktop_locale)
            .__("支持批量拖入 TXT / MD / MDX / PDF")
            .to_string(),
        [] => desktop_i18n::I18n::new(app.desktop_locale)
            .__("支持批量拖拽或点击选择 TXT / MD / MDX / PDF 文件")
            .to_string(),
        _ => file_imports_summary(i18n, selected),
    };
    let can_pick = !app.knowledge.selecting_import_file && !app.knowledge.importing_file;
    let can_import = has_selection
        && app.knowledge.selected_dataset_id.is_some()
        && !app.knowledge.importing_file;
    let pick_label = if app.knowledge.selecting_import_file {
        desktop_i18n::I18n::new(app.desktop_locale).__("选择中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("选择文件")
    };
    let manual_pick_label = if app.knowledge.selecting_import_file {
        desktop_i18n::I18n::new(app.desktop_locale).__("选择中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("手动选择文件")
    };
    let import_label = if app.knowledge.importing_file {
        desktop_i18n::I18n::new(app.desktop_locale).__("上传中").to_string()
    } else if selected_count > 1 {
        desktop_i18n::I18n::new(app.desktop_locale)
            .format("上传 %s 个文件", &[(selected_count).to_string().into()])
            .expect("valid display template")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("上传入库").to_string()
    };

    let mut actions = row![
        button(
            row![icon_svg(Icon::FileText, 14.0), text(pick_label).size(13)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press_maybe(
            can_pick.then_some(Message::Knowledge(KnowledgeToolMessage::PickImportFile))
        )
        .padding([9, 12])
        .style(knowledge_toolbar_button_style),
        button(
            row![primary_action_icon(Icon::CloudUpload, 14.0), text(import_label.clone()).size(13)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press_maybe(
            can_import.then_some(Message::Knowledge(KnowledgeToolMessage::ImportSelectedFile))
        )
        .padding([9, 12])
        .style(primary_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if has_selection {
        actions = actions.push(
            button(icon_svg(Icon::X, 13.0))
                .on_press_maybe(
                    (!app.knowledge.importing_file)
                        .then_some(Message::Knowledge(KnowledgeToolMessage::ClearImportFile)),
                )
                .padding([8, 10])
                .style(knowledge_toolbar_button_style)
                .width(Length::Fixed(40.0)),
        );
    }

    let drop_zone_content = container(
        column![
            icon_badge(Icon::CloudUpload, has_selection || drop_hovered),
            column![
                text(drop_title).size(20),
                text(drop_meta).size(13).style(settings_muted_text_style),
            ]
            .spacing(6)
            .align_x(Alignment::Center),
            row![
                settings_value_badge("TXT"),
                settings_value_badge("MD"),
                settings_value_badge("MDX"),
                settings_value_badge("PDF"),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            actions,
        ]
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center),
    )
    .padding([24, 20])
    .width(Length::Fill)
    .height(Length::Fixed(220.0))
    .style(move |theme: &Theme| file_drop_zone_style(theme, drop_hovered, has_selection));

    let drop_zone: Element<'_, Message> = Element::new(DropArea::new(
        drop_zone_content.into(),
        Message::Knowledge(KnowledgeToolMessage::ImportDroppedFiles),
        Some((
            Message::Knowledge(KnowledgeToolMessage::ImportDropHoverChanged(true)),
            Message::Knowledge(KnowledgeToolMessage::ImportDropHoverChanged(false)),
        )),
        true,
    ));

    let import_header = responsive(move |size| {
        let heading = section_help(app, "文件入库", "可以手动选择文件，也可以拖到下方区域。");
        let pick = button(
            row![primary_action_icon(Icon::FileText, 14.0), text(manual_pick_label).size(13)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press_maybe(
            can_pick.then_some(Message::Knowledge(KnowledgeToolMessage::PickImportFile)),
        )
        .padding([9, 12])
        .style(primary_action_btn_style);
        if size.width < 300.0 {
            column![heading, pick].spacing(8).into()
        } else {
            row![container(heading).width(Length::Fill), pick]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        }
    })
    .height(Length::Shrink);

    let mut content = column![import_header, drop_zone].spacing(14);

    if has_selection {
        let selected_header = row![
            text(desktop_i18n::I18n::new(app.desktop_locale).__("已选文件"))
                .size(12)
                .width(Length::Fill),
            button(
                row![primary_action_icon(Icon::CloudUpload, 14.0), text(import_label).size(13)]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .on_press_maybe(
                can_import.then_some(Message::Knowledge(KnowledgeToolMessage::ImportSelectedFile))
            )
            .padding([9, 12])
            .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        content = content.push(
            container(
                column![selected_header, build_file_import_list(i18n, selected),].spacing(10),
            )
            .padding([14, 14])
            .width(Length::Fill)
            .style(chrome_style),
        );
    }

    if !app.knowledge.import_polling_jobs.is_empty() {
        content =
            content.push(build_import_polling_panel(i18n, &app.knowledge.import_polling_jobs));
    }

    container(content).padding([16, 16]).width(Length::Fill).style(chrome_style).into()
}

pub(super) fn build_import_polling_panel(
    i18n: desktop_i18n::I18n,
    jobs: &[KnowledgeImportPollingJob],
) -> Element<'_, Message> {
    let mut rows = column![].spacing(6);
    for job in jobs.iter().take(8) {
        rows = rows.push(
            row![
                text(job.filename.as_str()).size(12).width(Length::Fill),
                settings_value_badge(import_job_status_label(i18n, &job.status)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    if jobs.len() > 8 {
        rows = rows.push(
            text(
                i18n.format("还有 %s 个后台入库任务", &[(jobs.len() - 8).to_string().into()])
                    .expect("valid display template"),
            )
            .size(11)
            .style(settings_muted_text_style),
        );
    }
    container(column![text(i18n.__("后台入库")).size(12), rows].spacing(10))
        .padding([14, 14])
        .width(Length::Fill)
        .style(chrome_style)
        .into()
}

pub(super) fn build_file_import_list(
    i18n: desktop_i18n::I18n,
    files: &[crate::state::KnowledgeFileImportDraft],
) -> Element<'_, Message> {
    let mut rows = column![].spacing(6);
    for file in files.iter().take(8) {
        rows = rows.push(
            row![
                text(file.filename.as_str()).size(12).width(Length::Fill),
                text(file_import_summary(i18n, file)).size(11).style(settings_muted_text_style),
                settings_value_badge(i18n.__("待入队")),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    if files.len() > 8 {
        rows = rows.push(
            text(
                i18n.format("还有 %s 个文件", &[(files.len() - 8).to_string().into()])
                    .expect("valid display template"),
            )
            .size(11)
            .style(settings_muted_text_style),
        );
    }
    let height = (files.len().min(6) as f32 * 28.0).max(84.0);
    knowledge_scrollable(rows).height(Length::Fixed(height)).into()
}

pub(super) fn build_text_document_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let editor = text_editor(&app.knowledge.document_content_editor)
        .on_action(|action| Message::Knowledge(KnowledgeToolMessage::DocumentContentAction(action)))
        .placeholder(
            desktop_i18n::I18n::new(app.desktop_locale).__("粘贴文本、Markdown 或说明文档内容"),
        )
        .height(Length::Fixed(320.0))
        .style(settings_text_editor_style);

    container(
        column![
            section_help(app, "文本入库", "适合长说明、Markdown、提示词、知识笔记直接粘贴入库。"),
            text_input(
                desktop_i18n::I18n::new(app.desktop_locale).__("文档名称"),
                &app.knowledge.document_name_input
            )
            .on_input(|value| {
                Message::Knowledge(KnowledgeToolMessage::DocumentNameChanged(value))
            })
            .padding([13, 14])
            .size(14)
            .style(settings_text_input_style),
            text(desktop_i18n::I18n::new(app.desktop_locale).__("内容输入区"))
                .size(12)
                .style(settings_muted_text_style),
            container(editor).height(Length::Fixed(320.0)).width(Length::Fill),
            row![
                button(
                    row![
                        primary_action_icon(Icon::CloudUpload, 14.0),
                        text(desktop_i18n::I18n::new(app.desktop_locale).__("入库")).size(13)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .on_press_maybe(
                    (!app.knowledge.creating_document)
                        .then_some(Message::Knowledge(KnowledgeToolMessage::CreateDocument),)
                )
                .padding([12, 18])
                .style(primary_action_btn_style),
                button(text(desktop_i18n::I18n::new(app.desktop_locale).__("清空")).size(13))
                    .on_press(Message::Knowledge(KnowledgeToolMessage::ClearDocumentDraft))
                    .padding([12, 18])
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .padding([16, 16])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

pub(super) fn file_import_summary(
    i18n: desktop_i18n::I18n,
    file: &crate::state::KnowledgeFileImportDraft,
) -> String {
    let mime = file.mime_type.as_deref().unwrap_or(i18n.__("未知类型"));
    format!("{} · {mime}", format_file_size(file.byte_len))
}

pub(super) fn file_imports_summary(
    i18n: desktop_i18n::I18n,
    files: &[crate::state::KnowledgeFileImportDraft],
) -> String {
    match files {
        [] => i18n.__("等待选择").to_string(),
        [file] => file_import_summary(i18n, file),
        _ => {
            let total_bytes =
                files.iter().fold(0_u64, |total, file| total.saturating_add(file.byte_len));
            i18n.format(
                "%s · %s 个文件",
                &[
                    (format_file_size(total_bytes)).to_string().into(),
                    (files.len()).to_string().into(),
                ],
            )
            .expect("valid display template")
        }
    }
}

pub(super) fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}
