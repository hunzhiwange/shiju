/// 单次工具循环允许排队重看的图片数量上限。
pub const MAX_PENDING_IMAGE_RELOADS: usize = 4;

/// 待重看图片队列：去重、有数量上限，并绑定当前资产范围。
#[derive(Debug, Clone, Default)]
pub struct PendingImageReloadQueue {
    scope_key: Option<String>,
    ids: Vec<String>,
}

impl PendingImageReloadQueue {
    /// 绑定当前图片资产范围键。
    pub fn set_scope_key(&mut self, scope_key: impl Into<String>) {
        self.scope_key = Some(scope_key.into());
    }

    /// 当前范围键。
    pub fn scope_key(&self) -> Option<&str> {
        self.scope_key.as_deref()
    }

    /// 入队一个 image_id；重复忽略，超量返回错误。
    pub fn enqueue(&mut self, image_id: &str) -> Result<(), String> {
        let image_id = image_id.trim();
        if image_id.is_empty() {
            return Err("image_id must be non-empty".to_string());
        }
        if self.ids.iter().any(|existing| existing == image_id) {
            return Ok(());
        }
        if self.ids.len() >= MAX_PENDING_IMAGE_RELOADS {
            return Err(format!(
                "pending image reload queue is full (max {MAX_PENDING_IMAGE_RELOADS})"
            ));
        }
        self.ids.push(image_id.to_string());
        Ok(())
    }

    /// 查看当前排队 ID。
    pub fn peek(&self) -> &[String] {
        &self.ids
    }

    /// 取出全部排队 ID（不清空 scope）。
    pub fn take_all(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ids)
    }

    /// 清空排队 ID。
    pub fn clear(&mut self) {
        self.ids.clear();
    }
}
