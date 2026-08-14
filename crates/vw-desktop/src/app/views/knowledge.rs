//! 知识库工作台视图。

use crate::app::assets::Icon;
use crate::app::components::input_panel::drop_area::DropArea;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings::SystemTab;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, icon_svg, primary_action_btn_style, round_icon_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_close_button,
    settings_error_banner, settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_success_banner, settings_text_editor_style, settings_text_input_style,
    settings_value_badge,
};
use crate::app::message::{KnowledgeToolMessage, ViewMessage};
use crate::app::state::{
    KnowledgeCreateDatasetTab, KnowledgeDatasetEmbeddingSelection, KnowledgeDetailTab,
    KnowledgeDocumentInputTab, KnowledgeImportPollingJob,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, pick_list, responsive, row, scrollable,
    text, text_editor, text_input,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Shadow, Size, Theme, Vector,
};
use std::fmt;
use vw_gateway_client::vw_api_types::knowledge::KnowledgeDocumentsBatchAction;
use vw_gateway_client::{
    KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode, KnowledgeDatasetDto,
    KnowledgeDocumentDto, KnowledgeDocumentStatus, KnowledgeEmbeddingPresetDto,
    KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestResponse, KnowledgeIndexingMode,
    KnowledgeMatchType, KnowledgeRetrievalMode,
};

const DOCUMENTS_PER_PAGE: usize = 10;
const DOCUMENT_PAGE_SIZE_OPTIONS: [usize; 6] = [10, 20, 30, 40, 50, 100];
const KNOWLEDGE_SCROLLBAR_WIDTH: f32 = 4.0;
const KNOWLEDGE_SCROLLBAR_GUTTER: f32 = 12.0;
const DOCUMENT_ACTIONS_WIDTH: f32 = 152.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentStatusFilterOption {
    label: &'static str,
    status: KnowledgeDocumentStatus,
}

#[derive(Debug)]
struct FilteredDocumentsPage<'a> {
    documents: Vec<&'a KnowledgeDocumentDto>,
    total_matches: usize,
    current_page: usize,
    total_pages: usize,
    page_size: usize,
    start_index: usize,
}

fn document_status_filter_options() -> [DocumentStatusFilterOption; 5] {
    [
        DocumentStatusFilterOption { label: "可用", status: KnowledgeDocumentStatus::Completed },
        DocumentStatusFilterOption {
            label: "索引中", status: KnowledgeDocumentStatus::Processing
        },
        DocumentStatusFilterOption { label: "入队", status: KnowledgeDocumentStatus::Pending },
        DocumentStatusFilterOption { label: "停用", status: KnowledgeDocumentStatus::Disabled },
        DocumentStatusFilterOption { label: "归档", status: KnowledgeDocumentStatus::Archived },
    ]
}

pub fn view(app: &App) -> Element<'_, Message> {
    let workspace = responsive(move |size| build_workspace(app, size));

    let mut content = column![].spacing(12).width(Length::Fill).height(Length::Fill);
    if let Some(notification) = &app.knowledge.notification {
        let banner = if app.knowledge.notification_is_error {
            settings_error_banner(notification)
        } else {
            settings_success_banner(notification)
        };
        content = content.push(banner);
    }
    content = content.push(container(workspace).width(Length::Fill).height(Length::Fill));

    let base: Element<'_, Message> = container(content.padding([30, 34]))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(palette.background.base.color.into()),
                ..Default::default()
            }
        })
        .into();

    if app.knowledge.create_dataset_modal_open {
        build_create_dataset_modal(app, Some(base))
    } else if app.knowledge.pending_delete_dataset_id.is_some()
        || app.knowledge.pending_delete_document_id.is_some()
    {
        build_delete_confirm_modal(app, Some(base))
    } else if app.knowledge.rename_document_id.is_some() {
        build_document_rename_modal(app, Some(base))
    } else {
        base
    }
}

fn build_workspace<'a>(app: &'a App, size: Size) -> Element<'a, Message> {
    if app.knowledge.showing_dataset_list || app.knowledge.selected_dataset_id.is_none() {
        build_dataset_catalog_view(app)
    } else {
        build_detail(app, size)
    }
}

