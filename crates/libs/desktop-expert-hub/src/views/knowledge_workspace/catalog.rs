use super::*;
use iced::widget::column;

pub(super) fn build_dataset_catalog_view(app: ViewInput<'_>) -> Element<'_, Message> {
    column![build_dataset_catalog_header(app), build_dataset_list_panel(app)]
        .spacing(26)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(super) fn build_dataset_catalog_header(app: ViewInput<'_>) -> Element<'_, Message> {
    let clear_notification: Element<'_, Message> = if app.knowledge.notification.is_some() {
        button(
            text(desktop_i18n::I18n::new(app.desktop_locale).__("清除提示"))
                .size(12)
                .line_height(1.0),
        )
        .on_press(Message::Knowledge(KnowledgeToolMessage::ClearNotification))
        .padding(0)
        .width(Length::Fixed(82.0))
        .height(Length::Fixed(38.0))
        .style(knowledge_toolbar_button_style)
        .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    row![
        build_dataset_search_input(app),
        knowledge_header_button(
            desktop_i18n::I18n::new(app.desktop_locale).__("刷新"),
            (!app.knowledge.is_busy()).then_some(KnowledgeToolMessage::Refresh),
        ),
        knowledge_header_button(
            desktop_i18n::I18n::new(app.desktop_locale).__("创建知识库"),
            (!app.knowledge.creating_dataset)
                .then_some(KnowledgeToolMessage::OpenCreateDatasetModal),
        ),
        Space::new().width(Length::Fill),
        clear_notification,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_dataset_search_input(app: ViewInput<'_>) -> Element<'_, Message> {
    container(
        text_input(
            desktop_i18n::I18n::new(app.desktop_locale).__("搜索"),
            &app.knowledge.dataset_search_query,
        )
        .on_input(|value| Message::Knowledge(KnowledgeToolMessage::DatasetSearchChanged(value)))
        .padding([9, 12])
        .size(13)
        .line_height(iced::Pixels(20.0))
        .width(Length::Fill)
        .style(settings_text_input_style),
    )
    .height(Length::Fixed(38.0))
    .width(Length::Fixed(260.0))
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

pub(super) fn knowledge_header_button(
    label: &'static str,
    message: Option<KnowledgeToolMessage>,
) -> Element<'static, Message> {
    button(
        container(text(label).size(12).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .style(rounded_action_btn_style)
    .padding(0)
    .width(Length::Fixed(96.0))
    .height(Length::Fixed(38.0))
    .on_press_maybe(message.map(Message::Knowledge))
    .into()
}

pub(super) fn build_dataset_list_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    if app.knowledge.loading_datasets && app.knowledge.datasets.is_empty() {
        return build_dataset_notice(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("正在加载知识库..."),
        );
    }

    if app.knowledge.datasets.is_empty() {
        return build_dataset_notice(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale)
                .__("暂无知识库，请创建知识库后添加文档入库。"),
        );
    }

    let query = app.knowledge.dataset_search_query.trim().to_lowercase();
    let filtered_datasets = app
        .knowledge
        .datasets
        .iter()
        .filter(|dataset| {
            query.is_empty()
                || dataset.name.to_lowercase().contains(&query)
                || dataset.description.to_lowercase().contains(&query)
                || dataset.id.to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();

    let mut cards = row![build_create_dataset_card(i18n)].spacing(18);
    for dataset in &filtered_datasets {
        cards = cards.push(dataset_item(app, dataset));
    }
    if filtered_datasets.is_empty() {
        cards = cards.push(build_no_dataset_result_card(i18n));
    }

    knowledge_scrollable(container(cards.wrap()).width(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

pub(super) fn build_dataset_notice(
    i18n: desktop_i18n::I18n,
    message: &'static str,
) -> Element<'static, Message> {
    container(
        column![
            text(message).size(15).style(settings_muted_text_style),
            button(text(i18n.__("创建知识库")).size(13))
                .style(primary_action_btn_style)
                .padding([10, 16])
                .on_press(Message::Knowledge(KnowledgeToolMessage::OpenCreateDatasetModal)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

pub(super) fn build_create_dataset_card(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    container(
        column![
            text(i18n.__("创建知识库")).size(14).style(settings_muted_text_style),
            create_dataset_action_button(Icon::FileEarmarkPlus, i18n.__("新建空白知识库")),
        ]
        .spacing(12)
        .width(Length::Fill),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .padding([22, 24])
    .style(dataset_card_container_style)
    .into()
}

pub(super) fn create_dataset_action_button(icon: Icon, label: &'static str) -> Element<'static, Message> {
    button(
        container(
            row![icon_badge(icon, true), text(label).size(14).line_height(1.0),]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 14])
        .align_y(iced::alignment::Vertical::Center),
    )
    .style(rounded_action_btn_style)
    .padding(0)
    .width(Length::Fill)
    .height(Length::Fixed(54.0))
    .on_press(Message::Knowledge(KnowledgeToolMessage::OpenCreateDatasetModal))
    .into()
}

pub(super) fn build_no_dataset_result_card(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    container(
        column![
            text(i18n.__("没有匹配的知识库")).size(15),
            text(i18n.__("换个搜索词再试。")).size(12).style(settings_muted_text_style),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(dataset_card_container_style)
    .into()
}
