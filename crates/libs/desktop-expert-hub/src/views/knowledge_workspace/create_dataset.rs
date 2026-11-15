use super::*;

pub(super) fn build_create_dataset_modal<'a>(
    app: ViewInput<'a>,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let heading = i18n.__("创建知识库");
    let title = row![
        settings_hover_help_label(
            i18n,
            app.app_theme,
            heading,
            text(heading).size(24),
            i18n.__("按模块完成基础信息、索引与检索配置。"),
        )
        .width(Length::Fill)
        .wrap(),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let submit_label = if app.knowledge.creating_dataset {
        desktop_i18n::I18n::new(app.desktop_locale).__("创建中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("创建知识库")
    };
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(13))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal))
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(submit_label).size(13))
            .on_press_maybe(
                (!app.knowledge.creating_dataset)
                    .then_some(Message::Knowledge(KnowledgeToolMessage::CreateDataset)),
            )
            .padding([9, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let card = settings_modal_card(
        column![title, build_create_dataset_panel(app), actions].spacing(16).width(Length::Fill),
    )
    .width(Length::Fixed(980.0))
    .max_width(1040.0)
    .height(Length::Fixed(660.0));

    settings_modal_overlay(
        base,
        Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal),
        card,
    )
}

pub(super) fn build_create_dataset_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let mut tabs = row![].spacing(8).align_y(Alignment::Center);
    for tab in KnowledgeCreateDatasetTab::ALL {
        tabs = tabs.push(segment_button(
            tab.title(),
            app.knowledge.create_dataset_tab == tab,
            Message::Knowledge(KnowledgeToolMessage::SelectCreateDatasetTab(tab)),
        ));
    }

    let content = match app.knowledge.create_dataset_tab {
        KnowledgeCreateDatasetTab::Basic => build_create_dataset_basic_tab(app),
        KnowledgeCreateDatasetTab::Indexing => build_create_dataset_indexing_tab(app),
        KnowledgeCreateDatasetTab::Retrieval => build_create_dataset_retrieval_tab(app),
    };

    column![
        container(tabs).padding([8, 10]).width(Length::Fill).style(chrome_style),
        knowledge_scrollable(content).height(Length::Fill),
    ]
    .spacing(14)
    .height(Length::Fill)
    .into()
}

pub(super) fn build_create_dataset_basic_tab(app: ViewInput<'_>) -> Element<'_, Message> {
    let fields = column![
        section_help(app, "基础信息", "名称用于识别知识库，描述用于说明内容范围。"),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("名称")).size(12),
        settings_text_input(
            desktop_i18n::I18n::new(app.desktop_locale).__("例如：产品文档"),
            &app.knowledge.dataset_name_input
        )
        .on_input(|value| Message::Knowledge(KnowledgeToolMessage::DatasetNameChanged(value))),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("描述")).size(12),
        settings_text_input(
            desktop_i18n::I18n::new(app.desktop_locale).__("简要说明知识库包含的内容"),
            &app.knowledge.dataset_description_input
        )
        .on_input(|value| {
            Message::Knowledge(KnowledgeToolMessage::DatasetDescriptionChanged(value))
        }),
    ]
    .spacing(10)
    .width(Length::Fill);

    let overview = column![
        section_header(
            desktop_i18n::I18n::new(app.desktop_locale).__("配置流程"),
            desktop_i18n::I18n::new(app.desktop_locale).__("创建前可随时切换页签检查配置。")
        ),
        create_dataset_step(
            "1",
            desktop_i18n::I18n::new(app.desktop_locale).__("基础信息"),
            desktop_i18n::I18n::new(app.desktop_locale).__("填写名称与用途")
        ),
        create_dataset_step(
            "2",
            desktop_i18n::I18n::new(app.desktop_locale).__("分段与索引"),
            desktop_i18n::I18n::new(app.desktop_locale).__("决定文档如何入库")
        ),
        create_dataset_step(
            "3",
            desktop_i18n::I18n::new(app.desktop_locale).__("检索设置"),
            desktop_i18n::I18n::new(app.desktop_locale).__("配置召回策略与参数")
        ),
    ]
    .spacing(12)
    .width(Length::Fill);

    row![
        container(fields).padding([18, 18]).width(Length::FillPortion(3)).style(chrome_style),
        container(overview).padding([18, 18]).width(Length::FillPortion(2)).style(chrome_style),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into()
}

pub(super) fn create_dataset_step<'a>(
    number: &'a str,
    title: &'a str,
    description: &'a str,
) -> Element<'a, Message> {
    row![
        settings_value_badge(number),
        column![text(title).size(13), text(description).size(11).style(settings_muted_text_style),]
            .spacing(3),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_create_dataset_indexing_tab(app: ViewInput<'_>) -> Element<'_, Message> {
    row![
        container(build_chunking_mode_section(app))
            .padding([16, 16])
            .width(Length::FillPortion(1))
            .style(chrome_style),
        column![
            container(build_indexing_mode_section(app))
                .padding([16, 16])
                .width(Length::Fill)
                .style(chrome_style),
            build_embedding_model_section(app),
        ]
        .spacing(14)
        .width(Length::FillPortion(1)),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into()
}

pub(super) fn build_create_dataset_retrieval_tab(app: ViewInput<'_>) -> Element<'_, Message> {
    row![
        container(build_retrieval_mode_section(app))
            .padding([16, 16])
            .width(Length::FillPortion(1))
            .style(chrome_style),
        container(build_dataset_param_section(app))
            .padding([16, 16])
            .width(Length::FillPortion(1))
            .style(chrome_style),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into()
}
