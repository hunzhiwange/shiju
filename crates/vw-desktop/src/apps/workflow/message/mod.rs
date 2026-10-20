//! # Workflow 消息模块
//!
//! 该模块定义 workflow 的消息类型，并将 UI、文件、导出和编辑操作分发到状态更新逻辑。

use super::model::{
    LoadedWorkflow, WorkflowAppMeta, WorkflowConnectionEndpoint, create_blank_workflow,
    load_document_from_path, load_document_from_value, suggested_workflow_file_name,
};
#[cfg(not(target_arch = "wasm32"))]
use super::model::{WorkflowViewport, load_document_from_text, serialize_workflow_yaml};
use super::state::{
    WorkflowAppEditorMode, WorkflowCanvasContextMenuTarget, WorkflowDataNodeEdit,
    WorkflowHttpRequestEdit, WorkflowNodeEditorTab, WorkflowNodeFieldEdit, WorkflowRunInputDraft,
    WorkflowSavedAppSummary, WorkflowStructuredNodeEdit, WorkflowVariablePanelKind,
};
use crate::app::{App, Message};
use iced::widget::text_editor;
use iced::{Point, Task, Vector};
#[cfg(not(target_arch = "wasm32"))]
use vw_gateway_client::{WorkflowRecordSummary, WorkflowRecordUpsertBody};

mod app;
mod canvas;
mod document;
mod node;
mod run;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
#[cfg(test)]
#[path = "canvas_tests.rs"]
mod canvas_tests;
#[cfg(test)]
#[path = "document_tests.rs"]
mod document_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "node_tests.rs"]
mod node_tests;
#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
fn render_workflow_svg_to_png(svg_data: &str) -> Option<Vec<u8>> {
    use resvg::usvg::{self, Tree};
    use tiny_skia::{Pixmap, Transform};

    let mut opt = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    opt.fontdb = std::sync::Arc::new(fontdb);

    let tree = Tree::from_str(svg_data, &opt).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = Pixmap::new(size.width(), size.height())?;

    let mut pm = pixmap.as_mut();
    resvg::render(&tree, Transform::default(), &mut pm);
    pixmap.encode_png().ok()
}

