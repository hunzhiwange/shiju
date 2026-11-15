//! 技能上游客户端的宿主适配与路由组合。
use axum::Router;
use gateway_types::ApiError;
use gateway_shiju_account::ShijuCredential;
use gateway_shiju_skill::*;
use crate::skills::shiju::{ShijuSkillClient, ShijuSkillError};

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let client = ShijuSkillClient::production()
        .unwrap_or_else(|error| panic!("failed to initialize SHIJU skill client: {error}"));
    router_with_client(client)
}

pub(crate) fn router_with_client<S: Clone + Send + Sync + 'static>(client: ShijuSkillClient) -> Router<S> {
    gateway_shiju::shiju_skills::router_with_client(client)
}

impl gateway_shiju::shiju_skills::ShijuSkillService for ShijuSkillClient {
    async fn search(&self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery) -> Result<ShijuSkillCursorPageDto, ApiError> {
        ShijuSkillClient::search(self, credential, query).await.map_err(map_error)
    }
    async fn installed(&self,
        credential: &ShijuCredential) -> Result<Vec<ShijuInstalledSkillDto>, ApiError> {
        ShijuSkillClient::installed(self, credential).await.map_err(map_error)
    }
    async fn tags(&self,
        credential: &ShijuCredential) -> Result<ShijuSkillTagCatalogDto, ApiError> {
        ShijuSkillClient::tags(self, credential).await.map_err(map_error)
    }
    async fn tenant_tags(&self,
        credential: &ShijuCredential) -> Result<ShijuSkillTagCatalogDto, ApiError> {
        ShijuSkillClient::tenant_tags(self, credential).await.map_err(map_error)
    }
    async fn system(&self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, ApiError> {
        ShijuSkillClient::system(self, credential, query).await.map_err(map_error)
    }
    async fn maintained(&self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, ApiError> {
        ShijuSkillClient::maintained(self, credential, query).await.map_err(map_error)
    }
    async fn detail(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<ShijuSkillDetailDto, ApiError> {
        ShijuSkillClient::detail(self, credential, skill_id).await.map_err(map_error)
    }
    async fn readme(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<ShijuSkillReadmeDto, ApiError> {
        ShijuSkillClient::readme(self, credential, skill_id).await.map_err(map_error)
    }
    async fn versions(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<Vec<ShijuSkillVersionDto>, ApiError> {
        ShijuSkillClient::versions(self, credential, skill_id).await.map_err(map_error)
    }
    async fn remote_install(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_install(self, credential, skill_id).await.map_err(map_error)
    }
    async fn remote_install_version(&self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_install_version(self, credential, skill_id, version_id).await.map_err(map_error)
    }
    async fn remote_uninstall(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_uninstall(self, credential, skill_id).await.map_err(map_error)
    }
}

fn map_error(error: ShijuSkillError) -> ApiError {
    if error.is_invalid_input() {
        ApiError::bad_request(error.to_string())
    } else {
        ApiError::internal("SHIJU skill upstream request failed")
    }
}


#[cfg(test)]
#[path = "shiju_skills_tests.rs"]
mod shiju_skills_tests;
