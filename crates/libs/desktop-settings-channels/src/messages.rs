use crate::state::*;
use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub enum ChannelsMessage {
    Effect(Box<crate::update::Effect>),
    ExecutionChoicesRequested(String),
    ExecutionChoicesLoaded {
        tenant: String,
        instance: String,
        generation: u64,
        result: Result<crate::state::ChannelExecutionChoices, String>,
    },
    ExecutionAgentSelected {
        instance: String,
        agent: Option<String>,
    },
    ExecutionProviderSelected {
        instance: String,
        provider: Option<String>,
    },
    ExecutionModelSelected {
        instance: String,
        model: Option<config_agent_profile::AgentModelReference>,
    },
    ProjectChoicesRequested(String),
    ProjectChoicesLoaded {
        tenant: String,
        instance: String,
        generation: u64,
        result: Result<Vec<(String, String)>, String>,
    },
    DefaultProjectSelected {
        instance: String,
        project: Option<String>,
    },
    Refresh,
    TabSelected(ChannelsSettingsTab),
    CliToggled(bool),
    ProjectDirChanged(String),
    MessageTimeoutSecsChanged(u32),
    EnabledToggled(String, bool),
    InstanceAdded(String),
    InstanceRemoved(String, usize),
    TextChanged(String, String),
    TextEditorAction(String, text_editor::Action),
    BoolToggled(String, bool),
    NumberChanged(String, u32),
    ReceiveModeChanged(String, String),
    WeComDeliveryOpened(crate::state::WeComDeliveryMode),
    WeComDeliveryClosed,
    WeComDeliveryChanged(String, String),
    WeComDeliveryClearSecret,
    WeComDeliverySave,
    WeComDeliverySaved(Result<(), String>),
    QrRequested(String),
    QrFetched {
        channel: String,
        generation: u64,
        result: Result<gateway_channel_qr::ChannelQrFetchResponseDto, String>,
    },
    QrPollTick {
        channel: String,
        generation: u64,
    },
    QrPolled {
        channel: String,
        generation: u64,
        result: Result<gateway_channel_qr::ChannelQrStatusResponseDto, String>,
    },
    QrClosed,
    QrSaveConfirmed,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Channels(ChannelsMessage),
}
#[derive(Debug, Clone)]
pub enum Message {
    None,
    Settings(SettingsMessage),
    CopyCode(String),
    Append(String),
    OpenUrlExternal(String),
}

impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::Append(text),
            TextContextEffect::OpenUrl(url) => Self::OpenUrlExternal(url),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