#[derive(Debug, Clone)]
pub enum WorkflowMessage {
    LoadSavedApps,
    LoadSavedAppsFinished(Result<Vec<WorkflowSavedAppSummary>, String>),
    OpenSavedApp(String),
    OpenSavedAppFinished(Result<LoadedWorkflow, String>),
    ShowSavedApps,
    SavedAppSearchChanged(String),
    ToggleSavedAppActions(String),
    CloseSavedAppActions,
    CopySavedAppUuid(String),
    ClearCopiedSavedAppUuid(String),
    OpenSavedAppRunPrompt(String),
    SavedAppRunPromptLoaded { uuid: String, result: Result<Vec<WorkflowRunInputDraft>, String> },
    SavedAppRunQueryChanged(String),
    SavedAppRunInputChanged(usize, String),
    SavedAppRunInputToggled(usize, bool),
    CancelSavedAppRunPrompt,
    SubmitSavedAppRunPrompt,
    SavedAppRunEvent { uuid: String, event: vw_gateway_client::GatewayWorkflowStreamEvent },
    SavedAppRunFinished { uuid: String, result: Result<(), String> },
    CancelSavedAppRun,
    RetrySavedAppRunPreview,
    CloseSavedAppRunPreview,
    OpenSavedAppRunNodeDetail(usize),
    OpenSavedAppRunOutputDetail,
    SavedAppRunDetailAction(text_editor::Action),
    CloseSavedAppRunDetail,
    CopySavedAppRunError,
    ClearSavedAppRunErrorCopied,
    RequestDeleteSavedApp(String),
    CancelDeleteSavedApp,
    DeleteSavedApp(String),
    DeleteSavedAppFinished(Result<String, String>),
    OpenInlineYaml { workflow_yaml: String, focus_node_id: Option<String> },
    OpenFile,
    OpenFileFinished(Result<Option<LoadedWorkflow>, String>),
    SelectApp(String),
    OpenCreateAppEditor,
    OpenEditAppEditor(Option<String>),
    CloseAppEditor,
    AppEditorNameChanged(String),
    AppEditorDescriptionChanged(String),
    AppEditorIconChanged(String),
    AppEditorUseIconAsAnswerIconChanged(bool),
    AppEditorMaxActiveRequestsChanged(String),
    OrganizeActiveApp,
    SubmitAppEditor,
    ToggleQuickInsertPanel,
    InsertSuggestedNode(String),
    OpenCreateNodeEditor(String),
    OpenCreateNodeEditorAt(String, Point),
    CreateContextNode(String),
    OpenDownstreamNodePicker(String),
    OpenEditNodeEditor(Option<String>),
    CloseNodeEditor,
    NodeEditorTabSelected(WorkflowNodeEditorTab),
    NodeEditorTitleChanged(String),
    NodeEditorDescriptionChanged(String),
    NodeEditorDescriptionAction(text_editor::Action),
    NodeEditorScrollCaptured,
    NodeEditorStartAddVariable,
    NodeEditorStartVariableHovered(Option<usize>),
    NodeEditorStartRemoveVariable(usize),
    NodeEditorStartSelectVariable(usize),
    NodeEditorStartCloseVariableEditor,
    NodeEditorStartSubmitVariableEditor,
    NodeEditorStartVariableEditorLabelChanged(String),
    NodeEditorStartVariableEditorNameChanged(String),
    NodeEditorStartVariableEditorTypeChanged(String),
    NodeEditorStartVariableEditorRequiredChanged(bool),
    NodeEditorStartVariableEditorHiddenChanged(bool),
    NodeEditorStartVariableEditorDefaultChanged(String),
    NodeEditorStartVariableEditorDefaultAction(text_editor::Action),
    NodeEditorStartVariableEditorMaxLengthChanged(String),
    NodeEditorStartVariableEditorAddOption,
    NodeEditorStartVariableEditorRemoveOption(usize),
    NodeEditorStartVariableEditorOptionChanged(usize, String),
    NodeEditorStartVariableEditorToggleFileType(String),
    NodeEditorStartVariableEditorExtensionsChanged(String),
    NodeEditorStartVariableEditorUploadMethodChanged(String),
    NodeEditorStartVariableEditorPickDefaultFile,
    NodeEditorStartVariableEditorPickDefaultFileFinished(Result<Option<String>, String>),
    NodeEditorStartVariableEditorRemoveDefaultFile(usize),
    NodeEditorStartVariableEditorOpenDefaultFileUrlInput,
    NodeEditorStartVariableEditorCloseDefaultFileUrlInput,
    NodeEditorStartVariableEditorDefaultFileUrlChanged(String),
    NodeEditorStartVariableEditorSubmitDefaultFileUrl,
    NodeEditorStartVariableLabelChanged(usize, String),
    NodeEditorStartVariableNameChanged(usize, String),
    NodeEditorStartVariableTypeChanged(usize, String),
    NodeEditorStartVariableRequiredChanged(usize, bool),
    NodeEditorStartVariableDefaultChanged(usize, String),
    NodeEditorStartVariablePlaceholderChanged(usize, String),
    NodeEditorStartVariableHintChanged(usize, String),
    NodeEditorStartVariableMaxLengthChanged(usize, String),
    InsertDownstreamNode(String, String),
    InsertDownstreamNodeFromHandle(String, String, String),
    NodeEditorShowRawDataEditorChanged(bool),
    NodeEditorIfElseAddCase,
    NodeEditorIfElseCaseLogicalOperatorChanged(usize, String),
    NodeEditorIfElseAddCondition(usize),
    NodeEditorIfElseRemoveCondition(usize, usize),
    NodeEditorIfElseConditionSelectorChanged(usize, usize, String),
    NodeEditorIfElseConditionOperatorChanged(usize, usize, String),
    NodeEditorIfElseConditionValueChanged(usize, usize, String),
    NodeEditorIfElseConditionVarTypeChanged(usize, usize, String),
    NodeEditorKnowledgeQuerySelectorChanged(String),
    NodeEditorKnowledgeQueryAttachmentSelectorChanged(String),
    NodeEditorKnowledgeDatasetIdsChanged(String),
    NodeEditorKnowledgeRetrievalModeChanged(String),
    NodeEditorKnowledgeTopKChanged(String),
    NodeEditorKnowledgeScoreThresholdEnabledChanged(bool),
    NodeEditorKnowledgeScoreThresholdChanged(String),
    NodeEditorKnowledgeRerankingEnabledChanged(bool),
    NodeEditorKnowledgeRerankingModelAction(text_editor::Action),
    NodeEditorKnowledgeMetadataFilteringModeChanged(String),
    NodeEditorKnowledgeMetadataModelConfigAction(text_editor::Action),
    NodeEditorKnowledgeSingleModelProviderChanged(String),
    NodeEditorKnowledgeSingleModelNameChanged(String),
    NodeEditorKnowledgeSingleModelModeChanged(String),
    NodeEditorToolProviderIdChanged(String),
    NodeEditorToolProviderTypeChanged(String),
    NodeEditorToolProviderNameChanged(String),
    NodeEditorToolNameChanged(String),
    NodeEditorToolLabelChanged(String),
    NodeEditorToolDescriptionChanged(String),
    NodeEditorToolCredentialIdChanged(String),
    NodeEditorToolPluginUniqueIdentifierChanged(String),
    NodeEditorToolParametersAction(text_editor::Action),
    NodeEditorToolConfigurationsAction(text_editor::Action),
    NodeEditorAgentStrategyProviderChanged(String),
    NodeEditorAgentStrategyNameChanged(String),
    NodeEditorAgentStrategyLabelChanged(String),
    NodeEditorAgentPluginUniqueIdentifierChanged(String),
    NodeEditorAgentOutputSchemaAction(text_editor::Action),
    NodeEditorAgentParametersAction(text_editor::Action),
    NodeEditorAgentMemoryEnabledChanged(bool),
    NodeEditorAgentMemoryWindowSizeChanged(String),
    NodeEditorAgentMemoryPromptAction(text_editor::Action),
    NodeEditorLlmProviderChanged(String),
    NodeEditorLlmModelNameChanged(String),
    NodeEditorLlmModelModeChanged(String),
    NodeEditorLlmEnableThinkingChanged(bool),
    NodeEditorLlmCompletionParamsAction(text_editor::Action),
    NodeEditorLlmStructuredOutputAction(text_editor::Action),
    NodeEditorLlmMemoryAction(text_editor::Action),
    NodeEditorLlmContextEnabledChanged(bool),
    NodeEditorLlmContextSelectorChanged(String),
    NodeEditorLlmSystemPromptAction(text_editor::Action),
    NodeEditorLlmUserPromptAction(text_editor::Action),
    NodeEditorLlmVisionEnabledChanged(bool),
    NodeEditorLlmVisionSelectorChanged(String),
    NodeEditorAnswerAction(text_editor::Action),
    NodeEditorEndFieldEdit(WorkflowNodeFieldEdit),
    NodeEditorTemplateFieldEdit(WorkflowNodeFieldEdit),
    NodeEditorTemplateAction(text_editor::Action),
    NodeEditorDocumentSelectorChanged(String),
    NodeEditorStructuredFieldEdit(WorkflowStructuredNodeEdit),
    NodeEditorClassifierAddClass,
    NodeEditorClassifierRemoveClass(usize),
    NodeEditorClassifierClassIdChanged(usize, String),
    NodeEditorClassifierClassNameChanged(usize, String),
    NodeEditorExtractorAddParameter,
    NodeEditorExtractorRemoveParameter(usize),
    NodeEditorHttpRequestEdit(WorkflowHttpRequestEdit),
    NodeEditorDataNodeEdit(WorkflowDataNodeEdit),
    NodeEditorCodeLanguageChanged(String),
    NodeEditorCodeAddInputVariable,
    NodeEditorCodeRemoveInputVariable(usize),
    NodeEditorCodeInputVariableNameChanged(usize, String),
    NodeEditorCodeInputVariableSelectorChanged(usize, String, String),
    NodeEditorCodeAddOutputVariable,
    NodeEditorCodeRemoveOutputVariable(usize),
    NodeEditorCodeOutputNameChanged(usize, String),
    NodeEditorCodeOutputTypeChanged(usize, String),
    NodeEditorCodeRetryEnabledChanged(bool),
    NodeEditorCodeRetryMaxRetriesChanged(u8),
    NodeEditorCodeRetryIntervalChanged(u16),
    NodeEditorCodeErrorStrategyChanged(String),
    NodeEditorCodeAction(text_editor::Action),
    NodeEditorCodeDefaultValueAction(text_editor::Action),
    NodeEditorDataAction(text_editor::Action),
    NodeEditorDslOpenContextMenu { x: f32, y: f32 },
    NodeEditorDslCloseContextMenu,
    NodeEditorDslUndo,
    NodeEditorDslRedo,
    NodeEditorDslContextMenuCopy,
    NodeEditorDslContextMenuCut,
    NodeEditorDslContextMenuPaste,
    NodeEditorDslContextMenuDelete,
    SubmitNodeEditor,
    OpenVariablePanel(WorkflowVariablePanelKind),
    CloseVariablePanel,
    OpenCreateEnvironmentVariableEditor,
    OpenEditEnvironmentVariableEditor(String),
    OpenCreateConversationVariableEditor,
    OpenEditConversationVariableEditor(String),
    CloseVariableEditor,
    VariableEditorNameChanged(String),
    VariableEditorDescriptionChanged(String),
    VariableEditorTypeChanged(String),
    VariableEditorValueAction(text_editor::Action),
    SubmitVariableEditor,
    DeleteEnvironmentVariable(String),
    DeleteConversationVariable(String),
    ToggleActionMenu,
    CloseFloatingPanels,
    SaveActiveApp,
    SaveActiveAppAs,
    SaveActiveAppFinished(Result<Option<WorkflowSaveTarget>, String>),
    ExportPng,
    ExportJpeg,
    ExportSvg,
    ExportFinished(Result<(), String>),
    Reload,
    SelectNode(String),
    SelectEdge(String),
    ClearSelection,
    PanBy(Vector),
    Zoom(f32, Option<Point>),
    ZoomSet(f32),
    ZoomFit,
    ToggleZoomMenu,
    NodeDragStart(String),
    NodeDragged(String, Vector),
    FinishNodeDrag,
    StartConnection(WorkflowConnectionEndpoint, Point),
    UpdateConnectionCursor(Point),
    FinishConnection(WorkflowConnectionEndpoint),
    CancelConnection,
    CancelInteraction,
    OpenCanvasContextMenu(WorkflowCanvasContextMenuTarget, Point, Point),
    CanvasContextMenuScrollCaptured,
    CloseCanvasContextMenu,
    DuplicateSelectedNode,
    DeleteSelectedNode,
    DeleteSelectedEdge,
    DeleteNodeById(String),
    DeleteEdgeById(String),
    JumpToNode(String),
    Undo,
    Redo,
    DismissError,
}

