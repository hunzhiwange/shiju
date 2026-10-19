//! 渠道配置聚合模块。
//!
//! 本模块汇总所有消息通道相关配置类型，包括 IM、协作平台、Webhook、邮件以及
//! 若干第三方服务。各具体通道类型按子模块拆分，避免顶层配置结构过于臃肿。

pub mod chat;
pub mod clawdtalk;
pub mod email;
pub mod lark;
pub mod other;
pub mod types;
pub mod wecom;

pub use chat::*;
pub use clawdtalk::*;
pub use email::*;
pub use lark::*;
pub use other::*;
pub use types::*;
pub use wecom::*;
