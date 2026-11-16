pub use desktop_settings_channels::state::*;

impl crate::app::App {
    pub fn channel_qr_flow_snapshot(&self) -> crate::app::ChannelQrFlowSnapshot {
        self.channels_settings.qr_flow_snapshot()
    }
}
