//! 可靠性设置的边界约束、状态更新与持久化配置投影。
use crate::state::ReliabilitySettingsState;
use crate::messages::SettingsMessage;
use config_reliability::ReliabilityConfig;

fn persist_reliability_settings(s: &ReliabilitySettingsState) -> ReliabilityConfig {
    let provider_retries = s.provider_retries.clamp(0, 20);
    let provider_backoff_ms = s.provider_backoff_ms.clamp(0, 60_000);
    let channel_initial_backoff_secs = s.channel_initial_backoff_secs.clamp(1, 3600);
    let channel_max_backoff_secs =
        s.channel_max_backoff_secs.clamp(s.channel_initial_backoff_secs.max(1), 3600);
    let scheduler_poll_secs = s.scheduler_poll_secs.clamp(1, 3600);
    let scheduler_retries = s.scheduler_retries.clamp(0, 20);
    let mut reliability = ReliabilityConfig::default();
    reliability.provider_retries = provider_retries;
    // Provider 退避时间：限制在 0-60000 毫秒（1分钟）之间
    reliability.provider_backoff_ms = provider_backoff_ms;
    // Channel 初始退避时间：限制在 1-3600 秒（1小时）之间
    reliability.channel_initial_backoff_secs = channel_initial_backoff_secs;
    // Channel 最大退避时间：必须 >= 初始退避时间，且 <= 3600 秒
    // max(1) 确保最小值为 1，避免与初始值比较时出现 0 的情况
    reliability.channel_max_backoff_secs = channel_max_backoff_secs;
    // 调度器轮询间隔：限制在 1-3600 秒之间
    reliability.scheduler_poll_secs = scheduler_poll_secs;
    // 调度器重试次数：限制在 0-20 次之间
    reliability.scheduler_retries = scheduler_retries;
    reliability
}


/// 更新本域状态，返回需要宿主持久化的配置。
pub fn update(state: &mut ReliabilitySettingsState, message: SettingsMessage) -> Option<ReliabilityConfig> {
    match message {
        // Provider 重试次数变更
        SettingsMessage::ReliabilityProviderRetriesChanged(v) => {
            // 约束值在 0-20 范围内
            state.provider_retries = v.clamp(0, 20);
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // Provider 退避时间变更（毫秒）
        SettingsMessage::ReliabilityProviderBackoffMsChanged(v) => {
            // 约束值在 0-60000 毫秒范围内
            state.provider_backoff_ms = v.clamp(0, 60_000);
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // Channel 初始退避时间变更（秒）
        SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(v) => {
            // 约束值在 1-3600 秒范围内
            state.channel_initial_backoff_secs = v.clamp(1, 3600);
            // 如果最大退避时间小于新的初始退避时间，则自动调整最大值
            // 这确保了 max_backoff >= initial_backoff 的约束
            if state.channel_max_backoff_secs
                < state.channel_initial_backoff_secs
            {
                state.channel_max_backoff_secs =
                    state.channel_initial_backoff_secs;
            }
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // Channel 最大退避时间变更（秒）
        SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(v) => {
            // 最小值为当前初始退避时间（至少为 1）
            let min_v = state.channel_initial_backoff_secs.max(1);
            // 约束值在 [初始值, 3600] 范围内
            state.channel_max_backoff_secs = v.clamp(min_v, 3600);
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // 调度器轮询间隔变更（秒）
        SettingsMessage::ReliabilitySchedulerPollSecsChanged(v) => {
            // 约束值在 1-3600 秒范围内
            state.scheduler_poll_secs = v.clamp(1, 3600);
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // 调度器重试次数变更
        SettingsMessage::ReliabilitySchedulerRetriesChanged(v) => {
            // 约束值在 0-20 次范围内
            state.scheduler_retries = v.clamp(0, 20);
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // 手动保存可靠性设置
        SettingsMessage::ReliabilitySave => {
            state.save_error = None;
            Some(persist_reliability_settings(state))
        }
        // 打开帮助模态框
        SettingsMessage::ReliabilityHelpOpen => {
            state.show_help_modal = true;
            None
        }
        // 关闭帮助模态框
        SettingsMessage::ReliabilityHelpClose => {
            state.show_help_modal = false;
            None
        }
        // 未匹配的消息，不做任何处理
        _ => None,
    }
}
#[cfg(test)]
#[path = "reliability_tests.rs"]
mod reliability_tests;
