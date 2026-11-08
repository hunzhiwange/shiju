//! Resolve capability evidence for one model in an already selected deployment.

use serde_json::Value;
use vw_api_types::model_catalog::{ModelCapabilityState, ModelMetadataDto};

pub struct ModelCapabilities<'a> {
    pub image_input: ModelCapabilityState,
    /// True only when the declaration or registry proves that text is the sole input modality.
    pub text_only: bool,
    pub tool_calls: ModelCapabilityState,
    pub reasoning: ModelCapabilityState,
    pub reasoning_efforts: Option<&'a [String]>,
}

/// Resolve explicit settings first, then the catalog row's modalities, then the
/// optional text-only registry. Callers supply settings and metadata for the
/// same deployment and model. Unknown is absence of evidence.
pub fn resolve_model_capabilities<'a>(
    model: &str,
    settings: Option<&Value>,
    metadata: &'a ModelMetadataDto,
    use_confirmed_registry: bool,
) -> ModelCapabilities<'a> {
    use ModelCapabilityState::{Supported, Unknown, Unsupported};

    let declaration =
        settings.and_then(|settings| settings_declaration(settings, model)).or_else(|| {
            let modes = metadata.input_modalities.as_ref()?;
            modality_declaration(modes.iter().map(|mode| Some(mode.as_str())))
        });
    let (image_input, text_only) = match declaration {
        Some(input) => (if input.image { Supported } else { Unsupported }, input.text_only),
        None if use_confirmed_registry && is_confirmed_text_only_model(model) => {
            (Unsupported, true)
        }
        None => (Unknown, false),
    };
    ModelCapabilities {
        image_input,
        text_only,
        tool_calls: metadata.tool_calls,
        reasoning: metadata.reasoning,
        reasoning_efforts: metadata.reasoning_efforts.as_deref(),
    }
}

struct InputDeclaration {
    image: bool,
    text_only: bool,
}

fn settings_declaration(settings: &Value, model: &str) -> Option<InputDeclaration> {
    if model.is_empty() {
        return None;
    }
    let catalog = settings.get("modelCatalog");
    [catalog.and_then(|catalog| catalog.get("models")), catalog, settings.get("models")]
        .into_iter()
        .flatten()
        .find_map(|rows| {
            if let Some(rows) = rows.as_array() {
                return rows.iter().find_map(|row| row_declaration(row, None, model));
            }
            rows.as_object()?.iter().find_map(|(key, row)| row_declaration(row, Some(key), model))
        })
}

fn row_declaration(row: &Value, key: Option<&str>, model: &str) -> Option<InputDeclaration> {
    let id = ["model", "id"]
        .into_iter()
        .find_map(|field| row.get(field).and_then(Value::as_str))
        .or(key)
        .or_else(|| row.get("name").and_then(Value::as_str));
    if id != Some(model) {
        return None;
    }

    let image_flag = ["supportsImage", "supports_image", "vision"]
        .into_iter()
        .find_map(|field| row.get(field).and_then(Value::as_bool));
    let modalities = [
        row.get("input"),
        row.pointer("/modalities/input"),
        row.get("input_modalities"),
        row.get("inputModalities"),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| modality_declaration(value.as_array()?.iter().map(Value::as_str)));

    match image_flag {
        Some(image) => Some(InputDeclaration {
            image,
            text_only: !image && modalities.is_some_and(|input| input.text_only),
        }),
        None => modalities,
    }
}

fn modality_declaration<'a>(
    modes: impl Iterator<Item = Option<&'a str>>,
) -> Option<InputDeclaration> {
    let mut image = false;
    let mut has_text = false;
    let mut has_other = false;
    for mode in modes {
        let mode = mode?.trim();
        if mode.is_empty() {
            return None;
        }
        image |= mode.eq_ignore_ascii_case("image");
        has_text |= mode.eq_ignore_ascii_case("text");
        has_other |= !mode.eq_ignore_ascii_case("text");
    }
    Some(InputDeclaration { image, text_only: has_text && !has_other })
}

/// Match only reviewed model names. Namespaces and the Claude 1M marker
/// do not identify variants, but model suffixes and legacy aliases do.
fn is_confirmed_text_only_model(model: &str) -> bool {
    let model = model.trim().rsplit('/').next().unwrap_or_default().to_ascii_lowercase();
    let model = model.strip_suffix("[1m]").unwrap_or(&model).trim_end();

    // Keep the dated Flash name, but leave the undated V4 Flash and V4 Pro
    // aliases to deployment declarations because their upstream routing varies.
    const CONFIRMED_NAMES: &[&str] = &[
        "ark-code-latest",
        "deepseek-chat",
        "deepseek-reasoner",
        "deepseek-v4-flash-0731",
        "glm-5.1",
        "glm-5.2",
        "glm-5.3",
        "kat-coder",
        "kat-coder-pro",
        "kat-coder-pro v1",
        "kat-coder-pro v2",
        "kat-coder-pro-v1",
        "kat-coder-pro-v2",
        "ling-2.5-1t",
        "ling-2.6-1t",
        "longcat-2.0",
        "longcat-flash-chat",
        "minimax-m2.7",
        "minimax-m2.7-highspeed",
        "mimo-v2.5-pro",
        "qwen3-coder-480b",
        "qwen3-coder-480b-a35b-instruct",
        "qwen3-coder-flash",
        "qwen3-coder-next",
        "qwen3-coder-plus",
        "step-3.5-flash",
        "step-3.5-flash-2603",
        "us.deepseek.r1-v1",
    ];
    CONFIRMED_NAMES.contains(&model)
}
