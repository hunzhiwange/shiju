//! 知识库存储后端与共享契约。
mod common;
pub use common::*;
pub mod postgres;
mod retrieval_helpers;
pub use retrieval_helpers::*;
