pub use desktop_settings_channels::state::*;

impl crate::app::App {
    pub fn channel_qr_flow_snapshot(&self) -> crate::app::ChannelQrFlowSnapshot {
        let flow = &self.channels_settings.qr_flow;
        let configured = match flow.channel.as_deref() {
            Some("dingtalk") => self.channels_settings.dingtalk.is_some(),
            Some("feishu") => self.channels_settings.feishu.is_some(),
            Some("qq") => self.channels_settings.qq.is_some(),
            Some("wecom") => self.channels_settings.wecom.is_some(),
            Some("wechat") => self.channels_settings.wechat.is_some(),
            _ => false,
        };
        crate::app::ChannelQrFlowSnapshot {
            channel: flow.channel.clone(),
            generation: flow.generation,
            poll_in_flight: flow.poll_in_flight,
            credentials_ready: flow.credentials_ready,
            configured,
        }
    }
}