fn build_dataset_catalog_view(app: &App) -> Element<'_, Message> {
    column![build_dataset_catalog_header(app), build_dataset_list_panel(app)]
        .spacing(26)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn build_dataset_catalog_header(app: &App) -> Element<'_, Message> {
    let clear_notification: Element<'_, Message> = if app.knowledge.notification.is_some() {
        button(text("清除提示").size(12).line_height(1.0))
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
            "刷新",
            (!app.knowledge.is_busy()).then_some(KnowledgeToolMessage::Refresh),
        ),
        knowledge_header_button(
            "创建知识库",
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

fn build_dataset_search_input(app: &App) -> Element<'_, Message> {
    container(
        text_input("搜索", &app.knowledge.dataset_search_query)
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

fn knowledge_header_button(
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

fn build_dataset_list_panel(app: &App) -> Element<'_, Message> {
    if app.knowledge.loading_datasets && app.knowledge.datasets.is_empty() {
        return build_dataset_notice("正在加载知识库...");
    }

    if app.knowledge.datasets.is_empty() {
        return build_dataset_notice("暂无知识库，请创建知识库后添加文档入库。");
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

    let mut cards = row![build_create_dataset_card()].spacing(18);
    for dataset in &filtered_datasets {
        cards = cards.push(dataset_item(app, dataset));
    }
    if filtered_datasets.is_empty() {
        cards = cards.push(build_no_dataset_result_card());
    }

    knowledge_scrollable(container(cards.wrap()).width(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn build_dataset_notice(message: &'static str) -> Element<'static, Message> {
    container(
        column![
            text(message).size(15).style(settings_muted_text_style),
            button(text("创建知识库").size(13))
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

fn build_create_dataset_card() -> Element<'static, Message> {
    container(
        column![
            text("创建知识库").size(14).style(settings_muted_text_style),
            create_dataset_action_button(Icon::FileEarmarkPlus, "新建空白知识库"),
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

fn create_dataset_action_button(icon: Icon, label: &'static str) -> Element<'static, Message> {
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

fn build_no_dataset_result_card() -> Element<'static, Message> {
    container(
        column![
            text("没有匹配的知识库").size(15),
            text("换个搜索词再试。").size(12).style(settings_muted_text_style),
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

fn build_create_dataset_modal<'a>(
    app: &'a App,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let title = row![
        column![
            text("创建知识库").size(24),
            text("按模块完成基础信息、索引与检索配置。").size(12).style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseCreateDatasetModal)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let submit_label = if app.knowledge.creating_dataset { "创建中" } else { "创建知识库" };
    let actions = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(13))
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

fn build_create_dataset_panel(app: &App) -> Element<'_, Message> {
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

fn build_create_dataset_basic_tab(app: &App) -> Element<'_, Message> {
    let fields = column![
        section_header("基础信息", "名称用于识别知识库，描述用于说明内容范围。"),
        text("名称").size(12),
        text_input("例如：产品文档", &app.knowledge.dataset_name_input)
            .on_input(|value| Message::Knowledge(KnowledgeToolMessage::DatasetNameChanged(value)))
            .padding([11, 12])
            .size(13)
            .style(settings_text_input_style),
        text("描述").size(12),
        text_input("简要说明知识库包含的内容", &app.knowledge.dataset_description_input)
            .on_input(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetDescriptionChanged(value))
            })
            .padding([11, 12])
            .size(13)
            .style(settings_text_input_style),
    ]
    .spacing(10)
    .width(Length::Fill);

    let overview = column![
        section_header("配置流程", "创建前可随时切换页签检查配置。"),
        create_dataset_step("1", "基础信息", "填写名称与用途"),
        create_dataset_step("2", "分段与索引", "决定文档如何入库"),
        create_dataset_step("3", "检索设置", "配置召回策略与参数"),
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

fn create_dataset_step<'a>(
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

fn build_create_dataset_indexing_tab(app: &App) -> Element<'_, Message> {
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

fn build_create_dataset_retrieval_tab(app: &App) -> Element<'_, Message> {
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

fn build_chunking_mode_section(app: &App) -> Element<'_, Message> {
    column![
        section_header("分段模式", "选择文档入库结构。"),
        option_card(
            "General",
            "通用文本分块。",
            Icon::ListUl,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::General,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::General,
            )),
        ),
        option_card(
            "Parent-Child",
            "子块检索，父块返回。",
            Icon::Columns,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::ParentChild,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::ParentChild,
            )),
        ),
        option_card(
            "Q&A",
            "问题检索，答案返回。",
            Icon::ChatTextFill,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::Qa,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::Qa,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

fn build_indexing_mode_section(app: &App) -> Element<'_, Message> {
    let vector_available = embedding_vector_available(app);
    column![
        section_header("索引模式", "高质量需要嵌入能力。"),
        option_card(
            "高质量",
            "生成向量索引。",
            Icon::Speedometer2,
            app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::HighQuality,
            vector_available,
            true,
            Message::Knowledge(KnowledgeToolMessage::DatasetIndexingModeChanged(
                KnowledgeIndexingMode::HighQuality,
            )),
        ),
        option_card(
            "经济",
            "关键词检索。",
            Icon::HddNetwork,
            app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::Economy,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetIndexingModeChanged(
                KnowledgeIndexingMode::Economy,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

fn build_embedding_model_section(app: &App) -> Element<'_, Message> {
    let status = app.knowledge.embedding_status.as_ref();
    let test_label = if app.knowledge.testing_embedding { "测试中" } else { "测试连接" };
    let test_enabled = !app.knowledge.testing_embedding && !app.knowledge.loading_embedding_status;

    let summary = row![
        embedding_summary_item("Provider", embedding_status_provider_label(status)),
        embedding_summary_item("模型", embedding_status_model_label(status)),
        embedding_summary_item("维度", embedding_status_dimensions_label(status)),
        embedding_summary_item("路由", embedding_status_route_label(status)),
    ]
    .spacing(8)
    .width(Length::Fill);

    let mut content = column![
        section_header("嵌入模型", "跟随全局配置，或锁定当前已生效模型。"),
        build_embedding_model_picker(app),
        summary,
        row![
            settings_value_badge(format!(
                "向量 {}",
                support_label(embedding_vector_available(app))
            )),
            settings_value_badge(format!(
                "混合 {}",
                support_label(embedding_hybrid_available(app))
            )),
        ]
        .spacing(6),
        row![
            button(
                row![primary_action_icon(Icon::Search, 13.0), text(test_label).size(12)].spacing(6)
            )
            .on_press_maybe(
                test_enabled.then_some(Message::Knowledge(KnowledgeToolMessage::RunEmbeddingTest))
            )
            .padding([8, 10])
            .style(primary_action_btn_style),
            button(row![icon_svg(Icon::Gear, 13.0), text("记忆配置").size(12)].spacing(6))
                .on_press(Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Memory)))
                .padding([8, 10])
                .style(knowledge_toolbar_button_style),
            button(row![icon_svg(Icon::Sliders, 13.0), text("嵌入路由").size(12)].spacing(6))
                .on_press(Message::View(ViewMessage::OpenSystemSettingsTab(
                    SystemTab::EmbeddingRoutes
                )))
                .padding([8, 10])
                .style(knowledge_toolbar_button_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(9);

    if app.knowledge.embedding_test_result.is_some() {
        content = content.push(embedding_kv_row(
            "测试结果",
            embedding_test_result_label(app.knowledge.embedding_test_result.as_ref()),
        ));
    }

    container(content).padding([16, 16]).width(Length::Fill).style(chrome_style).into()
}

fn embedding_summary_item<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    container(
        column![text(label).size(10).style(settings_muted_text_style), text(value).size(12),]
            .spacing(3),
    )
    .padding([8, 9])
    .width(Length::FillPortion(1))
    .style(|theme: &Theme| item_style(theme, false))
    .into()
}

fn build_retrieval_mode_section(app: &App) -> Element<'_, Message> {
    let vector_available = embedding_vector_available(app);
    let hybrid_available = embedding_hybrid_available(app);
    column![
        section_header("检索设置", "选择召回策略。"),
        option_card(
            "向量检索",
            "语义相似召回。",
            Icon::Grid1x2,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::Vector,
            vector_available,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::Vector,
            )),
        ),
        option_card(
            "全文检索",
            "按词汇召回。",
            Icon::FileText,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::FullText,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::FullText,
            )),
        ),
        option_card(
            "混合检索",
            "全文和向量合并。",
            Icon::Grid1x2,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::Hybrid,
            hybrid_available,
            true,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::Hybrid,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

fn build_dataset_param_section(app: &App) -> Element<'_, Message> {
    let mut content = column![section_header("参数", "默认召回参数。")].spacing(8);
    if app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::Economy {
        content = content.push(compact_input_row(
            "关键词数量",
            &app.knowledge.dataset_keyword_count_input,
            |value| Message::Knowledge(KnowledgeToolMessage::DatasetKeywordCountChanged(value)),
        ));
    }
    content =
        content.push(compact_input_row("Top K", &app.knowledge.dataset_top_k_input, |value| {
            Message::Knowledge(KnowledgeToolMessage::DatasetTopKChanged(value))
        }));
    content = content.push(
        checkbox(app.knowledge.dataset_score_threshold_enabled)
            .label("Score 阈值")
            .on_toggle(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetScoreThresholdEnabledChanged(value))
            })
            .style(settings_checkbox_style),
    );
    content = content.push(compact_input_row(
        "阈值",
        &app.knowledge.dataset_score_threshold_input,
        |value| Message::Knowledge(KnowledgeToolMessage::DatasetScoreThresholdChanged(value)),
    ));
    content = content.push(build_rerank_toggle(app));
    if app.knowledge.dataset_rerank_enabled {
        content = content.push(
            text_input("rerank 模型", &app.knowledge.dataset_rerank_model_input)
                .on_input(|value| {
                    Message::Knowledge(KnowledgeToolMessage::DatasetRerankModelChanged(value))
                })
                .padding([9, 10])
                .size(13)
                .style(settings_text_input_style),
        );
    }
    content.into()
}

fn build_rerank_toggle(app: &App) -> Element<'_, Message> {
    let rerank_available =
        app.knowledge.runtime_status.as_ref().is_some_and(|status| status.rerank);
    let label = if rerank_available { "Rerank 模型" } else { "Rerank 模型（未配置）" };
    let toggle = checkbox(app.knowledge.dataset_rerank_enabled && rerank_available)
        .label(label)
        .style(settings_checkbox_style);
    if rerank_available {
        toggle
            .on_toggle(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetRerankEnabledChanged(value))
            })
            .into()
    } else {
        toggle.into()
    }
}

fn section_header<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(12), text(description).size(11).style(settings_muted_text_style),]
        .spacing(2)
        .into()
}

fn compact_input_row<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(12).style(settings_muted_text_style).width(Length::Fixed(82.0)),
        text_input("", value)
            .on_input(on_input)
            .padding([8, 9])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn option_card<'a>(
    title: &'a str,
    description: &'a str,
    icon: Icon,
    active: bool,
    enabled: bool,
    recommended: bool,
    message: Message,
) -> Element<'a, Message> {
    let mut title_row = row![text(title).size(13)].spacing(6).align_y(Alignment::Center);
    if recommended {
        title_row = title_row.push(settings_value_badge("推荐"));
    }
    let content = row![
        icon_badge(icon, active),
        column![title_row, text(description).size(11).style(settings_muted_text_style),]
            .spacing(3)
            .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    button(
        container(content)
            .padding([10, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme| option_card_style(theme, active, enabled)),
    )
    .padding(0)
    .width(Length::Fill)
    .style(button::text)
    .on_press_maybe(enabled.then_some(message))
    .into()
}

fn dataset_item<'a>(app: &'a App, dataset: &'a KnowledgeDatasetDto) -> Element<'a, Message> {
    let selected = app.knowledge.selected_dataset_id.as_deref() == Some(dataset.id.as_str());
    let dataset_id = dataset.id.clone();
    let title = dataset_title(&dataset.name);
    let description = dataset_description(&dataset.description);
    let updated_label = format!("更新于 {}", format_dataset_time(dataset.updated_at_ms));

    let open_area = button(
        column![
            row![
                icon_badge(Icon::Journals, selected),
                column![
                    text(title).size(16).line_height(1.0).style(dataset_title_text_style),
                    text(updated_label).size(12).style(settings_muted_text_style),
                ]
                .spacing(6)
                .width(Length::Fill),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            text(description).size(13).style(settings_muted_text_style),
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .style(dataset_card_button_style)
    .padding(0)
    .width(Length::Fill)
    .on_press(Message::Knowledge(KnowledgeToolMessage::SelectDataset(dataset_id)));

    let menu = dataset_actions_menu(app, dataset);

    container(
        column![
            row![open_area, menu].spacing(8).align_y(Alignment::Start),
            dataset_id_row(dataset),
            row![
                dataset_tag(chunking_label(&dataset.chunking_mode)),
                dataset_tag(indexing_label(&dataset.indexing_mode)),
                dataset_tag(retrieval_label(&dataset.retrieval_mode)),
                dataset_tag_static(if selected { "当前选择" } else { "知识库" }),
            ]
            .spacing(6)
            .wrap(),
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .padding([16, 20])
    .style(dataset_card_container_style)
    .into()
}

/// 卡片中部一行：左侧展示知识库 ID，右侧为紧凑的“复制”按钮。
fn dataset_id_row(dataset: &KnowledgeDatasetDto) -> Element<'_, Message> {
    let copy_content =
        row![themed_inline_icon(Icon::Copy, 13.0), text("复制").size(11).line_height(1.0),]
            .spacing(5)
            .align_y(Alignment::Center);

    row![
        text(format!("ID: {}", dataset.id))
            .size(11)
            .style(settings_muted_text_style)
            .width(Length::Fill),
        settings_value_badge(format!("{} 文档", dataset.document_count)),
        button(
            container(copy_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .style(rounded_action_btn_style)
        .padding([5, 8])
        .width(Length::Fixed(58.0))
        .height(Length::Fixed(28.0))
        .on_press(Message::Knowledge(KnowledgeToolMessage::CopyDatasetId(dataset.id.clone()))),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// 卡片右上角的“更多操作”下拉菜单（删除）。
fn dataset_actions_menu<'a>(
    app: &'a App,
    dataset: &'a KnowledgeDatasetDto,
) -> Element<'a, Message> {
    let menu_open = app.knowledge.dataset_menu_open_id.as_deref() == Some(dataset.id.as_str());

    let trigger = button(
        container(themed_inline_icon(Icon::DotsThreeVertical, 16.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .style(round_icon_btn_style)
    .on_press(Message::Knowledge(KnowledgeToolMessage::ToggleDatasetMenu(dataset.id.clone())));

    let delete = button(
        row![
            themed_inline_icon(Icon::Trash, 14.0),
            text("删除知识库").size(13).width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::Knowledge(KnowledgeToolMessage::OpenDeleteDatasetConfirmFor(
        dataset.id.clone(),
    )))
    .padding([8, 10])
    .width(Length::Fill)
    .style(knowledge_danger_menu_entry_style);

    PointBelowOverlay::new(
        trigger,
        container(column![delete].spacing(4).width(Length::Fill))
            .width(Length::Fixed(168.0))
            .padding(6)
            .style(knowledge_status_menu_style),
    )
    .show(menu_open)
    .anchor(Point::new(-136.0, 36.0))
    .gap(4.0)
    .on_close(Message::Knowledge(KnowledgeToolMessage::CloseDatasetMenu))
    .capture_outside_click(false)
    .into()
}

fn knowledge_danger_menu_entry_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    iced::widget::button::Style {
        background: hovered.then(|| palette.danger.base.color.scale_alpha(0.16).into()),
        text_color: palette.danger.base.color,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

fn dataset_title(name: &str) -> String {
    if name.trim().is_empty() { "未命名知识库".to_string() } else { name.trim().to_string() }
}

fn dataset_description(description: &str) -> String {
    if description.trim().is_empty() {
        "暂无描述".to_string()
    } else {
        description.trim().to_string()
    }
}

fn dataset_tag(label: &'static str) -> Element<'static, Message> {
    dataset_tag_static(label)
}

fn dataset_tag_static(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(
                    palette.background.weak.color.scale_alpha(if is_dark_palette(theme) {
                        0.70
                    } else {
                        0.52
                    }),
                )),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.18),
                    radius: 4.0.into(),
                },
                text_color: Some(theme.palette().text.scale_alpha(0.72)),
                ..Default::default()
            }
        })
        .into()
}

fn format_dataset_time(timestamp_ms: u64) -> String {
    use chrono::{Local, TimeZone};

    let Ok(timestamp_ms) = i64::try_from(timestamp_ms) else {
        return "--".to_string();
    };
    let Some(dt) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return "--".to_string();
    };
    dt.format("%Y/%m/%d %H:%M").to_string()
}

fn dataset_card_container_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_palette(theme) {
            palette.background.weak.color.scale_alpha(0.74)
        } else {
            Color::WHITE.scale_alpha(0.88)
        })),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_palette(theme) {
                0.36
            } else {
                0.14
            }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_palette(theme) { 0.18 } else { 0.08 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

fn dataset_card_button_style(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        ..Default::default()
    }
}

fn dataset_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(if is_dark_palette(theme) { Color::WHITE } else { theme.palette().text }),
    }
}

fn build_detail<'a>(app: &'a App, size: Size) -> Element<'a, Message> {
    let Some(dataset) = app.knowledge.selected_dataset() else {
        return settings_panel(
            column![
                text("选择或创建一个知识库").size(18),
                text("左侧列表会显示全部知识库。").size(13).style(settings_muted_text_style),
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
                settings_value_badge(indexing_label(&dataset.indexing_mode)),
                settings_value_badge(retrieval_label(&dataset.retrieval_mode)),
                settings_value_badge(format!("{} 文档", dataset.document_count)),
                settings_value_badge(format!("{} 分段", dataset.chunk_count)),
            ]
            .spacing(6),
        ]
        .spacing(8)
        .width(Length::Fill),
        button(centered_header_action_content(Icon::Back, "返回列表"))
            .on_press(Message::Knowledge(KnowledgeToolMessage::ShowDatasetList))
            .padding(0)
            .width(Length::Fixed(124.0))
            .height(Length::Fixed(40.0))
            .style(rounded_action_btn_style),
        button(centered_header_action_content(Icon::Trash, "删除"))
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

fn build_tabs(app: &App) -> Element<'_, Message> {
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

fn build_documents_tab<'a>(app: &'a App, _size: Size) -> Element<'a, Message> {
    build_documents_table(app)
}

fn build_add_document_tab(app: &App) -> Element<'_, Message> {
    build_upload_panel(app)
}

fn build_upload_panel(app: &App) -> Element<'_, Message> {
    let body = match app.knowledge.document_input_tab {
        KnowledgeDocumentInputTab::Files => build_file_import_panel(app),
        KnowledgeDocumentInputTab::Text => build_text_document_panel(app),
    };

    settings_panel(
        knowledge_scrollable(
            column![
                row![
                    text("添加文档").size(14).width(Length::Fill),
                    settings_value_badge("TXT"),
                    settings_value_badge("MD"),
                    settings_value_badge("MDX"),
                    settings_value_badge("PDF"),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                text("支持拖拽文件上传，或直接粘贴长文本入库。")
                    .size(12)
                    .style(settings_muted_text_style),
                build_document_input_tabs(app),
                body,
            ]
            .spacing(14)
            .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}

fn build_document_input_tabs(app: &App) -> Element<'_, Message> {
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

fn build_file_import_panel(app: &App) -> Element<'_, Message> {
    let selected = app.knowledge.file_imports.as_slice();
    let selected_count = selected.len();
    let has_selection = selected_count > 0;
    let dragging_any_file =
        !app.dragging_file_paths.is_empty() || !app.pending_drop_file_paths.is_empty();
    let drop_hovered = app.knowledge.file_drop_hovered && dragging_any_file;
    let drop_title = match selected {
        [] if drop_hovered => "释放鼠标，开始准备上传".to_string(),
        [] => "拖拽文件到这里上传".to_string(),
        [file] => file.filename.clone(),
        _ => format!("已选择 {selected_count} 个文件"),
    };
    let drop_meta = match selected {
        [] if drop_hovered => "支持批量拖入 TXT / MD / MDX / PDF".to_string(),
        [] => "支持批量拖拽或点击选择 TXT / MD / MDX / PDF 文件".to_string(),
        _ => file_imports_summary(selected),
    };
    let can_pick = !app.knowledge.selecting_import_file && !app.knowledge.importing_file;
    let can_import = has_selection
        && app.knowledge.selected_dataset_id.is_some()
        && !app.knowledge.importing_file;
    let pick_label = if app.knowledge.selecting_import_file { "选择中" } else { "选择文件" };
    let manual_pick_label =
        if app.knowledge.selecting_import_file { "选择中" } else { "手动选择文件" };
    let import_label = if app.knowledge.importing_file {
        "上传中".to_string()
    } else if selected_count > 1 {
        format!("上传 {selected_count} 个文件")
    } else {
        "上传入库".to_string()
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

    let import_header = row![
        container(section_header("文件入库", "可以手动选择文件，也可以拖到下方区域。"))
            .width(Length::Fill),
        button(
            row![primary_action_icon(Icon::FileText, 14.0), text(manual_pick_label).size(13)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press_maybe(
            can_pick.then_some(Message::Knowledge(KnowledgeToolMessage::PickImportFile))
        )
        .padding([9, 12])
        .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut content = column![import_header, drop_zone].spacing(14);

    if has_selection {
        let selected_header = row![
            text("已选文件").size(12).width(Length::Fill),
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
            container(column![selected_header, build_file_import_list(selected),].spacing(10))
                .padding([14, 14])
                .width(Length::Fill)
                .style(chrome_style),
        );
    }

    if !app.knowledge.import_polling_jobs.is_empty() {
        content = content.push(build_import_polling_panel(&app.knowledge.import_polling_jobs));
    }

    container(content).padding([16, 16]).width(Length::Fill).style(chrome_style).into()
}

fn build_import_polling_panel(jobs: &[KnowledgeImportPollingJob]) -> Element<'_, Message> {
    let mut rows = column![].spacing(6);
    for job in jobs.iter().take(8) {
        rows = rows.push(
            row![
                text(job.filename.as_str()).size(12).width(Length::Fill),
                settings_value_badge(import_job_status_label(&job.status)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    if jobs.len() > 8 {
        rows = rows.push(
            text(format!("还有 {} 个后台入库任务", jobs.len() - 8))
                .size(11)
                .style(settings_muted_text_style),
        );
    }
    container(column![text("后台入库").size(12), rows].spacing(10))
        .padding([14, 14])
        .width(Length::Fill)
        .style(chrome_style)
        .into()
}

fn build_file_import_list(
    files: &[crate::app::state::KnowledgeFileImportDraft],
) -> Element<'_, Message> {
    let mut rows = column![].spacing(6);
    for file in files.iter().take(8) {
        rows = rows.push(
            row![
                text(file.filename.as_str()).size(12).width(Length::Fill),
                text(file_import_summary(file)).size(11).style(settings_muted_text_style),
                settings_value_badge("待入队"),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    if files.len() > 8 {
        rows = rows.push(
            text(format!("还有 {} 个文件", files.len() - 8))
                .size(11)
                .style(settings_muted_text_style),
        );
    }
    let height = (files.len().min(6) as f32 * 28.0).max(84.0);
    knowledge_scrollable(rows).height(Length::Fixed(height)).into()
}

fn build_text_document_panel(app: &App) -> Element<'_, Message> {
    let editor = text_editor(&app.knowledge.document_content_editor)
        .on_action(|action| Message::Knowledge(KnowledgeToolMessage::DocumentContentAction(action)))
        .placeholder("粘贴文本、Markdown 或说明文档内容")
        .height(Length::Fixed(320.0))
        .style(settings_text_editor_style);

    container(
        column![
            section_header("文本入库", "适合长说明、Markdown、提示词、知识笔记直接粘贴入库。"),
            text_input("文档名称", &app.knowledge.document_name_input)
                .on_input(|value| {
                    Message::Knowledge(KnowledgeToolMessage::DocumentNameChanged(value))
                })
                .padding([13, 14])
                .size(14)
                .style(settings_text_input_style),
            text("内容输入区").size(12).style(settings_muted_text_style),
            container(editor).height(Length::Fixed(320.0)).width(Length::Fill),
            row![
                button(
                    row![primary_action_icon(Icon::CloudUpload, 14.0), text("入库").size(13)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .on_press_maybe(
                    (!app.knowledge.creating_document)
                        .then_some(Message::Knowledge(KnowledgeToolMessage::CreateDocument),)
                )
                .padding([12, 18])
                .style(primary_action_btn_style),
                button(text("清空").size(13))
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

fn file_import_summary(file: &crate::app::state::KnowledgeFileImportDraft) -> String {
    let mime = file.mime_type.as_deref().unwrap_or("未知类型");
    format!("{} · {mime}", format_file_size(file.byte_len))
}

fn file_imports_summary(files: &[crate::app::state::KnowledgeFileImportDraft]) -> String {
    match files {
        [] => "等待选择".to_string(),
        [file] => file_import_summary(file),
        _ => {
            let total_bytes =
                files.iter().fold(0_u64, |total, file| total.saturating_add(file.byte_len));
            format!("{} · {} 个文件", format_file_size(total_bytes), files.len())
        }
    }
}

fn format_file_size(bytes: u64) -> String {
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

fn build_documents_table(app: &App) -> Element<'_, Message> {
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
            "全选",
            Some(Message::Knowledge(KnowledgeToolMessage::ToggleSelectAllVisibleDocuments)),
        ))
        .center_x(Length::Fixed(28.0)),
        text("序号").size(12).width(Length::Fixed(42.0)),
        text("名称").size(12).width(Length::Fill),
        text("分段模式").size(12).width(Length::Fixed(86.0)),
        text("字符数").size(12).width(Length::Fixed(76.0)),
        text("召回次数").size(12).width(Length::Fixed(76.0)),
        text("更新时间").size(12).width(Length::Fixed(150.0)),
        text("状态").size(12).width(Length::Fixed(74.0)),
        text("操作").size(12).width(Length::Fixed(DOCUMENT_ACTIONS_WIDTH)),
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
        rows = rows.push(empty_hint("正在加载文档"));
    } else if app.knowledge.documents.is_empty() {
        rows = rows.push(empty_hint("暂无文档"));
    } else if filtered_page.total_matches == 0 {
        rows = rows.push(empty_hint("没有匹配文档"));
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

fn build_document_status_menu(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(2);
    for option in document_status_filter_options() {
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

    let clear = button(text("清除筛选").size(12).width(Length::Fill))
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

fn knowledge_status_menu_style(theme: &Theme) -> iced::widget::container::Style {
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

fn knowledge_status_menu_entry_style(
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

fn build_documents_toolbar(app: &App) -> Element<'_, Message> {
    let selected_count = app.knowledge.document_status_filter.len();
    let summary = match selected_count {
        0 => "全部状态".to_string(),
        1 => document_status_filter_options()
            .into_iter()
            .find(|option| app.knowledge.document_status_filter.contains(&option.status))
            .map(|option| option.label.to_string())
            .unwrap_or_else(|| "全部状态".to_string()),
        count => format!("已选 {count} 项"),
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

    let search = text_input("搜索文档", &app.knowledge.document_search_query)
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
                row![themed_inline_icon(Icon::SquareHalf, 13.0), text("反选").size(13)].spacing(8)
            )
            .on_press(Message::Knowledge(KnowledgeToolMessage::InvertVisibleDocumentSelection))
            .padding([9, 12])
            .style(knowledge_toolbar_button_style),
            button(
                row![themed_inline_icon(Icon::ArrowRepeat, 13.0), text("刷新").size(13)].spacing(8)
            )
            .on_press_maybe(
                (!app.knowledge.is_busy())
                    .then_some(Message::Knowledge(KnowledgeToolMessage::Refresh))
            )
            .padding([9, 12])
            .style(knowledge_toolbar_button_style),
            button(
                row![primary_action_icon(Icon::FileEarmarkPlus, 13.0), text("新增文件").size(13)]
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

fn filtered_documents_page(app: &App) -> FilteredDocumentsPage<'_> {
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

fn document_page_size(app: &App) -> usize {
    match app.knowledge.document_page_size {
        10 | 20 | 30 | 40 | 50 | 100 => app.knowledge.document_page_size,
        _ => DOCUMENTS_PER_PAGE,
    }
}

fn document_total_pages(total_items: usize, page_size: usize) -> usize {
    total_items.div_ceil(page_size.max(1)).max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentPageItem {
    Page(usize),
    Ellipsis,
}

fn document_pagination_items(current_page: usize, total_pages: usize) -> Vec<DocumentPageItem> {
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

fn build_document_pagination(
    current_page: usize,
    total_pages: usize,
    total_matches: usize,
    page_size: usize,
) -> Element<'static, Message> {
    let mut pages = row![document_page_nav_button("上一页", current_page.checked_sub(1)),]
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
    pages = pages.push(document_page_nav_button("下一页", next_page));

    container(
        row![
            row![
                text(format!("共 {total_matches} 个文档"))
                    .size(12)
                    .style(settings_muted_text_style),
                text("每页").size(12).style(settings_muted_text_style),
                document_page_size_selector(page_size),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill),
            pages,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .height(Length::Fixed(50.0))
    .into()
}

fn document_page_size_selector(page_size: usize) -> Element<'static, Message> {
    pick_list(DOCUMENT_PAGE_SIZE_OPTIONS, Some(page_size), |size| {
        Message::Knowledge(KnowledgeToolMessage::DocumentPageSizeChanged(size))
    })
    .padding([5, 10])
    .text_size(12)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(86.0))
    .into()
}

fn document_page_button(page: usize, active: bool) -> Element<'static, Message> {
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

fn document_page_nav_button(label: &'static str, page: Option<usize>) -> Element<'static, Message> {
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

#[cfg(test)]
fn document_row<'a>(app: &'a App, document: &'a KnowledgeDocumentDto) -> Element<'a, Message> {
    let chunking_mode = app
        .knowledge
        .selected_dataset()
        .map(|dataset| &dataset.chunking_mode)
        .unwrap_or(&KnowledgeChunkingMode::General);
    document_row_with_index(app, document, 1, chunking_mode)
}

fn document_row_with_index<'a>(
    app: &'a App,
    document: &'a KnowledgeDocumentDto,
    row_number: usize,
    chunking_mode: &'a KnowledgeChunkingMode,
) -> Element<'a, Message> {
    let selected = app.knowledge.document_preview_open
        && app.knowledge.selected_document_id.as_deref() == Some(document.id.as_str());
    let reindex_enabled = !app.knowledge.is_busy();
    let loading_this = app.knowledge.loading_chunks
        && app.knowledge.selected_document_id.as_deref() == Some(document.id.as_str());
    let reindexing_this =
        app.knowledge.reindexing_document_id.as_deref() == Some(document.id.as_str());
    let updating_this = app.knowledge.updating_document_id.as_deref() == Some(document.id.as_str());
    let status = effective_document_status(document);
    let archived = status == KnowledgeDocumentStatus::Archived;
    let status_label = document_status_label(&status);
    let open_detail_message = (!loading_this).then_some(Message::Knowledge(
        KnowledgeToolMessage::LoadDocumentChunks(document.id.clone()),
    ));
    let title = document_title_button(&document.name, open_detail_message.clone(), selected);
    let name = if document.error_message.trim().is_empty() {
        column![title].spacing(2).width(Length::FillPortion(4))
    } else {
        column![title, text(&document.error_message).size(10).style(settings_muted_text_style),]
            .spacing(2)
            .width(Length::FillPortion(4))
    };

    let toggle_message = if archived {
        (!updating_this).then_some(Message::Knowledge(KnowledgeToolMessage::SetDocumentArchived {
            document_id: document.id.clone(),
            archived: false,
        }))
    } else {
        (!updating_this).then_some(Message::Knowledge(KnowledgeToolMessage::SetDocumentEnabled {
            document_id: document.id.clone(),
            enabled: !document.enabled,
        }))
    };
    let archive_message = (!archived && !updating_this).then_some(Message::Knowledge(
        KnowledgeToolMessage::SetDocumentArchived {
            document_id: document.id.clone(),
            archived: true,
        },
    ));
    let select_message = Some(Message::Knowledge(KnowledgeToolMessage::ToggleDocumentSelection(
        document.id.clone(),
    )));
    let selection_tooltip = if app.knowledge.selected_document_ids.contains(&document.id) {
        "取消选择"
    } else {
        "选择文档"
    };
    let content = row![
        container(document_selection_button(
            app.knowledge.selected_document_ids.contains(&document.id),
            selection_tooltip,
            select_message,
        ))
        .center_x(Length::Fixed(28.0)),
        text(row_number.to_string()).size(12).width(Length::Fixed(42.0)),
        name,
        text(chunking_label(chunking_mode))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(86.0)),
        text(compact_document_count_label(document.char_count))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(76.0)),
        text(compact_document_count_label(document.recall_count))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(76.0)),
        text(format_time(document.updated_at_ms))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(150.0)),
        container(document_status_badge(document, status_label)).width(Length::Fixed(74.0)),
        row![
            document_icon_action_button(
                centered_knowledge_icon(
                    if archived || !document.enabled { Icon::Check } else { Icon::EyeSlash },
                    10.0,
                ),
                if archived {
                    "恢复文档"
                } else if document.enabled {
                    "停用文档"
                } else {
                    "启用文档"
                },
                toggle_message,
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Eye, 10.0),
                "查看详情",
                open_detail_message,
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::ArrowRepeat, 10.0),
                "重新入库",
                (reindex_enabled && !reindexing_this).then_some(Message::Knowledge(
                    KnowledgeToolMessage::ReindexDocument(document.id.clone())
                )),
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Pencil, 10.0),
                "重命名",
                (!updating_this).then_some(Message::Knowledge(
                    KnowledgeToolMessage::OpenRenameDocument(document.id.clone()),
                )),
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Box, 10.0),
                "归档文档",
                archive_message,
                false,
            ),
            document_icon_action_button(
                centered_danger_icon(Icon::Trash, 10.0),
                "删除文档",
                (!app.knowledge.deleting).then_some(Message::Knowledge(
                    KnowledgeToolMessage::OpenDeleteDocumentConfirm(document.id.clone())
                )),
                true,
            ),
        ]
        .spacing(4)
        .width(Length::Fixed(DOCUMENT_ACTIONS_WIDTH)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(content)
        .padding([6, 8])
        .width(Length::Fill)
        .style(move |theme: &Theme| document_row_style(theme, selected))
        .into()
}

fn document_selection_button<'a>(
    selected: bool,
    tooltip_label: &'a str,
    message: Option<Message>,
) -> Element<'a, Message> {
    let button = button(centered_knowledge_icon(
        if selected { Icon::CheckSquare } else { Icon::Square },
        14.0,
    ))
    .on_press_maybe(message)
    .padding(0)
    .width(Length::Fixed(22.0))
    .height(Length::Fixed(22.0))
    .style(document_selection_button_style);
    Tooltip::new(button, knowledge_tooltip_content(tooltip_label), TooltipPosition::Top)
        .gap(6.0)
        .into()
}

fn document_selection_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

fn build_document_bulk_toolbar<'a>(
    app: &'a App,
    _visible_documents: &[&'a KnowledgeDocumentDto],
) -> Element<'a, Message> {
    let selected_count = app.knowledge.selected_document_ids.len();
    let pending_action = app.knowledge.pending_batch_document_action.as_ref();
    let busy = app.knowledge.batch_updating_documents;
    let enable_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Enable) {
        "启用中"
    } else {
        "批量启用"
    };
    let disable_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Disable) {
        "停用中"
    } else {
        "批量停用"
    };
    let archive_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Archive) {
        "归档中"
    } else {
        "批量归档"
    };

    container(
        row![
            text(format!("已选择 {selected_count} 篇文档"))
                .size(12)
                .width(Length::Fill)
                .style(settings_muted_text_style),
            button(text(enable_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Enable,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text(disable_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Disable,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text(archive_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Archive,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text("取消").size(12))
                .on_press(Message::Knowledge(KnowledgeToolMessage::ClearDocumentSelection))
                .padding([8, 12])
                .style(knowledge_toolbar_button_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

fn build_document_rename_modal<'a>(
    app: &'a App,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let Some(document_id) = app.knowledge.rename_document_id.as_deref() else {
        return base.unwrap_or_else(|| Space::new().into());
    };
    let current_name = app
        .knowledge
        .documents
        .iter()
        .find(|document| document.id == document_id)
        .map(|document| document.name.as_str())
        .unwrap_or("当前文档");
    let submitting = app.knowledge.updating_document_id.as_deref() == Some(document_id);
    let can_submit = !submitting && !app.knowledge.rename_document_name_input.trim().is_empty();
    let submit_label = if submitting { "保存中" } else { "保存" };
    let title_row = row![
        column![
            text("重命名文档").size(22),
            text(format!("当前名称：{current_name}")).size(12).style(settings_muted_text_style),
        ]
        .spacing(6)
        .width(Length::Fill),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let actions = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(13))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument))
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(submit_label).size(13))
            .on_press_maybe(can_submit.then_some(Message::Knowledge(
                KnowledgeToolMessage::SubmitDocumentRename {
                    document_id: document_id.to_string(),
                    name: app.knowledge.rename_document_name_input.clone(),
                },
            )))
            .padding([9, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_card(
        column![
            title_row,
            text_input("输入新的文档名称", &app.knowledge.rename_document_name_input)
                .on_input(|value| {
                    Message::Knowledge(KnowledgeToolMessage::RenameDocumentDraftChanged(value))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style),
            actions,
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(520.0))
    .max_width(560.0);

    settings_modal_overlay(
        base,
        Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument),
        card,
    )
}

fn document_title_button<'a>(
    label: &'a str,
    message: Option<Message>,
    selected: bool,
) -> Element<'a, Message> {
    button(
        container(text(label).size(12).line_height(1.0))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left),
    )
    .on_press_maybe(message)
    .padding([2, 4])
    .width(Length::Fill)
    .style(move |theme: &Theme, status| document_title_button_style(theme, status, selected))
    .into()
}

fn document_icon_action_button<'a>(
    content: Element<'a, Message>,
    tooltip_label: &'a str,
    message: Option<Message>,
    danger: bool,
) -> Element<'a, Message> {
    let button = button(content)
        .on_press_maybe(message)
        .padding(0)
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0))
        .style(if danger { round_danger_icon_btn_style } else { round_icon_btn_style });
    Tooltip::new(button, knowledge_tooltip_content(tooltip_label), TooltipPosition::Top)
        .gap(6.0)
        .into()
}

fn document_status_badge<'a>(
    document: &'a KnowledgeDocumentDto,
    status_label: &'a str,
) -> Element<'a, Message> {
    container(text(status_label).size(10).line_height(1.0))
        .padding([3, 7])
        .style(move |theme: &Theme| document_status_badge_style(theme, document))
        .into()
}

#[cfg(test)]
fn build_chunks_panel(app: &App) -> Element<'_, Message> {
    let selected_document = app.knowledge.selected_document();
    let title = selected_document
        .map(|document| format!("分段预览 · {}", document.name))
        .unwrap_or_else(|| "分段预览".to_string());
    let mut content = column![text(title).size(14)].spacing(10).height(Length::Fill);

    if selected_document.is_none() {
        content = content.push(empty_hint("选择文档查看分段"));
    } else if app.knowledge.loading_chunks {
        content = content.push(empty_hint("正在加载分段"));
    } else if app.knowledge.loaded_chunks.is_empty() {
        content = content.push(empty_hint("暂无分段"));
    } else {
        let mut list = column![].spacing(8);
        for chunk in &app.knowledge.loaded_chunks {
            let selected = app.knowledge.selected_chunk_id.as_deref() == Some(chunk.id.as_str());
            list = list.push(chunk_preview_card(chunk, selected));
        }
        content = content.push(knowledge_scrollable(list).height(Length::Fill));
        if let Some(chunk) = app.knowledge.selected_chunk() {
            content = content.push(chunk_content_panel(chunk, app.last_copied_code_hash));
        }
    }

    settings_panel(content).into()
}

fn build_document_preview_drawer(app: &App) -> Element<'_, Message> {
    let selected_document = app.knowledge.selected_document();
    let title = selected_document.map(|document| document.name.as_str()).unwrap_or("文档预览");

    let mut body = column![
        row![
            column![
                text("文档预览").size(16),
                text(title).size(12).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseDocumentPreview)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .height(Length::Fill);

    if selected_document.is_none() {
        body = body.push(empty_hint("选择文档查看分段"));
    } else if app.knowledge.loading_chunks {
        body = body.push(empty_hint("正在加载分段"));
    } else if app.knowledge.loaded_chunks.is_empty() {
        body = body.push(empty_hint("暂无分段"));
    } else {
        let mut list = column![].spacing(8);
        for chunk in &app.knowledge.loaded_chunks {
            let selected = app.knowledge.selected_chunk_id.as_deref() == Some(chunk.id.as_str());
            list = list.push(chunk_preview_card(chunk, selected));
        }
        body = body.push(knowledge_scrollable(list).height(Length::FillPortion(3)));
        if let Some(chunk) = app.knowledge.selected_chunk() {
            body = body.push(
                container(chunk_content_panel(chunk, app.last_copied_code_hash))
                    .height(Length::FillPortion(2)),
            );
        }
    }

    container(
        container(body)
            .width(Length::Fixed(430.0))
            .height(Length::Fill)
            .padding([18, 20])
            .style(document_preview_drawer_style),
    )
    .padding(iced::Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 0.0 })
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

fn document_preview_drawer_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_palette(theme) {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::WHITE.scale_alpha(0.98)
        })),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_palette(theme) {
                0.56
            } else {
                0.18
            }),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_palette(theme) { 0.28 } else { 0.14 }),
            offset: Vector::new(-8.0, 0.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn build_retrieval_tab(app: &App) -> Element<'_, Message> {
    let results = if app.knowledge.retrieve_results.is_empty() {
        empty_hint("召回结果会显示在这里")
    } else {
        let mut list = column![].spacing(10);
        for chunk in &app.knowledge.retrieve_results {
            list = list.push(chunk_card(chunk));
        }
        knowledge_scrollable(list).height(Length::Fill).into()
    };

    settings_panel(
        column![
            row![
                text_input("输入查询文本", &app.knowledge.retrieve_query_input)
                    .on_input(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveQueryChanged(value))
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
                text_input("Top K", &app.knowledge.retrieve_top_k_input)
                    .on_input(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveTopKChanged(value))
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fixed(90.0)),
                button(
                    row![primary_action_icon(Icon::Search, 14.0), text("测试").size(13)].spacing(8)
                )
                .on_press_maybe(
                    (!app.knowledge.retrieving)
                        .then_some(Message::Knowledge(KnowledgeToolMessage::Retrieve))
                )
                .padding([10, 14])
                .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                checkbox(app.knowledge.retrieve_score_threshold_enabled)
                    .label("Score 阈值")
                    .on_toggle(|value| {
                        Message::Knowledge(
                            KnowledgeToolMessage::RetrieveScoreThresholdEnabledChanged(value),
                        )
                    })
                    .style(settings_checkbox_style),
                text_input("0.15", &app.knowledge.retrieve_score_threshold_input)
                    .on_input(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveScoreThresholdChanged(
                            value,
                        ))
                    })
                    .padding([9, 10])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fixed(110.0)),
                checkbox(app.knowledge.retrieve_include_context)
                    .label("包含上下文")
                    .on_toggle(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveIncludeContextChanged(
                            value,
                        ))
                    })
                    .style(settings_checkbox_style),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(results).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(12)
        .height(Length::Fill),
    )
    .into()
}

