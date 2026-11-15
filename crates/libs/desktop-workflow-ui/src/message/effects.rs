//! Workflow file, gateway and image export tasks. Desktop injects gateway access.
use super::{WorkflowMessage, WorkflowSaveTarget};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;
use iced::Task;
use vw_api_workflow::{WorkflowRecordSummary, WorkflowRecordUpsertBody};

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

pub fn load_saved_apps_task(gateway_client: fn() -> Result<gateway_workflow::GatewayWorkflowClient, String>, tenant: vw_api_transport::TenantScope) -> Task<WorkflowMessage> {
    let client = gateway_client();
    Task::perform(
        async move {
            let client = client?;
            let records = client
                .workflow_applications_list()
                .await?;
            Ok(records.into_iter().map(saved_app_summary_from_record).collect())
        },
        move |result| {
            WorkflowMessage::ScopedSavedAppsLoaded {
                tenant: tenant.clone(),
                result,
            }
        },
    )
}

pub fn open_saved_app_task(gateway_client: fn() -> Result<gateway_workflow::GatewayWorkflowClient, String>, uuid: String) -> Task<WorkflowMessage> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (uuid, gateway_client);
        Task::perform(
            async {
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持读取本地 Workflow 应用",
                    vec![],
                ))
            },
            |res| WorkflowMessage::OpenSavedAppFinished(res),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let client = gateway_client()?;
                let record =
                    client
                        .workflow_application_get(&uuid)
                        .await?;
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
            |res| WorkflowMessage::OpenSavedAppFinished(res),
        )
    }
}

pub fn delete_saved_app_task(gateway_client: fn() -> Result<gateway_workflow::GatewayWorkflowClient, String>, uuid: String) -> Task<WorkflowMessage> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (uuid, gateway_client);
        Task::perform(
            async {
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持删除本地 Workflow 应用",
                    vec![],
                ))
            },
            |res| WorkflowMessage::DeleteSavedAppFinished(res),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                let client = gateway_client()?;
                let response =
                    client
                        .workflow_application_delete(&uuid)
                        .await?;
                if response.deleted {
                    Ok(response.uuid)
                } else {
                    Err(desktop_i18n::workflow::Feedback::new(
                        "未找到可删除的 Workflow 应用: %s",
                        vec![uuid],
                    ))
                }
            },
            |res| WorkflowMessage::DeleteSavedAppFinished(res),
        )
    }
}

fn saved_app_summary_from_record(record: WorkflowRecordSummary) -> WorkflowSavedAppSummary {
    WorkflowSavedAppSummary {
        uuid: record.uuid,
        name: record.name,
        description: record.description,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
    }
}

pub fn save_entry(gateway_client: fn() -> Result<gateway_workflow::GatewayWorkflowClient, String>,
    entry: desktop_workflow_state::state::WorkflowAppEntry,
    force_picker: bool,
) -> Task<WorkflowMessage> {
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
                let client = gateway_client()?;
                let body = WorkflowRecordUpsertBody {
                    uuid: entry.local_uuid.clone(),
                    name: entry.meta.name.clone(),
                    description: entry.meta.description.clone(),
                    workflow_yaml: content,
                };
                let record = match entry.local_uuid.as_deref() {
                    Some(uuid) => {
                        client
                            .workflow_application_update(uuid, &body)
                            .await?
                    }
                    None => {
                        client
                            .workflow_application_create(&body)
                            .await?
                    }
                };
                return Ok(Some(WorkflowSaveTarget::LocalUuid(record.uuid)));
            }

            #[cfg(target_arch = "wasm32")]
            {
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持保存本地工作流文件",
                    vec![],
                ))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
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

                std::fs::write(&path, content).map_err(|error| {
                    desktop_i18n::workflow::Feedback::new(
                        "写入工作流文件失败: %s",
                        vec![error.to_string()],
                    )
                })?;
                Ok(Some(WorkflowSaveTarget::FilePath(path)))
            }
        },
        |res| WorkflowMessage::SaveActiveAppFinished(res),
    )
}

