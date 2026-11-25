//! 领域更新效果的宿主持久化适配。
use super::messages::*;
use crate::app::{App, Message};
use iced::Task;
use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn refresh_media_catalog(app: &mut App) -> Task<Message> {
    let state = &mut app.multimodal_settings;
    if state.saving_media_model {
        return Task::none();
    }
    state.media_catalog_request = state.media_catalog_request.wrapping_add(1);
    state.media_catalog = None;
    let request = state.media_catalog_request;
    let endpoint = crate::app::gateway_client_endpoint();
    Task::perform(
        async move {
            let client = vw_gateway_client::GatewayClient::new(endpoint).map_err(|_| ())?;
            let config = gateway_config::GatewayConfigClient::new(client.transport().clone());
            let (catalog, defaults) = tokio::join!(
                client.global_media_models(),
                config.global_config_get_path(&["multimodal", "media_models"]),
            );
            let defaults = match defaults.map_err(|_| ())? {
                Some(value) => serde_json::from_value(value).map_err(|_| ())?,
                None => Default::default(),
            };
            Ok((catalog.map_err(|_| ())?, defaults))
        },
        move |result| {
            Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MediaCatalogLoaded {
                request,
                result,
            }))
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn gateway_changed(app: &mut App) -> Task<Message> {
    let state = &mut app.multimodal_settings;
    state.media_catalog_request = state.media_catalog_request.wrapping_add(1);
    state.media_catalog = None;
    state.media_models.clear();
    state.saving_media_model = false;
    if app.model_settings.active_tab == crate::app::state::ModelConfigTab::Multimodal {
        return refresh_media_catalog(app);
    }
    Task::none()
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    use crate::app::config::{patch_full_agent_config_async, spawn_gateway_task};

    let SettingsMessage::Multimodal(message) = message else {
        return Task::none();
    };
    match message {
        #[cfg(not(target_arch = "wasm32"))]
        MultimodalMessage::MediaCatalogRefresh => refresh_media_catalog(app),
        #[cfg(target_arch = "wasm32")]
        MultimodalMessage::MediaCatalogRefresh => Task::none(),
        MultimodalMessage::MediaCatalogLoaded { request, result } => {
            let state = &mut app.multimodal_settings;
            if state.media_catalog_request == request {
                state.media_catalog = Some(result.map(|(catalog, defaults)| {
                    state.media_models = defaults;
                    catalog
                }));
            }
            Task::none()
        }
        MultimodalMessage::MediaModelChanged { operation, model } => {
            let state = &mut app.multimodal_settings;
            if state.saving_media_model
                || state.media_catalog.is_none()
                || state.media_models.get(&operation) == model.as_ref()
            {
                return Task::none();
            }
            state.saving_media_model = true;
            state.save_error = None;
            let request = state.media_catalog_request;
            let endpoint = crate::app::gateway_client_endpoint();
            let value = json!(model);
            Task::perform(
                async move {
                    let client = vw_gateway_client::GatewayClient::new(endpoint)?;
                    gateway_config::GatewayConfigClient::new(client.transport().clone())
                        .global_config_patch_path(
                            &["multimodal", "media_models", operation.config_key()],
                            value,
                        )
                        .await
                },
                move |result| {
                    Message::Settings(SettingsMessage::Multimodal(
                        MultimodalMessage::MediaModelSaved {
                            request,
                            operation,
                            model: model.clone(),
                            result,
                        },
                    ))
                },
            )
        }
        MultimodalMessage::MediaModelSaved { request, operation, model, result } => {
            let state = &mut app.multimodal_settings;
            if state.media_catalog_request != request || !state.saving_media_model {
                return Task::none();
            }
            state.saving_media_model = false;
            if result.is_ok() {
                if let Some(model) = model {
                    state.media_models.insert(operation, model);
                } else {
                    state.media_models.remove(&operation);
                }
            }
            desktop_settings_runtime::save_feedback::multimodal_saved(state, result.err());
            Task::none()
        }
        message => {
            let domain_message =
                desktop_settings_runtime::messages::SettingsMessage::Multimodal(message.clone());
            let Some(config) = desktop_settings_runtime::multimodal::update(
                &mut app.multimodal_settings,
                domain_message,
            ) else {
                return Task::none();
            };
            let patch = match message {
                MultimodalMessage::MaxImagesChanged(_) => {
                    json!({ "max_images": config.max_images })
                }
                MultimodalMessage::MaxImageSizeMbChanged(_) => {
                    json!({ "max_image_size_mb": config.max_image_size_mb })
                }
                MultimodalMessage::AllowRemoteFetchToggled(_) => {
                    json!({ "allow_remote_fetch": config.allow_remote_fetch })
                }
                _ => json!({ "ocr": config.ocr }),
            };
            spawn_gateway_task("multimodal", async move {
                patch_full_agent_config_async(json!({ "multimodal": patch })).await
            })
        }
    }
}
#[cfg(test)]
#[path = "multimodal_tests.rs"]
mod multimodal_tests;