#[derive(Debug, Clone)]
pub enum WorkflowSaveTarget {
    LocalUuid(String),
    FilePath(String),
}

pub fn update(app: &mut App, message: WorkflowMessage) -> Task<Message> {
    app::handle(app, message.clone())
        .or_else(|| run::handle(app, message.clone()))
        .or_else(|| node::handle(app, message.clone()))
        .or_else(|| document::handle(app, message.clone()))
        .or_else(|| canvas::handle(app, message))
        .unwrap_or_else(Task::none)
}

pub(super) fn apply_loaded(app: &mut App, loaded: LoadedWorkflow) {
    app.workflow_state.apply_loaded(loaded, app.window_size);
    crate::apps::workflow::sync_top_tab(app);
}

pub(crate) fn load_saved_apps_task() -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        Task::perform(
            async { Err("Web 平台暂不支持读取本地 Workflow 应用列表".to_string()) },
            |res| Message::WorkflowTool(WorkflowMessage::LoadSavedAppsFinished(res)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let client = crate::app::config::gateway_client()?;
                let records = client.workflow_applications_list().await?;
                Ok(records.into_iter().map(saved_app_summary_from_record).collect())
            },
            |res| Message::WorkflowTool(WorkflowMessage::LoadSavedAppsFinished(res)),
        )
    }
}

