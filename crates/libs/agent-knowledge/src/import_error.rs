//! 保留文档导入错误的 HTTP 状态码转换。
impl From<crate::import::KnowledgeImportError> for gateway_error::ApiError {
    fn from(error: crate::import::KnowledgeImportError) -> Self {
        match error {
            crate::import::KnowledgeImportError::BadRequest(message) => Self::bad_request(message),
            crate::import::KnowledgeImportError::NotImplemented(message) => Self::not_implemented(message),
        }
    }
}
