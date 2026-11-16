use agent_provider::provider;

pub fn assistant_error(error: provider::CatalogError) -> message_types::AssistantError {
    use agent_config::runtime as config;
    use storage_types as storage;
    let io = match &error {
        provider::CatalogError::Configuration(
            config::Error::Io(io) | config::Error::Storage(storage::Error::Io(io)),
        ) => Some(io),
        _ => None,
    };
    let is_retryable = io.is_some_and(|io| {
        matches!(
            io.kind(),
            std::io::ErrorKind::Interrupted
                | std::io::ErrorKind::WouldBlock
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::NotConnected
                | std::io::ErrorKind::UnexpectedEof
        )
    });
    message_types::AssistantError::APIError {
        message: error.to_string(),
        status_code: None,
        is_retryable,
        response_headers: None,
        response_body: None,
        metadata: None,
    }
}
pub fn catalog_error_string(error: provider::CatalogError) -> String {
    serde_json::to_string(&assistant_error(error)).expect("assistant error serializes")
}
