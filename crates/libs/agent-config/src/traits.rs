//! 通道配置及通用配置项的名称与描述契约。
//!
//! 通道元数据是类型级静态信息；通用配置句柄允许实例根据自身状态提供元数据。

/// 描述一种通道配置的静态展示信息。
pub trait ChannelConfig {
    /// 返回面向用户的名称。
    fn name() -> &'static str;
    /// 返回简短说明。
    fn desc() -> &'static str;
}

// Maybe there should be a `&self` as parameter for custom channel/info or what...

/// 描述一个具体配置句柄的展示信息。
pub trait ConfigHandle {
    /// 返回该实例的名称。
    fn name(&self) -> &'static str;
    /// 返回该实例的简短说明。
    fn desc(&self) -> &'static str;
}
