//! 渠道服务认证、端点与默认配置。

pub mod clawdtalk;
pub mod email;
pub mod lark;
pub mod other;
pub mod wechat;
pub mod wecom;

pub use clawdtalk::*;
pub use email::*;
pub use lark::*;
pub use other::*;
pub use wechat::*;
pub use wecom::*;
