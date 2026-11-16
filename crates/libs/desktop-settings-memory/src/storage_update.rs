use crate::{state::StorageSettingsState, messages::StorageMessage};
pub fn config(s: &StorageSettingsState, storage: &mut config_memory::StorageConfig) -> Result<(), String> {
    let provider = s.provider.trim().to_string();
    let db_url = s.db_url_input.trim().to_string();
    let schema = {
        let value = s.schema.trim();
        if value.is_empty() { "public".to_string() } else { value.to_string() }
    };
    let table = {
        let value = s.table.trim();
        if value.is_empty() { "memories".to_string() } else { value.to_string() }
    };
    let connect_timeout_secs = if s.connect_timeout_secs_input.trim().is_empty() {
        None
    } else {
        Some(
            s.connect_timeout_secs_input
                .trim()
                .parse::<u64>()
                .map_err(|_| "connect_timeout_secs 必须是整数秒或留空".to_string())?,
        )
    };
    let tls = s.tls;

    storage.provider.config.provider = provider;
    storage.provider.config.db_url = if db_url.is_empty() { None } else { Some(db_url) };
    storage.provider.config.schema = schema;
    storage.provider.config.table = table;
    storage.provider.config.connect_timeout_secs = connect_timeout_secs;
    storage.provider.config.tls = tls;
    Ok(())
}

pub fn update(state: &mut StorageSettingsState, storage_message: StorageMessage) {
    match storage_message {
        StorageMessage::ProviderChanged(value) => {
            state.provider = value;
        }
        StorageMessage::DbUrlChanged(value) => {
            state.db_url_input = value;
        }
        StorageMessage::SchemaChanged(value) => {
            state.schema = value;
        }
        StorageMessage::TableChanged(value) => {
            state.table = value;
        }
        StorageMessage::ConnectTimeoutSecsChanged(value) => {
            state.connect_timeout_secs_input = value;
        }
        StorageMessage::TlsToggled(value) => {
            state.tls = value;
        }
        StorageMessage::Save => {}
    }

}

/// Apply an edit, validate the complete draft and update its save feedback.
/// The host owns the actual persistence port.
pub fn prepare_save(state: &mut StorageSettingsState, message: StorageMessage) -> Option<config_memory::StorageConfig> {
    update(state, message);
    let mut storage = config_memory::StorageConfig::default();
    match config(state, &mut storage) {
        Ok(()) => { state.save_error = None; Some(storage) }
        Err(error) => { state.save_error = Some(error); None }
    }
}
