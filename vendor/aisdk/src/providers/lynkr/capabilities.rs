//! Capabilities for lynkr models.
//!
//! This module defines model types and their capabilities for lynkr providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::lynkr::Lynkr;

model_capabilities! {
    provider: Lynkr,
    models: {
        LynkrAuto {
            model_name: "lynkr-auto",
            constructor_name: lynkr_auto,
            display_name: "Lynkr Auto (complexity routing)",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
