//! OMP directory editor; native configuration remains owned by the gateway.
use iced::widget::text_editor;
#[cfg(not(target_arch = "wasm32"))]
use vw_gateway_client::AggregationApiError;
use vw_gateway_client::GatewayClient;
use vw_gateway_client::vw_api_types::agent_apps::aggregation::*;

pub(crate) async fn read(
    client: &GatewayClient,
    query: &AggregationQuery,
) -> Result<OmpDirectoryState, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        client.omp_directory_state(query).await.map_err(error)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, query);
        Err("当前客户端不支持 OMP 原生目录".into())
    }
}

#[derive(Clone, Default)]
pub(crate) struct Form {
    pub state: Option<OmpDirectoryState>,
    pub project: Option<vw_gateway_client::vw_api_types::id::ProjectId>,
    pub access: u64,
    pub removing: Option<String>,
    pub id: String,
    pub name: String,
    pub declaration: text_editor::Content,
    pub pending: Option<u64>,
    pub error: Option<String>,
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OmpDirectoryForm([redacted])")
    }
}
impl Form {
    pub fn finish(&mut self, generation: u64, result: Result<OmpDirectoryState, String>) -> bool {
        if self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(state) => {
                self.state = Some(state);
                self.removing = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        true
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn error(error: AggregationApiError) -> String {
    let (status, code) = match &error {
        AggregationApiError::Http { status, body } => {
            (*status, body.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_str()))
        }
        _ => return "OMP 目录请求失败，请检查连接".into(),
    };
    if matches!(code, Some("unsupported_app" | "unsupported_target")) {
        return "此执行主机不支持 OMP 目录".into();
    }
    match status {
        409 => "目录修订已变更或事务待恢复，请保留草稿并刷新后核对",
        401 | 403 => "无权访问此项目的 OMP 目录",
        404 => "OMP 条目不存在，请刷新目录后核对",
        400 | 422 => "OMP YAML 无法安全编辑或模型声明无效，草稿已保留",
        502..=504 => "OMP 执行主机失败或不可用，草稿已保留",
        _ => "OMP 目录请求失败，请检查连接",
    }
    .into()
}

impl Form {
    pub fn write(
        &self,
        query: &AggregationQuery,
        remove: bool,
    ) -> Result<(String, OmpDirectoryWrite), String> {
        let state = self.state.as_ref().filter(|s| !s.revision.is_empty()).ok_or("请先刷新目录")?;
        let id = if remove {
            self.removing.as_deref().ok_or("请先确认移除条目")?
        } else {
            self.id.trim()
        };
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err("条目 ID 仅支持 ASCII 字母、数字、下划线和连字符".into());
        }
        let change = if remove {
            OmpDirectoryChange::Remove {}
        } else {
            if self.name.trim().is_empty() {
                return Err("请填写名称".into());
            }
            let raw = self.declaration.text();
            if raw.len() > 1024 * 1024 {
                return Err("模型声明过大".into());
            }
            let declaration: serde_json::Value =
                serde_yaml::from_str(&raw).map_err(|_| "模型声明必须为有效 YAML 对象")?;
            if !declaration.is_object() {
                return Err("模型声明必须为 YAML 对象".into());
            }
            let profile = vw_gateway_client::vw_api_types::app_providers::omp::OmpProfile {
                name: self.name.trim().into(),
                declaration,
            };
            if state.members.iter().any(|member| member == id) {
                OmpDirectoryChange::Replace { profile }
            } else {
                OmpDirectoryChange::Insert { profile }
            }
        };
        Ok((
            id.into(),
            OmpDirectoryWrite {
                target_host_id: query.target_host_id.clone(),
                project_id: query.project_id.clone(),
                expected_revision: state.revision.clone(),
                change,
            },
        ))
    }
}
pub(crate) async fn execute(
    client: GatewayClient,
    query: AggregationQuery,
    write: Option<(String, OmpDirectoryWrite)>,
) -> Result<OmpDirectoryState, String> {
    match write {
        Some((id, body)) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                client.omp_directory_entry(&id, &body).await.map_err(error)
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = (client, query, id, body);
                Err("当前客户端不支持 OMP 原生目录".into())
            }
        }
        None => read(&client, &query).await,
    }
}
