//! 识矩云端知识库的只读分页与检索适配。
use super::{ShijuAccountClient, ShijuAccountError, endpoint};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use vw_gateway_client::{
    ShijuKnowledgeBaseDto, ShijuKnowledgeDocumentDto, ShijuKnowledgePageDto,
    ShijuKnowledgePageQuery, ShijuKnowledgeSearchDto, ShijuKnowledgeSearchRequest,
};

const REQUEST_FAILED: &str = "SHIJU knowledge request failed";
const INVALID_RESPONSE: &str = "invalid SHIJU knowledge response";

// 独立传输保持其他识矩接口行为不变，禁止携带凭据跟随上游跳转。
pub(super) fn http_client() -> Result<reqwest::Client, ShijuAccountError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| ShijuAccountError::Message("failed to create SHIJU knowledge client"))
}

impl ShijuAccountClient {
    pub(crate) async fn knowledge_bases(
        &self,
        credential: &str,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>, ShijuAccountError> {
        validate_credential(credential)?;
        query.validate().map_err(|_| invalid_input("invalid SHIJU knowledge page"))?;
        let response = self
            .knowledge_http
            .get(endpoint(&self.gateway_base, "kb/api/v1/knowledge-bases")?)
            .header("X-Api-Key", credential)
            .query(query)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        let page: ShijuKnowledgePageDto<ShijuKnowledgeBaseDto> = decode(response).await?;
        validate_page(&page, query)?;
        for base in &page.items {
            validate_response_id(&base.kb_zid)?;
        }
        Ok(page)
    }

    pub(crate) async fn knowledge_documents(
        &self,
        credential: &str,
        kb_zid: &str,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>, ShijuAccountError> {
        validate_credential(credential)?;
        validate_id(kb_zid)?;
        query.validate().map_err(|_| invalid_input("invalid SHIJU knowledge page"))?;
        let response = self
            .knowledge_http
            .get(endpoint(&self.gateway_base, "kb/api/v1/documents")?)
            .header("X-Api-Key", credential)
            .query(query)
            .query(&[("kb_zid", kb_zid)])
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        let page: ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto> = decode(response).await?;
        validate_page(&page, query)?;
        for document in &page.items {
            validate_response_id(&document.doc_zid)?;
            validate_response_id(&document.kb_zid)?;
            if document.kb_zid != kb_zid {
                return Err(ShijuAccountError::Message(INVALID_RESPONSE));
            }
        }
        Ok(page)
    }

    pub(crate) async fn knowledge_search(
        &self,
        credential: &str,
        request: &ShijuKnowledgeSearchRequest,
    ) -> Result<ShijuKnowledgeSearchDto, ShijuAccountError> {
        validate_credential(credential)?;
        request.validate().map_err(|_| invalid_input("invalid SHIJU knowledge search"))?;
        let response = self
            .knowledge_http
            .post(endpoint(&self.gateway_base, "kb/api/v1/retrieval/query")?)
            .header("X-Api-Key", credential)
            .json(request)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        let result: ShijuKnowledgeSearchDto = decode(response).await?;
        for record in &result.records {
            validate_response_id(&record.kb_zid)?;
            validate_response_id(&record.doc_zid)?;
            if !record.score.is_finite()
                || (!request.kb_zids.is_empty() && !request.kb_zids.contains(&record.kb_zid))
                || (!request.doc_zids.is_empty() && !request.doc_zids.contains(&record.doc_zid))
            {
                return Err(ShijuAccountError::Message(INVALID_RESPONSE));
            }
        }
        Ok(result)
    }
}

fn validate_credential(credential: &str) -> Result<(), ShijuAccountError> {
    if credential.trim().is_empty() {
        return Err(invalid_input("SHIJU credential cannot be empty"));
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), ShijuAccountError> {
    if id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid_input("invalid SHIJU knowledge ID"));
    }
    Ok(())
}

fn validate_response_id(id: &str) -> Result<(), ShijuAccountError> {
    validate_id(id).map_err(|_| ShijuAccountError::Message(INVALID_RESPONSE))
}

fn validate_page<T>(
    page: &ShijuKnowledgePageDto<T>,
    query: &ShijuKnowledgePageQuery,
) -> Result<(), ShijuAccountError> {
    // total 为 u64，反序列化已拒绝负数；请求边界已在发送前验证。
    if page.page != query.page || page.page_size != query.page_size {
        return Err(ShijuAccountError::Message(INVALID_RESPONSE));
    }
    Ok(())
}

fn invalid_input(message: &'static str) -> ShijuAccountError {
    ShijuAccountError::Upstream {
        status: reqwest::StatusCode::BAD_REQUEST,
        message: message.into(),
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    code: i64,
    data: Option<T>,
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, ShijuAccountError> {
    let status = response.status();
    if !status.is_success() {
        // 不读取或公开上游错误正文，保留 401/403 等 HTTP 状态供网关映射。
        return Err(ShijuAccountError::Upstream { status, message: REQUEST_FAILED.into() });
    }
    let envelope = response
        .json::<Envelope<T>>()
        .await
        .map_err(|_| ShijuAccountError::Message(INVALID_RESPONSE))?;
    if envelope.code != 0 {
        return Err(ShijuAccountError::Message(REQUEST_FAILED));
    }
    envelope.data.ok_or(ShijuAccountError::Message(INVALID_RESPONSE))
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod shiju_knowledge_tests;

#[path = "knowledge_content.rs"]
mod knowledge_content;
#[cfg(test)]
#[path = "knowledge_content_tests.rs"]
mod shiju_knowledge_content_tests;