pub(super) fn open_saved_app_task(uuid: String) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = uuid;
        Task::perform(
            async { Err("Web 平台暂不支持读取本地 Workflow 应用".to_string()) },
            |res| Message::WorkflowTool(WorkflowMessage::OpenSavedAppFinished(res)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let client = crate::app::config::gateway_client()?;
                let record = client.workflow_application_get(&uuid).await?;
                let mut loaded = load_document_from_text(None, record.workflow_yaml)?;
                loaded.local_uuid = Some(record.uuid);
                if !record.name.trim().is_empty() {
                    loaded.source_name = record.name.clone();
                    loaded.app_meta.name = record.name.clone();
                    loaded.document.name = record.name;
                }
                if !record.description.trim().is_empty() {
                    loaded.app_meta.description = record.description;
                }
                Ok(loaded)
            },
            |res| Message::WorkflowTool(WorkflowMessage::OpenSavedAppFinished(res)),
        )
    }
}

pub(super) fn delete_saved_app_task(uuid: String) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = uuid;
        Task::perform(
            async { Err("Web 平台暂不支持删除本地 Workflow 应用".to_string()) },
            |res| Message::WorkflowTool(WorkflowMessage::DeleteSavedAppFinished(res)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let client = crate::app::config::gateway_client()?;
                let response = client.workflow_application_delete(&uuid).await?;
                if response.deleted {
                    Ok(response.uuid)
                } else {
                    Err(format!("未找到可删除的 Workflow 应用: {uuid}"))
                }
            },
            |res| Message::WorkflowTool(WorkflowMessage::DeleteSavedAppFinished(res)),
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn saved_app_summary_from_record(record: WorkflowRecordSummary) -> WorkflowSavedAppSummary {
    WorkflowSavedAppSummary {
        uuid: record.uuid,
        name: record.name,
        description: record.description,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
    }
}

pub(super) fn save_active_app(app: &mut App, force_picker: bool) -> Task<Message> {
    let Some(entry) = app.workflow_state.active_entry_snapshot() else {
        return Task::none();
    };

    save_entry(entry, force_picker)
}

pub(super) fn suggested_new_node_position(app: &App) -> Point {
    let zoom = app.workflow_state.zoom.max(0.0001);
    let center_screen = Point::new(app.window_size.0 * 0.52, app.window_size.1 * 0.5);
    let offset = (app.workflow_state.document.nodes.len() % 6) as f32 * 28.0;
    Point::new(
        (center_screen.x - app.workflow_state.pan.x) / zoom + offset,
        (center_screen.y - app.workflow_state.pan.y) / zoom + offset,
    )
}

pub(super) fn save_entry(
    entry: super::state::WorkflowAppEntry,
    force_picker: bool,
) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (entry, force_picker);
        Task::perform(
            async { Err("Web 平台暂不支持保存本地工作流文件".to_string()) },
            |res| Message::WorkflowTool(WorkflowMessage::SaveActiveAppFinished(res)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let content = serialize_workflow_yaml(
                    &entry.meta,
                    &entry.document,
                    &entry.environment_variables,
                    &entry.conversation_variables,
                    &entry.raw_root,
                    WorkflowViewport { x: entry.pan.x, y: entry.pan.y, zoom: entry.zoom },
                )?;

                if !force_picker {
                    let client = crate::app::config::gateway_client()?;
                    let body = WorkflowRecordUpsertBody {
                        uuid: entry.local_uuid.clone(),
                        name: entry.meta.name.clone(),
                        description: entry.meta.description.clone(),
                        workflow_yaml: content,
                    };
                    let record = match entry.local_uuid.as_deref() {
                        Some(uuid) => client.workflow_application_update(uuid, &body).await?,
                        None => client.workflow_application_create(&body).await?,
                    };
                    return Ok(Some(WorkflowSaveTarget::LocalUuid(record.uuid)));
                }

                let default_name = suggested_workflow_file_name(&entry.meta.name);
                let path = if force_picker || entry.source_path.is_none() {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Dify Workflow", &["yml", "yaml"])
                        .set_file_name(&default_name)
                        .save_file()
                        .await;

                    let Some(file) = file else {
                        return Ok(None);
                    };

                    file.path().to_string_lossy().to_string()
                } else {
                    entry.source_path.clone().unwrap_or(default_name)
                };

                std::fs::write(&path, content)
                    .map_err(|error| format!("写入工作流文件失败: {error}"))?;
                Ok(Some(WorkflowSaveTarget::FilePath(path)))
            },
            |res| Message::WorkflowTool(WorkflowMessage::SaveActiveAppFinished(res)),
        )
    }
}

