//! Explicit preview/confirmation. Call invalidate whenever scope, selection or draft changes.
use iced::Task;
use vw_gateway_client::vw_api_types::{
    agent_apps::DataManagementTarget,
    device_sync_config::{
        DeviceSyncLocalPreview, DeviceSyncRemoteCompatibility, DeviceSyncRemoteResponse,
        DeviceSyncTransport,
    },
    device_sync_task::{SyncDirection, SyncExpectedRemote, SyncTaskStatus},
};
use vw_gateway_client::{
    DeviceSyncDownloadRequest, DeviceSyncOperationsError, DeviceSyncSelection,
    DeviceSyncUploadRequest, GatewayClient,
};

#[derive(Clone, Debug)]
pub(crate) enum PreviewMessage {
    Inspect(SyncDirection),
    Confirm,
    Close,
    Inspected(
        u64,
        Result<(DeviceSyncRemoteResponse, DeviceSyncLocalPreview), DeviceSyncOperationsError>,
    ),
    Started(u64, Result<SyncTaskStatus, DeviceSyncOperationsError>),
}
#[derive(Default, Clone, Debug)]
pub(crate) struct PreviewState {
    pub busy: bool,
    pub remote: Option<DeviceSyncRemoteResponse>,
    pub direction: Option<SyncDirection>,
    local: Option<DeviceSyncLocalPreview>,
    identity: Option<String>,
    epoch: u64,
}
impl PreviewState {
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.busy = false;
        self.remote = None;
        self.local = None;
        self.direction = None;
        self.identity = None;
    }
    /// Returned status must be forwarded to the durable task panel; errors to toast.
    pub fn update(
        &mut self,
        event: PreviewMessage,
        client: GatewayClient,
        target: DataManagementTarget,
        token: String,
        transport: DeviceSyncTransport,
        selection: DeviceSyncSelection,
        dirty: bool,
    ) -> (Task<PreviewMessage>, Option<String>, Option<SyncTaskStatus>) {
        let identity = serde_json::to_string(&(&target, transport, &selection)).ok();
        if dirty || identity.is_none() {
            self.invalidate();
            return (Task::none(), Some("请先保存连接配置再预览".into()), None);
        }
        if self.identity.is_some() && self.identity != identity {
            self.invalidate();
        }
        match event {
            PreviewMessage::Close => self.invalidate(),
            PreviewMessage::Inspect(direction) if !self.busy => {
                self.invalidate();
                self.busy = true;
                self.identity = identity;
                self.direction = Some(direction);
                let epoch = self.epoch;
                return (
                    Task::perform(
                        async move {
                            let result = async {
                                let remote = client
                                    .device_sync_remote(
                                        transport,
                                        target.scope.project_id(),
                                        &target.target_host_id,
                                        &token,
                                    )
                                    .await?;
                                let local = client
                                    .device_sync_local_preview(
                                        transport, &target, &selection, &token,
                                    )
                                    .await?;
                                Ok((remote, local))
                            }
                            .await;
                            PreviewMessage::Inspected(epoch, result)
                        },
                        |e| e,
                    ),
                    None,
                    None,
                );
            }
            PreviewMessage::Inspected(epoch, result) if epoch == self.epoch && self.busy => {
                self.busy = false;
                match result {
                    Ok((remote, local)) => {
                        self.remote = Some(remote);
                        self.local = Some(local);
                    }
                    Err(_) => {
                        self.invalidate();
                        return (Task::none(), Some("预览失败，请重新获取远端状态".into()), None);
                    }
                }
            }
            PreviewMessage::Confirm if !self.busy && self.can_confirm() => {
                let remote = self.remote.take().expect("checked preview");
                let local = self.local.take().expect("checked local preview");
                let direction = self.direction.take().expect("checked direction");
                self.busy = true;
                let epoch = self.epoch;
                let request_id = format!(
                    "desktop-sync-{}-{epoch}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                );
                return (
                    Task::perform(
                        async move {
                            // Reinspect immediately; gateway also checks manifest CAS and local/config revisions.
                            let result = async {
                                if client
                                    .device_sync_remote(
                                        transport,
                                        target.scope.project_id(),
                                        &target.target_host_id,
                                        &token,
                                    )
                                    .await?
                                    != remote
                                {
                                    return Err(DeviceSyncOperationsError::Http {
                                        status: 409,
                                        body: serde_json::Value::Null,
                                    });
                                }
                                match (direction, remote) {
                                    (SyncDirection::Upload, remote) => {
                                        let expected_remote_snapshot = match remote {
                                            DeviceSyncRemoteResponse::Empty => {
                                                SyncExpectedRemote::Empty
                                            }
                                            DeviceSyncRemoteResponse::Snapshot {
                                                snapshot, ..
                                            } => SyncExpectedRemote::Snapshot { snapshot },
                                        };
                                        client
                                            .device_sync_upload(
                                                transport,
                                                &DeviceSyncUploadRequest {
                                                    target,
                                                    request_id,
                                                    expected_remote_snapshot,
                                                    selection,
                                                },
                                                &token,
                                            )
                                            .await
                                    }
                                    (
                                        SyncDirection::Download,
                                        DeviceSyncRemoteResponse::Snapshot { snapshot, .. },
                                    ) => {
                                        client
                                            .device_sync_download(
                                                transport,
                                                &DeviceSyncDownloadRequest {
                                                    target,
                                                    request_id,
                                                    expected_remote_snapshot: snapshot,
                                                    expected_local_snapshot: local.snapshot_id,
                                                    connection_revision: local.connection_revision,
                                                    selection,
                                                },
                                                &token,
                                            )
                                            .await
                                    }
                                    _ => Err(DeviceSyncOperationsError::InvalidInput),
                                }
                            }
                            .await;
                            PreviewMessage::Started(epoch, result)
                        },
                        |e| e,
                    ),
                    None,
                    None,
                );
            }
            PreviewMessage::Started(epoch, result) if epoch == self.epoch && self.busy => {
                self.invalidate();
                return match result {
                    Ok(status) => (Task::none(), None, Some(status)),
                    Err(DeviceSyncOperationsError::Http { status: 409, .. }) => {
                        (Task::none(), Some("状态已变化，请重新预览并确认".into()), None)
                    }
                    Err(_) => (
                        Task::none(),
                        Some("启动结果未确认，请刷新任务状态；不会自动重试".into()),
                        None,
                    ),
                };
            }
            _ => {}
        }
        (Task::none(), None, None)
    }
    pub fn can_confirm(&self) -> bool {
        self.local.is_some()
            && match (&self.direction, &self.remote) {
                (Some(SyncDirection::Upload), Some(DeviceSyncRemoteResponse::Empty)) => true,
                (
                    Some(_),
                    Some(DeviceSyncRemoteResponse::Snapshot {
                        compatibility: DeviceSyncRemoteCompatibility::Compatible,
                        ..
                    }),
                ) => true,
                _ => false,
            }
    }
    pub fn view(&self) -> iced::Element<'_, PreviewMessage> {
        use crate::app::components::system_settings_common::{
            primary_action_btn_style, rounded_action_btn_style,
        };
        use iced::widget::{button, column, row, text};
        let description = match &self.remote {
            Some(DeviceSyncRemoteResponse::Empty) => "远端为空；仅可上传".to_owned(),
            Some(DeviceSyncRemoteResponse::Snapshot {
                device_name,
                created_at,
                protocol_version,
                data_compat_version,
                manifest_path,
                compatibility,
                ..
            }) => format!(
                "设备：{device_name}\n时间：{created_at}\n协议：{protocol_version} / 数据：{data_compat_version}\n路径：{manifest_path}\n兼容性：{compatibility:?}"
            ),
            None => "操作前请获取远端预览；下载将替换所选本地数据".to_owned(),
        };
        let mut upload = button("预览上传").padding([8, 14]).style(rounded_action_btn_style);
        let mut download = button("预览下载").padding([8, 14]).style(rounded_action_btn_style);
        let mut confirm = button("确认执行").padding([8, 14]).style(primary_action_btn_style);
        if !self.busy {
            upload = upload.on_press(PreviewMessage::Inspect(SyncDirection::Upload));
            download = download.on_press(PreviewMessage::Inspect(SyncDirection::Download));
        }
        if !self.busy && self.can_confirm() {
            confirm = confirm.on_press(PreviewMessage::Confirm);
        }
        column![
            text(description),
            row![
                upload,
                download,
                confirm,
                button("取消")
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(PreviewMessage::Close)
            ]
            .spacing(8)
        ]
        .spacing(12)
        .into()
    }
}
