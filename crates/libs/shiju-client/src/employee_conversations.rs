//! 当前识矩账号的 Agent Cloud 数字员工直聊接口。

use std::{fmt, time::Duration};

pub use gateway_shiju_account::ShijuCredential;
use reqwest::{Method, Response, StatusCode, Url, header::HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const PRODUCTION_BASE: &str = "https://gateway.shiju.cloud/sj/agent/api/v1/";
const MAX_RESPONSE_BYTES: usize = 8 << 20;
const MAX_CONFLICT_BYTES: usize = 16 << 10;

#[cfg(all(feature = "employee-acp-test-support", debug_assertions))]
pub(crate) fn fixture_address() -> Result<Option<std::net::SocketAddr>, Error> {
    let Some(value) = std::env::var_os("SHIJU_EMPLOYEE_ACP_FIXTURE_ADDRESS") else {
        return Ok(None);
    };
    let address = value
        .to_str()
        .and_then(|value| value.parse::<std::net::SocketAddr>().ok())
        .filter(|address| address.ip().is_loopback() && address.port() != 0)
        .ok_or(Error::InvalidRequest)?;
    Ok(Some(address))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeKind {
    Private,
    Shared,
}

impl EmployeeKind {
    fn collection(self) -> &'static str {
        match self {
            Self::Private => "assistants",
            Self::Shared => "employees",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmployeeTarget {
    pub employee_id: String,
    pub kind: EmployeeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationKind {
    PrivateDirect,
    SharedDirect,
    TaskTeam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSender {
    User,
    Employee,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TurnStatus {
    Accepted,
    Delivered,
    Responding,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Conversation {
    pub conversation_id: String,
    /// 私有和共享会话均使用此字段标识员工。
    pub assistant_id: String,
    pub active_version_id: String,
    pub title: String,
    pub kind: ConversationKind,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConversationMessage {
    pub message_id: String,
    pub sender: MessageSender,
    pub body: String,
    pub reply_to_message_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConversationTurn {
    pub turn_id: String,
    pub user_message_id: String,
    pub employee_version_id: String,
    pub status: TurnStatus,
    #[serde(default)]
    pub partial_reply: String,
    pub error_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
    pub turns: Vec<ConversationTurn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConversationSubmission {
    pub conversation: Conversation,
    pub message: ConversationMessage,
    pub turn: ConversationTurn,
    pub idempotent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmployeeConversationError {
    InvalidCredential,
    InvalidRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Busy,
    IdempotencyConflict,
    Conflict,
    Unavailable,
    InvalidResponse,
}

impl fmt::Display for EmployeeConversationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCredential => "识矩登录凭据格式无效",
            Self::InvalidRequest => "数字员工会话请求无效",
            Self::Unauthorized => "识矩登录已失效，请重新登录",
            Self::Forbidden => "当前识矩账号没有访问权限",
            Self::NotFound => "数字员工会话或员工不可见",
            Self::Busy => "数字员工正在回复，请等待当前轮次结束",
            Self::IdempotencyConflict => "数字员工消息提交标识已用于其他请求",
            Self::Conflict => "数字员工当前状态不允许此会话操作",
            Self::Unavailable => "数字员工会话服务暂不可用，请重试",
            Self::InvalidResponse => "数字员工会话服务返回的数据无法读取",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EmployeeConversationError {}

type Error = EmployeeConversationError;

#[derive(Clone)]
pub struct EmployeeConversationClient {
    http: reqwest::Client,
    base: Url,
    api_key: HeaderValue,
}

impl EmployeeConversationClient {
    /// 以当前登录账号访问 Agent Cloud。调试测试构建可显式使用回环替身。
    pub fn production(credential: &ShijuCredential) -> Result<Self, Error> {
        #[cfg(all(feature = "employee-acp-test-support", debug_assertions))]
        if let Some(address) = fixture_address()? {
            return Self::with_loopback(address, credential);
        }
        let base = Url::parse(PRODUCTION_BASE).expect("fixed Agent Cloud gateway URL");
        Self::new(base, credential)
    }

    #[cfg(any(
        test,
        feature = "test-support",
        all(feature = "employee-acp-test-support", debug_assertions)
    ))]
    /// 使用本机回环 HTTP 替身验证产品接口。
    pub fn with_loopback(
        address: std::net::SocketAddr,
        credential: &ShijuCredential,
    ) -> Result<Self, Error> {
        if !address.ip().is_loopback() {
            return Err(Error::InvalidRequest);
        }
        let base = Url::parse(&format!("http://{address}/sj/agent/api/v1/"))
            .map_err(|_| Error::InvalidRequest)?;
        Self::new(base, credential)
    }

    fn new(base: Url, credential: &ShijuCredential) -> Result<Self, Error> {
        let mut api_key =
            HeaderValue::from_str(credential.expose()).map_err(|_| Error::InvalidCredential)?;
        api_key.set_sensitive(true);
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .https_only(base.scheme() == "https");
        if base.scheme() == "http" {
            builder = builder.no_proxy();
        }
        let http = builder.build().map_err(|_| Error::Unavailable)?;
        Ok(Self { http, base, api_key })
    }

    pub async fn list_conversations(
        &self,
        target: &EmployeeTarget,
    ) -> Result<Vec<Conversation>, Error> {
        #[derive(Deserialize)]
        struct List {
            items: Vec<Conversation>,
        }
        let url = self.url(&[target.kind.collection(), &target.employee_id, "conversations"])?;
        let result: List = Self::decode(self.request(Method::GET, url), StatusCode::OK).await?;
        Ok(result.items)
    }

    pub async fn create_conversation(
        &self,
        target: &EmployeeTarget,
        message: &str,
        title: Option<&str>,
        idempotency_key: &str,
    ) -> Result<ConversationSubmission, Error> {
        let url = self.url(&[target.kind.collection(), &target.employee_id, "conversations"])?;
        self.submit(url, message, title, idempotency_key).await
    }

    pub async fn history(&self, conversation_id: &str) -> Result<ConversationHistory, Error> {
        let url = self.url(&["conversations", conversation_id, "messages"])?;
        Self::decode(self.request(Method::GET, url), StatusCode::OK).await
    }

    pub async fn send_message(
        &self,
        conversation_id: &str,
        message: &str,
        idempotency_key: &str,
    ) -> Result<ConversationSubmission, Error> {
        let url = self.url(&["conversations", conversation_id, "messages"])?;
        self.submit(url, message, None, idempotency_key).await
    }

    pub async fn cancel_turn(
        &self,
        conversation_id: &str,
        turn_id: &str,
    ) -> Result<ConversationTurn, Error> {
        let url = self.url(&["conversations", conversation_id, "turns", turn_id, "cancel"])?;
        Self::decode(self.request(Method::POST, url), StatusCode::OK).await
    }

    fn url(&self, segments: &[&str]) -> Result<Url, Error> {
        // URL path segment construction drops dot segments.
        if segments.iter().any(|segment| matches!(*segment, "" | "." | "..")) {
            return Err(Error::InvalidRequest);
        }
        let mut url = self.base.clone();
        url.path_segments_mut()
            .expect("hierarchical Agent Cloud URL")
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .header("X-Api-Key", self.api_key.clone())
            .header("Accept", "application/json")
    }

    async fn submit(
        &self,
        url: Url,
        message: &str,
        title: Option<&str>,
        idempotency_key: &str,
    ) -> Result<ConversationSubmission, Error> {
        #[derive(Serialize)]
        struct Submission<'a> {
            message: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<&'a str>,
        }
        if idempotency_key.trim().is_empty() {
            return Err(Error::InvalidRequest);
        }
        let key = HeaderValue::from_str(idempotency_key).map_err(|_| Error::InvalidRequest)?;
        let request = self
            .request(Method::POST, url)
            .header("Idempotency-Key", key)
            .json(&Submission { message, title });
        Self::decode(request, StatusCode::CREATED).await
    }

    async fn decode<T: DeserializeOwned>(
        request: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<T, Error> {
        let mut response = request.send().await.map_err(|_| Error::Unavailable)?;
        match response.status() {
            status if status == expected => {
                let body = Self::read_body(&mut response, MAX_RESPONSE_BYTES).await?;
                serde_json::from_slice(&body).map_err(|_| Error::InvalidResponse)
            }
            StatusCode::BAD_REQUEST => Err(Error::InvalidRequest),
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            StatusCode::FORBIDDEN => Err(Error::Forbidden),
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            StatusCode::CONFLICT => Err(Self::read_conflict(&mut response).await),
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS => Err(Error::Unavailable),
            status if status.is_server_error() => Err(Error::Unavailable),
            _ => Err(Error::InvalidResponse),
        }
    }

    async fn read_conflict(response: &mut Response) -> Error {
        #[derive(Deserialize)]
        struct Conflict {
            code: String,
        }
        let Ok(body) = Self::read_body(response, MAX_CONFLICT_BYTES).await else {
            return Error::Conflict;
        };
        match serde_json::from_slice::<Conflict>(&body)
            .ok()
            .as_ref()
            .map(|error| error.code.as_str())
        {
            Some("CONVERSATION_BUSY") => Error::Busy,
            Some("IDEMPOTENCY_CONFLICT") => Error::IdempotencyConflict,
            _ => Error::Conflict,
        }
    }

    async fn read_body(response: &mut Response, limit: usize) -> Result<Vec<u8>, Error> {
        if response.content_length().is_some_and(|length| length > limit as u64) {
            return Err(Error::InvalidResponse);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if chunk.len() > limit - body.len() {
                return Err(Error::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

#[cfg(test)]
#[path = "employee_conversations_tests.rs"]
mod tests;
