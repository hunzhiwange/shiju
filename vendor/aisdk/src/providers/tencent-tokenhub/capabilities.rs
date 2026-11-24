//! Capabilities for tencent_tokenhub models.
//!
//! This module defines model types and their capabilities for tencent_tokenhub providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::tencent_tokenhub::TencentTokenhub;

model_capabilities! {
    provider: TencentTokenhub,
    models: {
        Hy3 {
            model_name: "hy3",
            constructor_name: hy3,
            display_name: "Hy3",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Hy3Preview {
            model_name: "hy3-preview",
            constructor_name: hy3_preview,
            display_name: "Hy3 preview",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
        Hy4Preview {
            model_name: "hy4-preview",
            constructor_name: hy4_preview,
            display_name: "Hy4 preview",
            capabilities: [ReasoningSupport, TextInputSupport, TextOutputSupport, ToolCallSupport]
        },
    }
}
