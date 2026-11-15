//! WhatsApp Cloud API 通道由 channel-whatsapp 提供。
#[cfg(not(test))]
pub use channel_whatsapp::WhatsAppChannel;

// 宿主测试需要 cfg(test) 专用的本地 HTTP 构造器；生产构建仅重导出。
#[cfg(test)]
#[path = "../../../../libs/channel-whatsapp/src/lib.rs"]
mod implementation;
#[cfg(test)]
pub use implementation::WhatsAppChannel;