fn build_settings_tab<'a>(app: &'a App, dataset: &'a KnowledgeDatasetDto) -> Element<'a, Message> {
    let mut status_items = column![].spacing(10);
    if let Some(status) = &app.knowledge.runtime_status {
        status_items = status_items
            .push(kv_row("全文检索", support_label(status.full_text)))
            .push(kv_row("向量检索", support_label(status.vector)))
            .push(kv_row("混合检索", support_label(status.hybrid)))
            .push(kv_row("Rerank", support_label(status.rerank)));
        for (key, value) in &status.notes {
            status_items = status_items.push(kv_row(key, value));
        }
    } else {
        status_items = status_items.push(kv_row("运行状态", "检测中"));
    }

    settings_panel(
        column![
            build_embedding_model_section(app),
            kv_row("知识库 ID", &dataset.id),
            kv_row("分段模式", chunking_label(&dataset.chunking_mode)),
            kv_row("索引模式", indexing_label(&dataset.indexing_mode)),
            kv_row("检索模式", retrieval_label(&dataset.retrieval_mode)),
            kv_row("关键词数量", dataset.keyword_count.to_string()),
            kv_row("Top K", dataset.top_k.to_string()),
            kv_row("Score 阈值", score_threshold_label(dataset)),
            kv_row("Rerank 开关", enabled_label(dataset.rerank_enabled)),
            kv_row("Embedding", dataset_embedding_model_label(app, dataset)),
            kv_row("Rerank", dataset.rerank_model.as_deref().unwrap_or("未启用")),
            container(status_items).padding([8, 0]),
        ]
        .spacing(12),
    )
    .into()
}

