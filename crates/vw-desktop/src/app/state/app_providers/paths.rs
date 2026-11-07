use vw_gateway_client::{
    AgentAppsReadError,
    vw_api_types::app_providers::paths::{ProviderDirectoryStatus, ProviderPaths},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Form {
    pub paths: Option<ProviderPaths>,
    pub draft: String,
    pub pending: Option<u64>,
    pub saving: bool,
    pub local_open: bool,
    pub error: Option<&'static str>,
}

impl Form {
    pub fn finish(
        &mut self,
        generation: u64,
        result: Result<ProviderDirectoryStatus, AgentAppsReadError>,
    ) -> bool {
        if self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(status) => {
                self.paths = Some(status.paths);
                self.draft = status.directory_override.unwrap_or_default();
                self.error = None;
            }
            Err(error) => {
                self.error = Some(match error {
                    AgentAppsReadError::Http { status: 401 | 403, .. } => "执行主机拒绝了此操作。",
                    AgentAppsReadError::Http { status: 409, .. } => {
                        "目录正在被使用，或有未完成的供应商事务。"
                    }
                    AgentAppsReadError::Http { status: 400 | 422, .. } => {
                        "执行主机无法验证此目录。请选择授权用户目录下已有的私有文件夹。"
                    }
                    _ => "无法确认执行主机的配置目录。请重新读取。",
                });
            }
        }
        std::mem::take(&mut self.saving) && self.error.is_none()
    }
}
