use crate::{DesktopSkillCatalogEntryDto, DesktopSkillDetailDto, DesktopSkillPathDto};
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// 读取 Skills 目录页可展示的技能列表。
    pub async fn skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        let mut query = Vec::new();
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.transport.get_json("/v1/skills", &query).await
    }

    /// 读取 desktop 专用技能目录。
    pub async fn desktop_skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        let mut query = Vec::new();
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.transport.get_json("/v1/desktop/skills", &query).await
    }

    /// 读取指定技能的文档与可执行操作。
    pub async fn skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        let mut query = vec![("skill_id".to_string(), skill_id.to_string())];
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.transport.get_json("/v1/skills/detail", &query).await
    }

    /// 读取 desktop 专用技能详情。
    pub async fn desktop_skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        let mut query = vec![("skill_id".to_string(), skill_id.to_string())];
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.transport.get_json("/v1/desktop/skills/detail", &query).await
    }

    /// 在当前项目下创建新的技能 scaffold。
    pub async fn skill_create(&self, project_path: &str) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/skills/create",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 创建 desktop 专用技能 scaffold。
    pub async fn desktop_skill_create(&self, project_path: &str) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/desktop/skills/create",
                &[],
                &serde_json::json!({"project_path": project_path}),
            )
            .await?;
        Ok(response.path)
    }

    /// 将内置技能安装到当前项目 skills 目录。
    pub async fn skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/skills/install-built-in",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 安装 desktop 专用内置技能。
    pub async fn desktop_skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/desktop/skills/install-built-in",
                &[],
                &serde_json::json!({"project_path": project_path, "skill_id": skill_id}),
            )
            .await?;
        Ok(response.path)
    }

    /// 设置本地技能启用状态。
    pub async fn skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/skills/set-enabled",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                    "enabled": enabled,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 设置 desktop 专用技能启用状态。
    pub async fn desktop_skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/desktop/skills/set-enabled",
                &[],
                &serde_json::json!({"project_path": project_path, "skill_id": skill_id, "enabled": enabled}),
            )
            .await?;
        Ok(response.path)
    }

    /// 删除本地技能目录。
    pub async fn skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/skills/delete",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 删除 desktop 专用技能目录。
    pub async fn desktop_skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self.transport.post_json(
                "/v1/desktop/skills/delete",
                &[],
                &serde_json::json!({"project_path": project_path, "skill_id": skill_id}),
            )
            .await?;
        Ok(response.path)
    }
}
