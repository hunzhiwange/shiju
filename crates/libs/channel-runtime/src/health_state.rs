/// 通道健康状态枚举。
///
/// 表示通道连接的健康检查结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelHealthState {
    /// 健康：通道正常工作
    Healthy,
    /// 不健康：通道存在问题
    Unhealthy,
    /// 超时：健康检查超时未响应
    Timeout,
}
