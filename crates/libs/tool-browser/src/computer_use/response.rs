use serde::Deserialize;
use serde_json::Value;

/// 计算机使用边车服务的响应格式
///
/// 该结构体表示从计算机使用服务返回的标准响应格式，
/// 包含执行结果、数据和错误信息。
///
/// # 字段说明
///
/// - `success`: 操作是否成功（可选，默认为 true）
/// - `data`: 返回的数据内容，通常为 JSON 格式
/// - `error`: 错误信息，仅在操作失败时存在
///
/// # 反序列化
///
/// 所有字段都使用 `#[serde(default)]`，允许服务返回不完整的响应结构。
#[derive(Debug, Deserialize)]
pub(crate) struct ComputerUseResponse {
    /// 操作是否成功
    /// None 时默认认为成功
    #[serde(default)]
    pub(crate) success: Option<bool>,

    /// 返回的数据内容
    /// 包含操作结果的具体信息，如截图路径、元素信息等
    #[serde(default)]
    pub(crate) data: Option<Value>,

    /// 错误信息
    /// 当 success 为 false 或操作失败时包含错误描述
    #[serde(default)]
    pub(crate) error: Option<String>,
}
#[cfg(test)]
#[path = "response_tests.rs"]
mod response_tests;
