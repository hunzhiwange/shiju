pub use model_transport::routing_selection::*;
pub fn configured_protocol(model: &model_types::Model) -> Result<model_protocols::ModelProtocol, axum::http::StatusCode> {
    if model_protocol::responses::is_native(model) { return Ok(model_protocols::ModelProtocol::Responses); }
    configured_api_protocol(&model.api)
}
