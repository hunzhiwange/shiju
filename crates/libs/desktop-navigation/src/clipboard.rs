//! Copy feedback state; clipboard access remains an explicit host effect.
use std::hash::{Hash, Hasher};
pub struct Feedback {
    /// 最近复制的代码哈希
    pub last_copied_code_hash: Option<u64>,
    /// 最近复制时间
    pub last_copy_time: Option<web_time::SystemTime>,
}
impl Feedback {
    pub fn new() -> Self {
        Self { last_copied_code_hash: None, last_copy_time: None }
    }
    pub fn copied(&mut self, text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        self.last_copied_code_hash = Some(hash);
        self.last_copy_time = Some(web_time::SystemTime::now());
        hash
    }
    pub fn clear(&mut self, hash: u64) {
        if self.last_copied_code_hash == Some(hash) {
            self.last_copied_code_hash = None;
            self.last_copy_time = None;
        }
    }
}
