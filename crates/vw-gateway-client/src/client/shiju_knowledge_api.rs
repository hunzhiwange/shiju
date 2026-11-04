//! 只读云端知识库契约；页码与正文字符偏移分别建模。
use super::{GatewayClient, ShijuCredential};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuKnowledgeBaseDto {
    pub kb_zid: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuKnowledgeDocumentDto {
    pub doc_zid: String,
    pub kb_zid: String,
    pub title: String,
    #[serde(default)]
    pub original_filename: String,
    pub lifecycle_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuKnowledgePageDto<T> {
    pub items: Vec<T>,
    pub page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuKnowledgePageQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size", rename = "pageSize")]
    pub page_size: u32,
    #[serde(default)]
    pub q: String,
}
fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    20
}
fn default_top_k() -> u32 {
    5
}
fn default_limit_chars() -> u32 {
    16000
}
impl Default for ShijuKnowledgePageQuery {
    fn default() -> Self {
        Self { page: default_page(), page_size: default_page_size(), q: String::new() }
    }
}
impl ShijuKnowledgePageQuery {
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 || !(1..=100).contains(&self.page_size) {
            return Err("invalid_knowledge_page".into());
        }
        Ok(())
    }
    fn query_pairs(&self) -> Result<Vec<(String, String)>, String> {
        self.validate()?;
        Ok(vec![
            ("page".into(), self.page.to_string()),
            ("pageSize".into(), self.page_size.to_string()),
            ("q".into(), self.q.clone()),
        ])
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuKnowledgeSearchRequest {
    pub query: String,
    #[serde(default)]
    pub kb_zids: Vec<String>,
    #[serde(default)]
    pub doc_zids: Vec<String>,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
}
impl ShijuKnowledgeSearchRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty() {
            return Err("empty_knowledge_query".into());
        }
        if !(1..=20).contains(&self.top_k) {
            return Err("invalid_knowledge_top_k".into());
        }
        for id in self.kb_zids.iter().chain(&self.doc_zids) {
            validate_identifier(id)?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShijuKnowledgeRecordDto {
    pub kb_zid: String,
    pub doc_zid: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    pub score: f64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ShijuKnowledgeSearchDto {
    pub records: Vec<ShijuKnowledgeRecordDto>,
    pub total: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuKnowledgeReadRequest {
    pub doc_zid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kb_zid: Option<String>,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_limit_chars")]
    pub limit_chars: u32,
}
impl ShijuKnowledgeReadRequest {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier(&self.doc_zid)?;
        if let Some(id) = &self.kb_zid {
            validate_identifier(id)?;
        }
        if !(1..=32000).contains(&self.limit_chars) {
            return Err("invalid_knowledge_limit_chars".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ShijuKnowledgeReadDto {
    pub doc_zid: String,
    pub kb_zid: String,
    pub title: String,
    pub content: String,
    pub total_chars: u64,
    pub next_offset: Option<u64>,
}
macro_rules! redacted_debug {
    ($($ty:ty),+ $(,)?) => {$ (
        impl std::fmt::Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($ty), "(<redacted>)"))
            }
        }
    )+};
}
redacted_debug!(
    ShijuKnowledgeSearchRequest,
    ShijuKnowledgeRecordDto,
    ShijuKnowledgeSearchDto,
    ShijuKnowledgeReadRequest,
    ShijuKnowledgeReadDto
);

impl GatewayClient {
    pub async fn shiju_knowledge_bases(
        &self,
        credential: &ShijuCredential,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>, String> {
        self.shiju_account_sensitive_request::<(), _>(
            reqwest::Method::GET,
            "/v1/shiju/knowledge/bases",
            &query.query_pairs()?,
            None,
            Some(credential),
        )
        .await
    }
    pub async fn shiju_knowledge_documents(
        &self,
        credential: &ShijuCredential,
        kb_zid: &str,
        query: &ShijuKnowledgePageQuery,
    ) -> Result<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>, String> {
        validate_identifier(kb_zid)?;
        let mut pairs = query.query_pairs()?;
        pairs.push(("kb_zid".into(), kb_zid.into()));
        self.shiju_account_sensitive_request::<(), _>(
            reqwest::Method::GET,
            "/v1/shiju/knowledge/documents",
            &pairs,
            None,
            Some(credential),
        )
        .await
    }
    pub async fn shiju_knowledge_search(
        &self,
        credential: &ShijuCredential,
        request: &ShijuKnowledgeSearchRequest,
    ) -> Result<ShijuKnowledgeSearchDto, String> {
        request.validate()?;
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            "/v1/shiju/knowledge/search",
            &[],
            Some(request),
            Some(credential),
        )
        .await
    }
    pub async fn shiju_knowledge_read(
        &self,
        credential: &ShijuCredential,
        request: &ShijuKnowledgeReadRequest,
    ) -> Result<ShijuKnowledgeReadDto, String> {
        request.validate()?;
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            "/v1/shiju/knowledge/read",
            &[],
            Some(request),
            Some(credential),
        )
        .await
    }
}

fn validate_identifier(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("invalid_knowledge_id".into());
    }
    Ok(())
}
