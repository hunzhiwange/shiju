//! Workflow editing drafts, identifiers, and validation.
use desktop_workflow_model::*;
use iced::{Point, Vector, widget::text_editor};
use serde_yaml::{Mapping, Value};
use web_time::{SystemTime, UNIX_EPOCH};
pub mod drafts;
pub mod ids;
pub mod node_validation;
pub mod start_variable_validation;
mod validation_helpers;
use drafts::*;
use ids::*;
use start_variable_validation::*;
pub use validation_helpers::*;
#[cfg(test)]
mod ids_tests;
pub mod app_ui;
pub mod canvas_ops;
pub mod state;

#[cfg(test)]
mod desktop_split_22_tests;
