//! Parsing for complete model references.

use std::str::FromStr;

use crate::{Error, Result};

/// A complete model reference split into its provider and model identifiers.
///
/// Model references use the form `provider/model`. Only the first slash is
/// treated as a separator, so nested model identifiers remain intact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRef {
    /// The provider identifier before the first slash.
    pub provider_id: String,
    /// The model identifier after the first slash.
    pub model_id: String,
}

impl ModelRef {
    /// Parses a complete `provider/model` reference.
    ///
    /// Surrounding whitespace is removed from each identifier. The reference
    /// is rejected when it has no slash or either identifier is empty.
    pub fn parse(reference: &str) -> Result<Self> {
        let (provider_id, model_id) = reference.split_once('/').ok_or_else(|| {
            Error::InvalidInput(
                "model reference must use the complete `provider/model` form".into(),
            )
        })?;
        let provider_id = provider_id.trim();
        let model_id = model_id.trim();

        if provider_id.is_empty() {
            return Err(Error::InvalidInput(
                "model reference provider ID must not be empty".into(),
            ));
        }
        if model_id.is_empty() {
            return Err(Error::InvalidInput(
                "model reference model ID must not be empty".into(),
            ));
        }

        Ok(Self {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
        })
    }
}

impl FromStr for ModelRef {
    type Err = Error;

    fn from_str(reference: &str) -> Result<Self> {
        Self::parse(reference)
    }
}

#[cfg(test)]
mod tests {
    use super::ModelRef;
    use crate::Error;

    #[test]
    fn model_ref_parses_provider_and_versioned_model() {
        let model_ref = ModelRef::parse(" alibaba-cn/qwen3.7-plus-2026-05-26 ")
            .expect("valid model reference should parse");

        assert_eq!(model_ref.provider_id, "alibaba-cn");
        assert_eq!(model_ref.model_id, "qwen3.7-plus-2026-05-26");
        assert_eq!(model_ref.clone(), model_ref);
    }

    #[test]
    fn model_ref_preserves_nested_model_path() {
        let model_ref: ModelRef = " provider/org/model version "
            .parse()
            .expect("nested model reference should parse");

        assert_eq!(model_ref.provider_id, "provider");
        assert_eq!(model_ref.model_id, "org/model version");
    }

    #[test]
    fn model_ref_rejects_incomplete_references() {
        for input in ["provider", "/model", "provider/", " /model", "provider/ "] {
            assert!(
                matches!(input.parse::<ModelRef>(), Err(Error::InvalidInput(_))),
                "{input:?} should be rejected as invalid input"
            );
        }
    }
}
