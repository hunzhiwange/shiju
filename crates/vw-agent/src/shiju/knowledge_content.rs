//! 按上游字符偏移读取正文；文件名仅用于寻址，身份始终由 ID 核验。
use super::*;
use vw_gateway_client::{ShijuKnowledgeReadDto, ShijuKnowledgeReadRequest};

const MAX_CONTENT_BYTES: usize = 1024 * 1024;
const CONTENT_TOO_LARGE: &str = "SHIJU knowledge response exceeds size limit";

impl ShijuAccountClient {
    pub(crate) async fn knowledge_read(
        &self,
        credential: &str,
        request: &ShijuKnowledgeReadRequest,
    ) -> Result<ShijuKnowledgeReadDto, ShijuAccountError> {
        validate_credential(credential)?;
        request.validate().map_err(|_| invalid_input("invalid SHIJU knowledge read"))?;
        let response = self
            .knowledge_http
            .get(endpoint(&self.gateway_base, &format!("kb/api/v1/documents/{}", request.doc_zid))?)
            .header("X-Api-Key", credential)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        let envelope: Envelope<ShijuKnowledgeDocumentDto> =
            serde_json::from_slice(&bounded_body(response).await?)
                .map_err(|_| invalid_response())?;
        if envelope.code != 0 {
            return Err(ShijuAccountError::Message(REQUEST_FAILED));
        }
        let document = envelope.data.ok_or_else(invalid_response)?;
        validate_response_id(&document.kb_zid)?;
        if document.doc_zid != request.doc_zid
            || request.kb_zid.as_ref().is_some_and(|id| *id != document.kb_zid)
        {
            return Err(invalid_response());
        }
        let title =
            if document.title.is_empty() { &document.original_filename } else { &document.title };
        if title.trim().is_empty() || matches!(title.as_str(), "." | "..") {
            return Err(invalid_response());
        }
        let mut url = endpoint(&self.gateway_base, "kb/api/v1/agent/documents/by-filename/")?;
        url.path_segments_mut()
            .map_err(|_| invalid_response())?
            .pop_if_empty()
            .push(title)
            .push("content.md");
        let response = self
            .knowledge_http
            .get(url)
            .header("X-Api-Key", credential)
            .query(&[("offset", request.offset), ("limit_chars", u64::from(request.limit_chars))])
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ShijuAccountError::Upstream { status, message: REQUEST_FAILED.into() });
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next());
        if !mime.is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/markdown")) {
            return Err(invalid_response());
        }
        let bytes = bounded_body(response).await?;
        let raw = std::str::from_utf8(&bytes).map_err(|_| invalid_response())?;
        parse_page(raw, &document, request)
    }
}

fn invalid_response() -> ShijuAccountError {
    ShijuAccountError::Message(INVALID_RESPONSE)
}

async fn bounded_body(mut response: reqwest::Response) -> Result<Vec<u8>, ShijuAccountError> {
    let status = response.status();
    if !status.is_success() {
        return Err(ShijuAccountError::Upstream { status, message: REQUEST_FAILED.into() });
    }
    if response.content_length().is_some_and(|length| length > MAX_CONTENT_BYTES as u64) {
        return Err(ShijuAccountError::Message(CONTENT_TOO_LARGE));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) =
        response.chunk().await.map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?
    {
        if chunk.len() > MAX_CONTENT_BYTES - bytes.len() {
            return Err(ShijuAccountError::Message(CONTENT_TOO_LARGE));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Deserialize)]
struct ContentMetadata {
    doc_zid: String,
    kb_zid: String,
    total_chars: u64,
    has_more: bool,
    next_offset: Option<u64>,
}

fn parse_page(
    raw: &str,
    document: &ShijuKnowledgeDocumentDto,
    request: &ShijuKnowledgeReadRequest,
) -> Result<ShijuKnowledgeReadDto, ShijuAccountError> {
    let normalized = raw.replace("\r\n", "\n");
    let (yaml, section) = normalized
        .strip_prefix("---\n")
        .and_then(|raw| raw.split_once("\n---\n"))
        .ok_or_else(invalid_response)?;
    let meta: ContentMetadata = serde_yaml::from_str(yaml).map_err(|_| invalid_response())?;
    if meta.doc_zid != request.doc_zid
        || meta.kb_zid != document.kb_zid
        || request.offset > meta.total_chars
    {
        return Err(invalid_response());
    }
    let next_offset = if meta.has_more {
        let next = meta.next_offset.ok_or_else(invalid_response)?;
        if next <= request.offset || next > meta.total_chars {
            return Err(invalid_response());
        }
        Some(next)
    } else {
        None
    };
    let content = extract_content(section, meta.has_more)?;
    Ok(ShijuKnowledgeReadDto {
        doc_zid: document.doc_zid.clone(),
        kb_zid: document.kb_zid.clone(),
        title: if document.title.is_empty() {
            document.original_filename.clone()
        } else {
            document.title.clone()
        },
        content: content.to_string(),
        total_chars: meta.total_chars,
        next_offset,
    })
}

fn extract_content(section: &str, has_more: bool) -> Result<&str, ShijuAccountError> {
    let mut body = None;
    let mut continuation = None;
    let mut instructions = None;
    let mut offset = 0;
    // 保留普通 Markdown 标题；保留字重复或顺序不明时拒绝，而非猜测截断位置。
    for line in section.split_inclusive('\n') {
        let heading = line.strip_suffix('\n').unwrap_or(line);
        let slot = match heading.trim() {
            "## 正文" => Some(&mut body),
            "## 续读" => Some(&mut continuation),
            "## 操作指引" => Some(&mut instructions),
            _ => None,
        };
        if let Some(slot) = slot {
            if heading != heading.trim()
                || slot.is_some()
                || !line.ends_with('\n')
                || !section[offset + line.len()..].starts_with('\n')
            {
                return Err(invalid_response());
            }
            *slot = Some((offset, offset + line.len() + 1));
        }
        offset += line.len();
    }
    let (_, start) = body.ok_or_else(invalid_response)?;
    let mut end = section.len();
    if let Some((position, _)) = instructions {
        if position < start {
            return Err(invalid_response());
        }
        end = position;
    }
    if let Some((position, _)) = continuation {
        if !has_more || position < start || position >= end {
            return Err(invalid_response());
        }
        end = position;
    }
    Ok(section[start..end].trim())
}