pub fn open_file() -> Task<WorkflowMessage> {
    #[cfg(target_arch = "wasm32")]
    {
        Task::perform(
            async {
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持本地工作流文件选择",
                    vec![],
                ))
            },
            |res| WorkflowMessage::OpenFileFinished(res),
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
            |res| WorkflowMessage::OpenFileFinished(res),
        )
    }
}

pub fn suggested_export_file_name(title: &str, extension: &str) -> String {
    let default_name = suggested_workflow_file_name(title);
    let stem = default_name.strip_suffix(".yml").unwrap_or(default_name.as_str());
    format!("{stem}.{extension}")
}

pub fn export_svg(state: &mut WorkflowState) -> Task<WorkflowMessage> {
    if state.active_app_id.is_none() {
        return Task::none();
    }

    state.close_floating_panels();
    let file_name = suggested_export_file_name(state.title(), "svg");
    let svg_data = desktop_workflow_canvas::export_svg(&state.document);

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
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持导出工作流图片",
                    vec![],
                ))
            }
        },
        |result| WorkflowMessage::ExportFinished(result),
    )
}

pub fn export_png(state: &mut WorkflowState) -> Task<WorkflowMessage> {
    if state.active_app_id.is_none() {
        return Task::none();
    }

    state.close_floating_panels();
    let file_name = suggested_export_file_name(state.title(), "png");
    let svg_data = desktop_workflow_canvas::export_svg(&state.document);

    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = rfd::AsyncFileDialog::new().set_file_name(&file_name).save_file().await;
                if let Some(file) = file {
                    let png = render_workflow_svg_to_png(&svg_data).ok_or_else(|| {
                        desktop_i18n::workflow::Feedback::new("导出 PNG 失败", vec![])
                    })?;
                    file.write(&png).await.map_err(|error| error.to_string())?;
                    let _ = open::that(file.path());
                }
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = (file_name, svg_data);
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持导出工作流图片",
                    vec![],
                ))
            }
        },
        |result| WorkflowMessage::ExportFinished(result),
    )
}

pub fn export_jpeg(state: &mut WorkflowState) -> Task<WorkflowMessage> {
    if state.active_app_id.is_none() {
        return Task::none();
    }

    state.close_floating_panels();
    let file_name = suggested_export_file_name(state.title(), "jpg");
    let svg_data = desktop_workflow_canvas::export_svg(&state.document);

    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = rfd::AsyncFileDialog::new().set_file_name(&file_name).save_file().await;
                if let Some(file) = file {
                    let png = render_workflow_svg_to_png(&svg_data).ok_or_else(|| {
                        desktop_i18n::workflow::Feedback::new("导出 JPEG 失败", vec![])
                    })?;
                    let image = image::load_from_memory(&png).map_err(|error| {
                        desktop_i18n::workflow::Feedback::new(
                            "解码 PNG 失败: %s",
                            vec![error.to_string()],
                        )
                    })?;
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
                        .map_err(|error| {
                            desktop_i18n::workflow::Feedback::new(
                                "编码 JPEG 失败: %s",
                                vec![error.to_string()],
                            )
                        })?;
                    file.write(&jpeg_data.into_inner()).await.map_err(|error| error.to_string())?;
                    let _ = open::that(file.path());
                }
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = (file_name, svg_data);
                Err(desktop_i18n::workflow::Feedback::new(
                    "Web 平台暂不支持导出工作流图片",
                    vec![],
                ))
            }
        },
        |result| WorkflowMessage::ExportFinished(result),
    )
}

pub fn export_finished(
    state: &mut WorkflowState,
    result: Result<(), desktop_i18n::workflow::Feedback>,
) {
    match result {
        Ok(()) => {
            state
                .set_status(desktop_i18n::format::Message::new("已导出工作流图片", vec![]));
        }
        Err(error) => state.set_error(error),
    }
}


#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