fn chunk_card(chunk: &KnowledgeChunkDto) -> Element<'_, Message> {
    let score = chunk.score.map(|value| format!("{value:.3}")).unwrap_or_else(|| "-".to_string());
    let mut details = column![text(&chunk.content).size(13)].spacing(8);
    if let Some(matched) = chunk.matched_content.as_deref().filter(|value| !value.trim().is_empty())
    {
        details = details.push(
            column![
                text("匹配内容").size(11).style(settings_muted_text_style),
                text(matched).size(12),
            ]
            .spacing(3),
        );
    }
    if let Some(context) = chunk.context.as_deref().filter(|value| !value.trim().is_empty()) {
        details = details.push(
            column![
                text("上下文").size(11).style(settings_muted_text_style),
                text(context).size(12),
            ]
            .spacing(3),
        );
    }

    container(
        column![
            row![
                text(&chunk.title).size(14).width(Length::Fill),
                settings_value_badge(match_type_label(&chunk.match_type)),
                settings_value_badge(format!("score {score}")),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            details,
        ]
        .spacing(8),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

fn chunk_preview_card(chunk: &KnowledgeChunkDto, selected: bool) -> Element<'_, Message> {
    let mut badges = row![
        settings_value_badge(format!("#{}", chunk.ordinal)),
        settings_value_badge(chunk_type_label(&chunk.chunk_type)),
        settings_value_badge(format!("{}..{}", chunk.start_at, chunk.end_at)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if chunk.parent_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("parent"));
    }
    if chunk.prev_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("prev"));
    }
    if chunk.next_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("next"));
    }

    let title = chunk_preview_title(chunk);
    let preview = chunk.content.lines().next().unwrap_or("").trim();
    let body = column![
        row![
            text(title).size(13).width(Length::Fill),
            settings_value_badge(match_type_label(&chunk.match_type)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(preview).size(12).style(settings_muted_text_style),
        badges,
    ]
    .spacing(7);

    button(
        container(body)
            .padding([10, 12])
            .width(Length::Fill)
            .style(move |theme: &Theme| item_style(theme, selected)),
    )
    .padding(0)
    .width(Length::Fill)
    .style(button::text)
    .on_press(Message::Knowledge(KnowledgeToolMessage::SelectChunk(chunk.id.clone())))
    .into()
}

fn chunk_preview_title(chunk: &KnowledgeChunkDto) -> String {
    let title = chunk.title.trim();
    if !title.is_empty() {
        return title.to_string();
    }
    let first_line = chunk.content.lines().next().unwrap_or("").trim();
    if first_line.is_empty() { format!("分段 {}", chunk.ordinal) } else { first_line.to_string() }
}

fn hash_copy_text(content: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn chunk_content_panel<'a>(
    chunk: &'a KnowledgeChunkDto,
    last_copied_code_hash: Option<u64>,
) -> Element<'a, Message> {
    let copied = last_copied_code_hash == Some(hash_copy_text(&chunk.content));
    let copy_icon = if copied { Icon::Check } else { Icon::Copy };
    let copy_label = if copied { "已复制" } else { "复制" };
    container(
        column![
            row![
                text("完整内容").size(12).style(settings_muted_text_style).width(Length::Fill),
                button(
                    row![
                        icon_svg(copy_icon, 12.0).style(|theme: &Theme, _status| {
                            iced::widget::svg::Style { color: Some(theme.palette().text) }
                        }),
                        text(copy_label).size(12)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .on_press(Message::CopyCode(chunk.content.clone()))
                .padding([6, 10])
                .style(knowledge_toolbar_button_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            knowledge_scrollable(
                container(text(&chunk.content).size(12).style(settings_muted_text_style))
                    .width(Length::Fill)
            )
            .height(Length::Fill),
        ]
        .spacing(10)
        .height(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(chrome_style)
    .into()
}

fn kv_row<'a>(label: &'a str, value: impl ToString + 'a) -> Element<'a, Message> {
    row![
        text(label).size(13).style(settings_muted_text_style).width(Length::Fixed(120.0)),
        text(value.to_string()).size(13).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn embedding_kv_row<'a>(label: &'a str, value: impl ToString + 'a) -> Element<'a, Message> {
    row![
        text(label).size(12).style(settings_muted_text_style).width(Length::Fixed(76.0)),
        text(value.to_string()).size(12).width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn knowledge_scrollable<'a>(
    content: impl Into<Element<'a, Message>> + 'a,
) -> scrollable::Scrollable<'a, Message> {
    scrollable(content).direction(Direction::Vertical(
        Scrollbar::new()
            .width(KNOWLEDGE_SCROLLBAR_WIDTH)
            .scroller_width(KNOWLEDGE_SCROLLBAR_WIDTH)
            .spacing(KNOWLEDGE_SCROLLBAR_GUTTER - KNOWLEDGE_SCROLLBAR_WIDTH),
    ))
}

fn segment_button<'a>(label: &'a str, active: bool, message: Message) -> Element<'a, Message> {
    button(text(label).size(12))
        .on_press(message)
        .padding([7, 11])
        .style(move |theme: &Theme, status| segment_style(theme, status, active))
        .into()
}

fn icon_badge(icon: Icon, active: bool) -> Element<'static, Message> {
    container(icon_svg(icon, 18.0).style(move |theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(if active { theme.palette().primary } else { theme.palette().text }),
    }))
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(if active {
                theme.palette().primary.scale_alpha(0.12)
            } else {
                palette.background.weak.color.scale_alpha(0.42)
            })),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: if active {
                    theme.palette().primary.scale_alpha(0.72)
                } else {
                    palette.background.strong.color.scale_alpha(0.88)
                },
            },
            ..Default::default()
        }
    })
    .into()
}