pub(super) fn open_file() -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        Task::perform(
            async { Err("Web 平台暂不支持本地工作流文件选择".to_string()) },
            |res| Message::WorkflowTool(WorkflowMessage::OpenFileFinished(res)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("Dify Workflow", &["yml", "yaml", "json"])
                    .pick_file()
                    .await;

                let Some(file) = file else {
                    return Ok(None);
                };

                let path = file.path().to_string_lossy().to_string();
                let bytes = file.read().await;
                let text = String::from_utf8_lossy(&bytes).to_string();
                let loaded = load_document_from_text(Some(path), text)?;

                Ok(Some(loaded))
            },
            |res| Message::WorkflowTool(WorkflowMessage::OpenFileFinished(res)),
        )
    }
}

pub(super) fn suggested_export_file_name(title: &str, extension: &str) -> String {
    let default_name = suggested_workflow_file_name(title);
    let stem = default_name.strip_suffix(".yml").unwrap_or(default_name.as_str());
    format!("{stem}.{extension}")
}

pub(super) fn export_svg(app: &mut App) -> Task<Message> {
    if app.workflow_state.active_app_id.is_none() {
        return Task::none();
    }

    app.workflow_state.close_floating_panels();
    let file_name = suggested_export_file_name(app.workflow_state.title(), "svg");
    let svg_data = crate::apps::workflow::canvas::export_svg(&app.workflow_state.document);

    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = rfd::AsyncFileDialog::new().set_file_name(&file_name).save_file().await;
                if let Some(file) = file {
                    file.write(svg_data.as_bytes()).await.map_err(|error| error.to_string())?;
                    let _ = open::that(file.path());
                }
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = (file_name, svg_data);
                Err("Web 平台暂不支持导出工作流图片".to_string())
            }
        },
        |result| Message::WorkflowTool(WorkflowMessage::ExportFinished(result)),
    )
}

