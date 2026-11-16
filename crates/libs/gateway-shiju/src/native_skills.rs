//! Native private-skill client adapter and error boundary.
use agent_skills::shiju::{ShijuSkillClient, ShijuSkillError};
use gateway_types::ApiError;
use gateway_shiju_account::ShijuCredential;
use gateway_shiju_skill::*;

struct NativeSkillClient(ShijuSkillClient);
pub fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    let client = ShijuSkillClient::production()
        .unwrap_or_else(|error| panic!("failed to initialize SHIJU skill client: {error}"));
    router_with_client(client)
}
pub fn router_with_client<S: Clone + Send + Sync + 'static>(client: ShijuSkillClient) -> axum::Router<S> {
    super::shiju_skills::router_with_client(NativeSkillClient(client))
}
impl super::shiju_skills::ShijuSkillService for NativeSkillClient {
    async fn search(&self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery) -> Result<ShijuSkillCursorPageDto, ApiError> {
        ShijuSkillClient::search(&self.0, credential, query).await.map_err(map_error)
    }
    async fn installed(&self,
        credential: &ShijuCredential) -> Result<Vec<ShijuInstalledSkillDto>, ApiError> {
        ShijuSkillClient::installed(&self.0, credential).await.map_err(map_error)
    }
    async fn tags(&self,
        credential: &ShijuCredential) -> Result<ShijuSkillTagCatalogDto, ApiError> {
        ShijuSkillClient::tags(&self.0, credential).await.map_err(map_error)
    }
    async fn tenant_tags(&self,
        credential: &ShijuCredential) -> Result<ShijuSkillTagCatalogDto, ApiError> {
        ShijuSkillClient::tenant_tags(&self.0, credential).await.map_err(map_error)
    }
    async fn system(&self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, ApiError> {
        ShijuSkillClient::system(&self.0, credential, query).await.map_err(map_error)
    }
    async fn maintained(&self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, ApiError> {
        ShijuSkillClient::maintained(&self.0, credential, query).await.map_err(map_error)
    }
    async fn detail(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<ShijuSkillDetailDto, ApiError> {
        ShijuSkillClient::detail(&self.0, credential, skill_id).await.map_err(map_error)
    }
    async fn readme(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<ShijuSkillReadmeDto, ApiError> {
        ShijuSkillClient::readme(&self.0, credential, skill_id).await.map_err(map_error)
    }
    async fn versions(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<Vec<ShijuSkillVersionDto>, ApiError> {
        ShijuSkillClient::versions(&self.0, credential, skill_id).await.map_err(map_error)
    }
    async fn remote_install(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_install(&self.0, credential, skill_id).await.map_err(map_error)
    }
    async fn remote_install_version(&self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_install_version(&self.0, credential, skill_id, version_id).await.map_err(map_error)
    }
    async fn remote_uninstall(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> Result<(), ApiError> {
        ShijuSkillClient::remote_uninstall(&self.0, credential, skill_id).await.map_err(map_error)
    }
}

fn map_error(error: ShijuSkillError) -> ApiError {
    if error.is_invalid_input() {
        ApiError::bad_request(error.to_string())
    } else {
        ApiError::internal("SHIJU skill upstream request failed")
    }
}