fn empty_hint<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(13).style(settings_muted_text_style))
        .padding([18, 16])
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(chrome_style)
        .into()
}

fn item_style(theme: &Theme, selected: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let base = if selected {
        theme.palette().primary.scale_alpha(0.10)
    } else {
        palette.background.base.color
    };
    iced::widget::container::Style {
        background: Some(base.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if selected { theme.palette().primary } else { palette.background.strong.color },
        },
        ..Default::default()
    }
}

fn option_card_style(theme: &Theme, active: bool, enabled: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let background = if active {
        theme.palette().primary.scale_alpha(0.10)
    } else {
        palette.background.weak.color.scale_alpha(if enabled { 0.42 } else { 0.22 })
    };
    let border_color = if active {
        theme.palette().primary
    } else {
        palette.background.strong.color.scale_alpha(if enabled { 0.80 } else { 0.42 })
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border {
            radius: 8.0.into(),
            width: if active { 2.0 } else { 1.0 },
            color: border_color,
        },
        text_color: (!enabled).then_some(theme.palette().text.scale_alpha(0.52)),
        ..Default::default()
    }
}

fn chrome_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    iced::widget::container::Style {
        background: Some(
            if is_dark {
                palette.background.base.color.scale_alpha(0.72)
            } else {
                palette.background.weak.color.scale_alpha(0.42)
            }
            .into(),
        ),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 0.80 }),
        },
        ..Default::default()
    }
}