pub(super) fn export_png(app: &mut App) -> Task<Message> {
    if app.workflow_state.active_app_id.is_none() {
        return Task::none();
    }

    app.workflow_state.close_floating_panels();
    let file_name = suggested_export_file_name(app.workflow_state.title(), "png");
    let svg_data = crate::apps::workflow::canvas::export_svg(&app.workflow_state.document);

    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = rfd::AsyncFileDialog::new().set_file_name(&file_name).save_file().await;
                if let Some(file) = file {
                    let png = render_workflow_svg_to_png(&svg_data)
                        .ok_or_else(|| "导出 PNG 失败".to_string())?;
                    file.write(&png).await.map_err(|error| error.to_string())?;
                    let _ = open::that(file.path());
                }
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = (file_name, svg_data);
                Err("Web 平台暂不支持导出工作流图片".to_string())
            }
        },
        |result| Message::WorkflowTool(WorkflowMessage::ExportFinished(result)),
    )
}

pub(super) fn export_jpeg(app: &mut App) -> Task<Message> {
    if app.workflow_state.active_app_id.is_none() {
        return Task::none();
    }

    app.workflow_state.close_floating_panels();
    let file_name = suggested_export_file_name(app.workflow_state.title(), "jpg");
    let svg_data = crate::apps::workflow::canvas::export_svg(&app.workflow_state.document);

    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = rfd::AsyncFileDialog::new().set_file_name(&file_name).save_file().await;
                if let Some(file) = file {
                    let png = render_workflow_svg_to_png(&svg_data)
                        .ok_or_else(|| "导出 JPEG 失败".to_string())?;
                    let image = image::load_from_memory(&png)
                        .map_err(|error| format!("解码 PNG 失败: {error}"))?;
                    let rgb = image::DynamicImage::ImageRgba8(image.to_rgba8()).into_rgb8();
                    let mut jpeg_data = std::io::Cursor::new(Vec::new());
                    let mut encoder =
                        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 90);
                    encoder
                        .encode(
                            rgb.as_raw(),
                            rgb.width(),
                            rgb.height(),
                            image::ExtendedColorType::Rgb8,
                        )
                        .map_err(|error| format!("编码 JPEG 失败: {error}"))?;
                    file.write(&jpeg_data.into_inner()).await.map_err(|error| error.to_string())?;
                    let _ = open::that(file.path());
                }
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = (file_name, svg_data);
                Err("Web 平台暂不支持导出工作流图片".to_string())
            }
        },
        |result| Message::WorkflowTool(WorkflowMessage::ExportFinished(result)),
    )
}

pub(super) fn export_finished(app: &mut App, result: Result<(), String>) {
    match result {
        Ok(()) => {
            app.workflow_state.status_message = Some("已导出工作流图片".to_string());
        }
        Err(error) => app.workflow_state.set_error(error),
    }
}
