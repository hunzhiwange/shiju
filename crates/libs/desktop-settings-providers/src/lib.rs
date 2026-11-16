//! 模型提供商设置领域。
pub mod state;
pub mod models;
pub mod model_routes;
pub mod view_message;
pub mod view_style;
pub mod models_view;
pub mod model_routes_view;
pub mod custom_model_modal;
pub mod model_tasks;
pub mod model_update;

pub mod app_init;

pub mod provider_state;
pub mod provider_models;
pub mod provider_tasks;
pub mod provider_catalog;
pub mod embedding_view_message;
pub mod embedding_routes_view;

pub mod catalog;
pub mod catalog_message;
pub mod connect;
pub mod connected;
pub mod custom_provider;
pub mod embedding_routes_update;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod connect_tests;
#[cfg(test)]
mod connected_tests;
#[cfg(test)]
mod custom_provider_tests;
#[cfg(test)]
mod provider_view_test_support;

pub mod provider_helpers;
pub mod provider_message;
pub mod provider_refresh;
pub mod provider_update;
pub mod catalog_api;
pub mod discovery;
pub mod metadata_fill;
#[cfg(not(target_arch = "wasm32"))]
pub mod managed_shiju;
#[cfg(test)]
mod catalog_api_tests;

pub mod routing_persistence;

#[cfg(test)]
mod provider_dispatch_tests;

pub mod settings_message;

pub mod settings_util;

pub mod save_feedback;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_11_tests;

#[cfg(test)]
mod desktop_split_28_tests;

#[cfg(test)]
mod host_module_17_tests;