fn centered_knowledge_icon(icon: Icon, size: f32) -> Element<'static, Message> {
    container(icon_svg(icon, size).style(|theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(theme.palette().text),
    }))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn centered_danger_icon(icon: Icon, size: f32) -> Element<'static, Message> {
    container(
        icon_svg(icon, size).style(|_theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(Color::WHITE),
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn themed_inline_icon(icon: Icon, size: f32) -> iced::widget::svg::Svg<'static> {
    icon_svg(icon, size).style(|theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(theme.palette().text),
    })
}

fn primary_action_icon(icon: Icon, size: f32) -> iced::widget::svg::Svg<'static> {
    icon_svg(icon, size)
        .style(|_theme: &Theme, _status| iced::widget::svg::Style { color: Some(Color::WHITE) })
}

fn centered_header_action_content(icon: Icon, label: &'static str) -> Element<'static, Message> {
    container(
        row![themed_inline_icon(icon, 14.0), text(label).size(13)]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .width(Length::Shrink)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .padding([0, 14])
    .into()
}

fn round_danger_icon_btn_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = danger_action_btn_style(theme, status);
    style.border.radius = 999.0.into();
    style
}

fn build_delete_confirm_modal<'a>(
    app: &'a App,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let (title, description, confirm_message) =
        if let Some(document_id) = app.knowledge.pending_delete_document_id.as_deref() {
            let document_name = app
                .knowledge
                .documents
                .iter()
                .find(|document| document.id == document_id)
                .map(|document| document.name.as_str())
                .unwrap_or("当前文档");
            (
                "确认删除文档",
                format!("将删除文档「{document_name}」。此操作不可撤销。"),
                Message::Knowledge(KnowledgeToolMessage::DeleteDocument(document_id.to_string())),
            )
        } else {
            let dataset_name = app
                .knowledge
                .pending_delete_dataset_id
                .as_deref()
                .and_then(|dataset_id| app.knowledge.dataset_by_id(dataset_id))
                .or_else(|| app.knowledge.selected_dataset())
                .map(|dataset| dataset.name.as_str())
                .unwrap_or("当前知识库");
            (
                "确认删除知识库",
                format!("将删除知识库「{dataset_name}」及其内容。此操作不可撤销。"),
                Message::Knowledge(KnowledgeToolMessage::DeleteSelectedDataset),
            )
        };

    let title_row = row![
        column![text(title).size(22), text(description).size(12).style(settings_muted_text_style),]
            .spacing(6)
            .width(Length::Fill),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let actions = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(13))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm))
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text("确认删除").size(13))
            .on_press_maybe((!app.knowledge.deleting).then_some(confirm_message))
            .padding([9, 14])
            .style(danger_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let card = settings_modal_card(
        column![
            title_row,
            container(
                text("删除后无法恢复，请确认当前选择无误。")
                    .size(13)
                    .style(settings_muted_text_style),
            )
            .padding([14, 16])
            .width(Length::Fill),
            actions,
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(520.0))
    .max_width(560.0);

    settings_modal_overlay(base, Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm), card)
}

