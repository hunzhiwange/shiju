//! 只读图片资产重看工具。
//!
//! 模型通过 `image_id` 请求在下一 LLM step 一次性重新查看压缩模型输入。
//! 工具结果仅返回简短确认或错误，绝不包含 Base64、路径、OCR 或摘要正文。

use super::context::{current_tool_use_context, enqueue_current_pending_image_reload};
use super::traits::{Tool, ToolResult};
use crate::app::agent::multimodal::assets::{ImageAssetId, ImageAssetScope, ImageAssetStore};
use async_trait::async_trait;
use serde_json::{Value, json};

/// 按当前范围登记待重看图片 ID。
pub struct ImageAssetReadTool {
    store: ImageAssetStore,
}

impl ImageAssetReadTool {
    /// 使用全局持久 data 目录中的资产存储。
    pub fn new() -> Self {
        Self { store: ImageAssetStore::from_global_paths() }
    }

    /// 测试可注入临时资产根。
    pub fn with_store(store: ImageAssetStore) -> Self {
        Self { store }
    }
}

impl Default for ImageAssetReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ImageAssetReadTool {
    fn name(&self) -> &str {
        "image_asset_read"
    }

    fn description(&self) -> &str {
        "Request a one-shot reload of a previously stored image by image_id for the next model step. \
Does not return image bytes."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "image_id": {
                    "type": "string",
                    "description": "Content-addressed SHA-256 image id visible in the current scope"
                }
            },
            "required": ["image_id"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let Some(image_id_raw) = args.get("image_id").and_then(Value::as_str).map(str::trim) else {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("image_id is required".to_string()),
            });
        };

        let image_id = match ImageAssetId::from_hex(image_id_raw) {
            Ok(id) => id,
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "image_id must be a full 64-character SHA-256 hex digest".to_string(),
                    ),
                });
            }
        };

        let Some(context) = current_tool_use_context() else {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("image_asset_read requires an active tool session context".to_string()),
            });
        };

        let Some(scope_key) = context.image_asset_scope_key() else {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("no image asset scope is bound for this turn".to_string()),
            });
        };

        let scope = match ImageAssetScope::new(scope_key) {
            Ok(scope) => scope,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                });
            }
        };

        if !self.store.is_published(&scope, &image_id) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("image_id is unknown or not visible in the current scope".to_string()),
            });
        }

        if let Err(error) = enqueue_current_pending_image_reload(&image_id) {
            return Ok(ToolResult { success: false, output: String::new(), error: Some(error) });
        }

        Ok(ToolResult {
            success: true,
            output: format!(
                "Queued image asset {} for one-shot reload on the next model step.",
                image_id.as_str()
            ),
            error: None,
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
