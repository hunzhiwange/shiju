//! Capabilities for lilac models.
//!
//! This module defines model types and their capabilities for lilac providers.
//! Users can implement additional traits on custom models.

use crate::core::capabilities::*;
use crate::model_capabilities;
use crate::providers::lilac::Lilac;

model_capabilities! {
    provider: Lilac,
    models: {
    }
}