fn file_drop_zone_style(
    theme: &Theme,
    highlighted: bool,
    has_selection: bool,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let border_color = if highlighted {
        theme.palette().primary
    } else if has_selection {
        palette.primary.base.color.scale_alpha(0.70)
    } else {
        palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 0.78 })
    };
    let background = if highlighted {
        theme.palette().primary.scale_alpha(if is_dark { 0.18 } else { 0.12 })
    } else if has_selection {
        palette.background.weak.color.scale_alpha(if is_dark { 0.72 } else { 0.48 })
    } else if is_dark {
        palette.background.base.color.scale_alpha(0.84)
    } else {
        palette.background.weak.color.scale_alpha(0.34)
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border {
            radius: 18.0.into(),
            width: if highlighted { 2.0 } else { 1.0 },
            color: border_color,
        },
        ..Default::default()
    }
}

fn segment_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let background = if active {
        theme.palette().primary.scale_alpha(0.88)
    } else if hovered {
        if is_dark {
            palette.background.strong.color.scale_alpha(0.88)
        } else {
            palette.background.strong.color.scale_alpha(0.62)
        }
    } else {
        if is_dark {
            palette.background.base.color.scale_alpha(0.72)
        } else {
            palette.background.weak.color.scale_alpha(0.42)
        }
    };
    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: if active { Color::WHITE } else { theme.palette().text },
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if active {
                theme.palette().primary
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 1.0 })
            },
        },
        ..Default::default()
    }
}

fn knowledge_toolbar_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let disabled = matches!(status, iced::widget::button::Status::Disabled);
    let background = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                palette.background.weak.color
            } else {
                Color::WHITE.scale_alpha(0.94)
            }
        }
        iced::widget::button::Status::Pressed => {
            if is_dark {
                palette.background.strong.color.scale_alpha(0.94)
            } else {
                palette.background.weak.color.scale_alpha(0.90)
            }
        }
        iced::widget::button::Status::Disabled => {
            if is_dark {
                palette.background.base.color.scale_alpha(0.32)
            } else {
                palette.background.weak.color.scale_alpha(0.52)
            }
        }
        _ => {
            if is_dark {
                palette.background.base.color.scale_alpha(0.68)
            } else {
                Color::WHITE.scale_alpha(0.72)
            }
        }
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(if disabled { 0.44 } else { 0.92 })
    } else {
        Color::from_rgba8(15, 23, 42, if disabled { 0.08 } else { 0.14 })
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text.scale_alpha(if disabled { 0.42 } else { 0.92 }),
        border: Border { radius: 12.0.into(), width: 1.0, color: border_color },
        ..Default::default()
    }
}

fn is_dark_palette(theme: &Theme) -> bool {
    let bg = theme.palette().background;
    bg.r + bg.g + bg.b < 1.5
}

fn indexing_label(mode: &KnowledgeIndexingMode) -> &'static str {
    match mode {
        KnowledgeIndexingMode::Economy => "经济",
        KnowledgeIndexingMode::HighQuality => "高质量",
    }
}

fn chunking_label(mode: &KnowledgeChunkingMode) -> &'static str {
    match mode {
        KnowledgeChunkingMode::General => "General",
        KnowledgeChunkingMode::ParentChild => "Parent-Child",
        KnowledgeChunkingMode::Qa => "Q&A",
    }
}

fn retrieval_label(mode: &KnowledgeRetrievalMode) -> &'static str {
    match mode {
        KnowledgeRetrievalMode::FullText => "全文检索",
        KnowledgeRetrievalMode::Vector => "向量检索",
        KnowledgeRetrievalMode::Hybrid => "混合检索",
    }
}

fn document_status_label(status: &KnowledgeDocumentStatus) -> &'static str {
    match status {
        KnowledgeDocumentStatus::Pending => "入队",
        KnowledgeDocumentStatus::Processing => "索引中",
        KnowledgeDocumentStatus::Completed => "成功",
        KnowledgeDocumentStatus::Failed => "失败",
        KnowledgeDocumentStatus::Disabled => "停用",
        KnowledgeDocumentStatus::Archived => "归档",
    }
}

fn effective_document_status(document: &KnowledgeDocumentDto) -> KnowledgeDocumentStatus {
    if !document.enabled && document.status != KnowledgeDocumentStatus::Archived {
        KnowledgeDocumentStatus::Disabled
    } else {
        document.status.clone()
    }
}

fn compact_document_count_label(value: u64) -> String {
    compact_count_label(value)
}

fn compact_count_label(value: u64) -> String {
    const THOUSAND: u64 = 1_000;
    const MILLION: u64 = 1_000_000;
    if value < THOUSAND {
        return value.to_string();
    }
    if value < MILLION {
        return compact_decimal(value, THOUSAND, "K");
    }
    compact_decimal(value, MILLION, "M")
}

fn compact_decimal(value: u64, base: u64, suffix: &str) -> String {
    let whole = value / base;
    let decimal = ((value % base) * 10) / base;
    if decimal == 0 { format!("{whole}{suffix}") } else { format!("{whole}.{decimal}{suffix}") }
}

