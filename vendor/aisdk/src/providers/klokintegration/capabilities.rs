//! Capabilities for klokintegration models.
//!
//! This module defines model types and their capabilities for klokintegration providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::klokintegration::Klokintegration;

model_capabilities! {
    provider: Klokintegration,
    models: {
        Kloker {
            model_name: "Kloker",
            constructor_name: kloker,
            display_name: "Kloker",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KlokerIntegrationArchitect {
            model_name: "Kloker-Integration-Architect",
            constructor_name: kloker_integration_architect,
            display_name: "Kloker Integration Architect",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        KlokerIntegrationDeveloper {
            model_name: "Kloker-Integration-Developer",
            constructor_name: kloker_integration_developer,
            display_name: "Kloker Integration Developer",
            capabilities: [TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
