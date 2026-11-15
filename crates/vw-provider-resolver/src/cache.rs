//! 进程内异步缓存，状态由 `provider-state-cache` 唯一持有。

pub use provider_state_cache::{get_or_init, invalidate};