fn import_job_status_label(status: &vw_gateway_client::KnowledgeIndexJobStatus) -> &'static str {
    match status {
        vw_gateway_client::KnowledgeIndexJobStatus::Queued => "入队",
        vw_gateway_client::KnowledgeIndexJobStatus::Processing => "索引中",
        vw_gateway_client::KnowledgeIndexJobStatus::Completed => "成功",
        vw_gateway_client::KnowledgeIndexJobStatus::Failed => "失败",
    }
}

fn chunk_type_label(chunk_type: &KnowledgeChunkType) -> &'static str {
    match chunk_type {
        KnowledgeChunkType::Text => "文本",
        KnowledgeChunkType::ParentText => "父块",
        KnowledgeChunkType::Qa => "Q&A",
    }
}

fn match_type_label(match_type: &KnowledgeMatchType) -> &'static str {
    match match_type {
        KnowledgeMatchType::FullText => "全文",
        KnowledgeMatchType::Vector => "向量",
        KnowledgeMatchType::Hybrid => "混合",
        KnowledgeMatchType::Parent => "父块",
        KnowledgeMatchType::Nearby => "邻近",
    }
}

fn support_label(value: bool) -> &'static str {
    if value { "可用" } else { "不可用" }
}

fn enabled_label(value: bool) -> &'static str {
    if value { "启用" } else { "关闭" }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasetEmbeddingOption {
    selection: KnowledgeDatasetEmbeddingSelection,
    label: String,
}

impl fmt::Display for DatasetEmbeddingOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn build_embedding_model_picker(app: &App) -> Element<'_, Message> {
    let options = dataset_embedding_options(app);
    let selected = options
        .iter()
        .find(|option| option.selection == app.knowledge.dataset_embedding_selection)
        .cloned()
        .or_else(|| options.first().cloned());

    pick_list(options, selected, |option| {
        Message::Knowledge(KnowledgeToolMessage::DatasetEmbeddingSelectionChanged(option.selection))
    })
    .padding([9, 11])
    .text_size(12)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into()
}

fn dataset_embedding_options(app: &App) -> Vec<DatasetEmbeddingOption> {
    let mut options = vec![DatasetEmbeddingOption {
        selection: KnowledgeDatasetEmbeddingSelection::FollowGlobal,
        label: "跟随全局配置（推荐）".to_string(),
    }];
    let Some(status) = app.knowledge.embedding_status.as_ref() else {
        return options;
    };
    let model = status.model.trim();
    if model.is_empty() || !status.vector_available {
        return options;
    }
    let current_label = app
        .knowledge
        .embedding_presets
        .iter()
        .find(|preset| preset.provider_id == status.provider_id && preset.model == status.model)
        .map(embedding_preset_label)
        .unwrap_or_else(|| {
            format!(
                "{} · {} / {}维",
                embedding_status_provider_label(Some(status)),
                model,
                status.dimensions
            )
        });
    options.push(DatasetEmbeddingOption {
        selection: KnowledgeDatasetEmbeddingSelection::LockCurrent,
        label: format!("锁定当前 · {current_label}"),
    });
    options
}

fn embedding_vector_available(app: &App) -> bool {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| status.vector_available)
        .or_else(|| app.knowledge.runtime_status.as_ref().map(|status| status.vector))
        .unwrap_or(false)
}

fn embedding_hybrid_available(app: &App) -> bool {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| status.hybrid_available)
        .or_else(|| app.knowledge.runtime_status.as_ref().map(|status| status.hybrid))
        .unwrap_or(false)
}

fn embedding_status_provider_label(status: Option<&KnowledgeEmbeddingStatusDto>) -> String {
    status
        .map(|status| status.provider_id.trim())
        .filter(|provider| !provider.is_empty())
        .unwrap_or("检测中")
        .to_string()
}

fn embedding_status_model_label(status: Option<&KnowledgeEmbeddingStatusDto>) -> String {
    status
        .map(|status| status.model.trim())
        .filter(|model| !model.is_empty())
        .unwrap_or("检测中")
        .to_string()
}

fn embedding_status_dimensions_label(status: Option<&KnowledgeEmbeddingStatusDto>) -> String {
    status.map(|status| format!("{} 维", status.dimensions)).unwrap_or_else(|| "检测中".to_string())
}

fn embedding_status_route_label(status: Option<&KnowledgeEmbeddingStatusDto>) -> String {
    status
        .and_then(|status| status.route_hint.as_deref())
        .map(str::trim)
        .filter(|hint| !hint.is_empty())
        .unwrap_or("默认")
        .to_string()
}

fn embedding_preset_label(preset: &KnowledgeEmbeddingPresetDto) -> String {
    let provider = preset.provider_id.to_ascii_lowercase();
    let label = preset.label.to_ascii_lowercase();
    let family = if preset.id.starts_with("alibaba")
        || provider.contains("alibaba")
        || label.contains("dashscope")
    {
        "阿里百炼/DashScope"
    } else if preset.id.contains("doubao")
        || preset.id.contains("volcengine")
        || provider.contains("volces")
        || label.contains("doubao")
        || label.contains("volcengine")
    {
        "豆包/火山方舟"
    } else {
        preset.label.trim()
    };
    format!("{family} · {} / {}维", preset.model.trim(), preset.dimensions)
}

fn embedding_test_result_label(result: Option<&KnowledgeEmbeddingTestResponse>) -> String {
    let Some(result) = result else {
        return "尚未测试".to_string();
    };
    let status = if result.ok { "通过" } else { "失败" };
    let dimensions = result
        .observed_dimensions
        .map(|value| format!("{value} 维"))
        .unwrap_or_else(|| format!("配置 {} 维", result.configured_dimensions));
    let latency = result
        .latency_ms
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| "耗时未知".to_string());
    if result.ok {
        format!("{status} · {dimensions} · {latency}")
    } else {
        format!("{status} · {} · {latency}", result.message)
    }
}

fn dataset_embedding_model_label(app: &App, _dataset: &KnowledgeDatasetDto) -> String {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| format!("{} / {}维", status.model, status.dimensions))
        .unwrap_or_else(|| "跟随记忆配置/嵌入路由".to_string())
}

fn score_threshold_label(dataset: &KnowledgeDatasetDto) -> String {
    if dataset.score_threshold_enabled {
        format!("{:.2}", dataset.score_threshold)
    } else {
        "关闭".to_string()
    }
}

fn format_time(ms: u64) -> String {
    use chrono::{Local, TimeZone};

    if ms == 0 {
        return "-".to_string();
    }
    let Ok(timestamp_ms) = i64::try_from(ms) else {
        return "-".to_string();
    };
    let Some(dt) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return "-".to_string();
    };
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn document_row_style(theme: &Theme, selected: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(
            if selected {
                theme.palette().primary.scale_alpha(0.10)
            } else if is_dark_palette(theme) {
                palette.background.base.color.scale_alpha(0.26)
            } else {
                Color::WHITE.scale_alpha(0.68)
            }
            .into(),
        ),
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn document_title_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: if hovered || selected {
            theme.palette().primary
        } else {
            theme.palette().text
        },
        border: Border { radius: 6.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

fn document_status_badge_style(
    theme: &Theme,
    document: &KnowledgeDocumentDto,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let (background, border_color, text_color) = if !document.enabled {
        (
            palette.background.strong.color.scale_alpha(0.18),
            palette.background.strong.color.scale_alpha(0.34),
            theme.palette().text.scale_alpha(0.72),
        )
    } else {
        match document.status {
            KnowledgeDocumentStatus::Completed => (
                Color::from_rgb8(34, 197, 94).scale_alpha(0.16),
                Color::from_rgb8(34, 197, 94).scale_alpha(0.38),
                Color::from_rgb8(22, 163, 74),
            ),
            KnowledgeDocumentStatus::Processing => (
                Color::from_rgb8(59, 130, 246).scale_alpha(0.16),
                Color::from_rgb8(59, 130, 246).scale_alpha(0.34),
                Color::from_rgb8(37, 99, 235),
            ),
            KnowledgeDocumentStatus::Pending => (
                Color::from_rgb8(245, 158, 11).scale_alpha(0.18),
                Color::from_rgb8(245, 158, 11).scale_alpha(0.36),
                Color::from_rgb8(217, 119, 6),
            ),
            KnowledgeDocumentStatus::Failed => (
                Color::from_rgb8(239, 68, 68).scale_alpha(0.16),
                Color::from_rgb8(239, 68, 68).scale_alpha(0.34),
                Color::from_rgb8(220, 38, 38),
            ),
            KnowledgeDocumentStatus::Disabled => (
                palette.background.strong.color.scale_alpha(0.18),
                palette.background.strong.color.scale_alpha(0.34),
                theme.palette().text.scale_alpha(0.72),
            ),
            KnowledgeDocumentStatus::Archived => (
                palette.background.strong.color.scale_alpha(0.18),
                palette.background.strong.color.scale_alpha(0.34),
                theme.palette().text.scale_alpha(0.72),
            ),
        }
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border { radius: 999.0.into(), width: 1.0, color: border_color },
        text_color: Some(text_color),
        ..Default::default()
    }
}

fn knowledge_tooltip_content<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        text(label)
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .padding([6, 8])
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Color::from_rgba8(15, 23, 42, 0.94).into()),
        text_color: Some(Color::WHITE),
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    })
    .into()
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod knowledge_tests;
