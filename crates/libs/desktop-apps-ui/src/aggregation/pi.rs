use crate::ProviderScope as Scope;
use vw_api_transport::ProjectId;
use vw_api_app_aggregation::*;
use iced::widget::text_editor;
use gateway_aggregation_native::AggregationApiError;
use vw_gateway_client::vw_api_app_provider_terminal::pi::PiProfile;
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Id(String),
    Name(String),
    Declaration(text_editor::Action),
    Save,
    ReviewRemove(String),
    CancelRemove,
    ConfirmRemove,
    Completed {
        scope: Scope,
        project: ProjectId,
        access: u64,
        generation: u64,
        result: Result<PiDirectoryState, AggregationApiError>,
    },
}
#[derive(Clone, Default)]
pub struct Form {
    pub context: Option<(ProjectId, u64)>,
    pub snapshot: Option<PiDirectoryState>,
    pub id: String,
    pub name: String,
    pub declaration: text_editor::Content,
    pub removing: Option<String>,
    pub pending: Option<u64>,
    pub error: Option<&'static str>,
}
impl Form {
    pub fn write(
        &self,
        query: &AggregationQuery,
        remove: bool,
    ) -> Result<(String, PiDirectoryWrite), &'static str> {
        let state =
            self.snapshot.as_ref().filter(|s| !s.revision.is_empty()).ok_or("请先刷新目录")?;
        let id = if remove {
            self.removing.as_deref().ok_or("请先确认删除条目")?
        } else {
            self.id.trim()
        };
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err("条目 ID 仅支持 ASCII 字母、数字、下划线和连字符");
        }
        let change = if remove {
            if !state.members.iter().any(|member| member == id) {
                return Err("条目已变化，请刷新目录");
            }
            PiDirectoryChange::Remove {}
        } else {
            if self.name.trim().is_empty() {
                return Err("请填写名称");
            }
            let declaration = serde_json::from_str(&self.declaration.text())
                .map_err(|_| "模型声明必须为 JSON 对象")?;
            if !matches!(declaration, serde_json::Value::Object(_)) {
                return Err("模型声明必须为 JSON 对象");
            }
            let profile = PiProfile { name: self.name.trim().into(), declaration };
            if state.members.iter().any(|member| member == id) {
                PiDirectoryChange::Replace { profile }
            } else {
                PiDirectoryChange::Insert { profile }
            }
        };
        Ok((
            id.into(),
            PiDirectoryWrite {
                target_host_id: query.target_host_id.clone(),
                project_id: query.project_id.clone(),
                expected_revision: state.revision.clone(),
                change,
            },
        ))
    }
    pub fn finish(
        &mut self,
        generation: u64,
        result: Result<PiDirectoryState, AggregationApiError>,
    ) -> bool {
        if self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(state) => {
                self.snapshot = Some(state);
                self.removing = None;
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.removing = None;
                self.error = Some(feedback(&error));
            }
        }
        true
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiDirectoryEvent")
    }
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiDirectoryForm")
    }
}
fn feedback(error: &AggregationApiError) -> &'static str {
    match error {
        AggregationApiError::Http { status: 401 | 403, .. } => "无权读取或修改此项目的 Pi 目录",
        AggregationApiError::Http { status: 409, .. } => {
            "目录已变化或存在待恢复事务，请刷新后重新确认"
        }
        AggregationApiError::Http { status: 404, .. } => "条目不存在，请刷新目录",
        AggregationApiError::Http { status: 422, body }
            if matches!(
                body.pointer("/error/code").and_then(serde_json::Value::as_str),
                Some("unsupported_app" | "unsupported_target")
            ) =>
        {
            "当前执行主机不支持 Pi 模型目录"
        }
        AggregationApiError::Http { status: 400 | 422, .. } => {
            "目标不支持或模型声明无效，请检查后刷新"
        }
        AggregationApiError::Http { status: 502..=504, .. } => {
            "执行主机不可用或执行失败，请刷新确认实际状态"
        }
        _ => "Pi 目录请求失败，请刷新确认实际状态",
    }
}
