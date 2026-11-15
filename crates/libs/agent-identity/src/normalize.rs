use crate::model::{
    AieosIdentity, CapabilitiesSection, HistorySection, IdentitySection, InterestsSection,
    LinguisticsSection, MotivationsSection, Names, OceanTraits, PhysicalitySection,
    PsychologySection,
};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// 规范化 AIEOS 身份数据。
///
/// 将原始 JSON 值转换为强类型的 `AieosIdentity` 结构，
/// 对各个数据段进行规范化处理。
pub fn normalize_aieos_identity(payload: &Value) -> AieosIdentity {
    AieosIdentity {
        identity: normalize_identity_section(value_at_path(payload, &["identity"])),
        psychology: normalize_psychology_section(value_at_path(payload, &["psychology"])),
        linguistics: normalize_linguistics_section(value_at_path(payload, &["linguistics"])),
        motivations: normalize_motivations_section(value_at_path(payload, &["motivations"])),
        capabilities: normalize_capabilities_section(value_at_path(payload, &["capabilities"])),
        physicality: normalize_physicality_section(value_at_path(payload, &["physicality"])),
        history: normalize_history_section(value_at_path(payload, &["history"])),
        interests: normalize_interests_section(value_at_path(payload, &["interests"])),
    }
}

fn normalize_identity_section(section: Option<&Value>) -> Option<IdentitySection> {
    let section = section?;

    let names = normalize_names(value_at_path(section, &["names"]));
    let bio = value_at_path(section, &["bio"]).and_then(value_to_text);
    let origin = value_at_path(section, &["origin"]).and_then(value_to_text);
    let residence = value_at_path(section, &["residence"]).and_then(value_to_text);

    if names.is_none() && bio.is_none() && origin.is_none() && residence.is_none() {
        return None;
    }

    Some(IdentitySection { names, bio, origin, residence })
}

fn normalize_names(value: Option<&Value>) -> Option<Names> {
    let value = value?;

    let mut names = Names {
        first: value_at_path(value, &["first"]).and_then(scalar_to_string),
        last: value_at_path(value, &["last"]).and_then(scalar_to_string),
        nickname: value_at_path(value, &["nickname"]).and_then(scalar_to_string),
        full: value_at_path(value, &["full"]).and_then(scalar_to_string),
    };

    if names.full.is_none() {
        if let (Some(first), Some(last)) = (&names.first, &names.last) {
            names.full = Some(format!("{first} {last}"));
        }
    }

    if names.first.is_none()
        && names.last.is_none()
        && names.nickname.is_none()
        && names.full.is_none()
    {
        return None;
    }

    Some(names)
}

fn normalize_psychology_section(section: Option<&Value>) -> Option<PsychologySection> {
    let section = section?;

    let neural_matrix = value_at_path(section, &["neural_matrix"]).and_then(numeric_map_from_value);
    let mbti = value_at_path(section, &["mbti"])
        .and_then(scalar_to_string)
        .or_else(|| value_at_path(section, &["traits", "mbti"]).and_then(scalar_to_string));
    let ocean = value_at_path(section, &["ocean"])
        .or_else(|| value_at_path(section, &["traits", "ocean"]))
        .and_then(normalize_ocean_traits);
    let moral_compass = value_at_path(section, &["moral_compass"])
        .map(normalize_moral_compass)
        .filter(|items| !items.is_empty());

    if neural_matrix.is_none() && mbti.is_none() && ocean.is_none() && moral_compass.is_none() {
        return None;
    }

    Some(PsychologySection { neural_matrix, mbti, ocean, moral_compass })
}

fn normalize_ocean_traits(value: &Value) -> Option<OceanTraits> {
    let value = value.as_object()?;

    let traits = OceanTraits {
        openness: value.get("openness").and_then(numeric_from_value),
        conscientiousness: value.get("conscientiousness").and_then(numeric_from_value),
        extraversion: value.get("extraversion").and_then(numeric_from_value),
        agreeableness: value.get("agreeableness").and_then(numeric_from_value),
        neuroticism: value.get("neuroticism").and_then(numeric_from_value),
    };

    if traits.openness.is_none()
        && traits.conscientiousness.is_none()
        && traits.extraversion.is_none()
        && traits.agreeableness.is_none()
        && traits.neuroticism.is_none()
    {
        return None;
    }

    Some(traits)
}

fn normalize_moral_compass(value: &Value) -> Vec<String> {
    let mut values = Vec::new();

    if let Some(map) = value.as_object() {
        if let Some(alignment) = map.get("alignment").and_then(scalar_to_string) {
            values.push(format!("Alignment: {alignment}"));
        }

        if let Some(core_values) = map.get("core_values") {
            values.extend(list_from_value(core_values));
        }

        if let Some(conflict_style) =
            map.get("conflict_resolution_style").and_then(scalar_to_string)
        {
            values.push(format!("Conflict style: {conflict_style}"));
        }

        if values.is_empty() {
            values.extend(list_from_value(value));
        }
    } else {
        values.extend(list_from_value(value));
    }

    dedupe_non_empty(values)
}

fn normalize_linguistics_section(section: Option<&Value>) -> Option<LinguisticsSection> {
    let section = section?;

    let style = value_at_path(section, &["style"]).and_then(value_to_text).or_else(|| {
        non_empty_list_at(section, &["text_style", "style_descriptors"]).map(|list| list.join(", "))
    });

    let formality = value_at_path(section, &["formality"]).and_then(value_to_text).or_else(|| {
        value_at_path(section, &["text_style", "formality_level"]).and_then(|value| {
            numeric_from_value(value)
                .map(|number| format!("{number:.2}"))
                .or_else(|| value_to_text(value))
        })
    });

    let catchphrases = non_empty_list_at(section, &["catchphrases"])
        .or_else(|| non_empty_list_at(section, &["idiolect", "catchphrases"]));
    let forbidden_words = non_empty_list_at(section, &["forbidden_words"])
        .or_else(|| non_empty_list_at(section, &["idiolect", "forbidden_words"]));

    if style.is_none() && formality.is_none() && catchphrases.is_none() && forbidden_words.is_none()
    {
        return None;
    }

    Some(LinguisticsSection { style, formality, catchphrases, forbidden_words })
}

fn normalize_motivations_section(section: Option<&Value>) -> Option<MotivationsSection> {
    let section = section?;

    let core_drive = value_at_path(section, &["core_drive"]).and_then(value_to_text);
    let short_term_goals = non_empty_list_at(section, &["short_term_goals"])
        .or_else(|| non_empty_list_at(section, &["goals", "short_term"]));
    let long_term_goals = non_empty_list_at(section, &["long_term_goals"])
        .or_else(|| non_empty_list_at(section, &["goals", "long_term"]));
    let fears = value_at_path(section, &["fears"]).and_then(|fears| {
        let values = if fears.is_object() {
            let mut combined =
                non_empty_list_at(section, &["fears", "rational"]).unwrap_or_default();
            if let Some(mut irrational) = non_empty_list_at(section, &["fears", "irrational"]) {
                combined.append(&mut irrational);
            }
            if combined.is_empty() { list_from_value(fears) } else { combined }
        } else {
            list_from_value(fears)
        };

        let deduped = dedupe_non_empty(values);
        if deduped.is_empty() { None } else { Some(deduped) }
    });

    if core_drive.is_none()
        && short_term_goals.is_none()
        && long_term_goals.is_none()
        && fears.is_none()
    {
        return None;
    }

    Some(MotivationsSection { core_drive, short_term_goals, long_term_goals, fears })
}

fn normalize_capabilities_section(section: Option<&Value>) -> Option<CapabilitiesSection> {
    let section = section?;

    let skills = non_empty_list_at(section, &["skills"]);
    let tools = non_empty_list_at(section, &["tools"]);

    if skills.is_none() && tools.is_none() {
        return None;
    }

    Some(CapabilitiesSection { skills, tools })
}

fn normalize_physicality_section(section: Option<&Value>) -> Option<PhysicalitySection> {
    let section = section?;

    let appearance =
        value_at_path(section, &["appearance"]).and_then(value_to_text).or_else(|| {
            let mut descriptors = Vec::new();

            if let Some(face_shape) =
                value_at_path(section, &["face", "shape"]).and_then(scalar_to_string)
            {
                descriptors.push(format!("脸型: {face_shape}"));
            }

            if let Some(build_description) =
                value_at_path(section, &["body", "build_description"]).and_then(scalar_to_string)
            {
                descriptors.push(format!("体型: {build_description}"));
            }

            if let Some(aesthetic) =
                value_at_path(section, &["style", "aesthetic_archetype"]).and_then(scalar_to_string)
            {
                descriptors.push(format!("审美: {aesthetic}"));
            }

            if descriptors.is_empty() { None } else { Some(descriptors.join("; ")) }
        });

    let avatar_description = value_at_path(section, &["avatar_description"])
        .and_then(value_to_text)
        .or_else(|| value_at_path(section, &["image_prompts", "portrait"]).and_then(value_to_text));

    if appearance.is_none() && avatar_description.is_none() {
        return None;
    }

    Some(PhysicalitySection { appearance, avatar_description })
}

fn normalize_history_section(section: Option<&Value>) -> Option<HistorySection> {
    let section = section?;

    let origin_story = value_at_path(section, &["origin_story"]).and_then(value_to_text);
    let education = non_empty_list_at(section, &["education"]);
    let occupation = value_at_path(section, &["occupation"]).and_then(value_to_text);

    if origin_story.is_none() && education.is_none() && occupation.is_none() {
        return None;
    }

    Some(HistorySection { origin_story, education, occupation })
}

fn normalize_interests_section(section: Option<&Value>) -> Option<InterestsSection> {
    let section = section?;

    let hobbies = non_empty_list_at(section, &["hobbies"]);
    let favorites = value_at_path(section, &["favorites"]).and_then(favorites_map);
    let lifestyle = value_at_path(section, &["lifestyle"]).and_then(value_to_text);

    if hobbies.is_none() && favorites.is_none() && lifestyle.is_none() {
        return None;
    }

    Some(InterestsSection { hobbies, favorites, lifestyle })
}

fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.as_object()?.get(*segment)?;
    }
    Some(current)
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_owned()) }
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        _ => None,
    }
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(_) | Value::Number(_) | Value::Bool(_) => scalar_to_string(value),
        Value::Array(_) => {
            let values = list_from_value(value);
            if values.is_empty() { None } else { Some(values.join(", ")) }
        }
        Value::Object(map) => summarize_object(map),
    }
}

fn summarize_object(map: &Map<String, Value>) -> Option<String> {
    let mut parts = Vec::new();
    summarize_object_into_parts("", map, &mut parts);
    if parts.is_empty() { None } else { Some(parts.join("; ")) }
}

fn summarize_object_into_parts(prefix: &str, map: &Map<String, Value>, parts: &mut Vec<String>) {
    for (key, value) in map {
        if key.starts_with('@') {
            continue;
        }

        let label = key.replace('_', " ");
        let full_label = if prefix.is_empty() { label } else { format!("{prefix} {label}") };

        match value {
            Value::Object(inner) => summarize_object_into_parts(&full_label, inner, parts),
            Value::Array(_) => {
                let values = list_from_value(value);
                if !values.is_empty() {
                    parts.push(format!("{full_label}: {}", values.join(", ")));
                }
            }
            _ => {
                if let Some(text) = scalar_to_string(value) {
                    parts.push(format!("{full_label}: {text}"));
                }
            }
        }
    }
}

fn list_from_value(value: &Value) -> Vec<String> {
    let mut values = Vec::new();

    match value {
        Value::Array(entries) => {
            for entry in entries {
                values.extend(list_from_value(entry));
            }
        }
        Value::Object(map) => {
            if let Some(name) = map.get("name").and_then(scalar_to_string) {
                values.push(name);
            } else if let Some(title) = map.get("title").and_then(scalar_to_string) {
                values.push(title);
            } else if let Some(summary) = summarize_object(map) {
                values.push(summary);
            }
        }
        _ => {
            if let Some(text) = scalar_to_string(value) {
                values.push(text);
            }
        }
    }

    dedupe_non_empty(values)
}

fn dedupe_non_empty(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !deduped.iter().any(|existing: &String| existing.eq_ignore_ascii_case(trimmed)) {
            deduped.push(trimmed.to_owned());
        }
    }
    deduped
}

fn numeric_map_from_value(value: &Value) -> Option<HashMap<String, f64>> {
    let map = value.as_object()?;
    let mut numeric_values = HashMap::new();

    for (key, entry) in map {
        if key.starts_with('@') {
            continue;
        }
        if let Some(number) = numeric_from_value(entry) {
            numeric_values.insert(key.clone(), number);
        }
    }

    if numeric_values.is_empty() { None } else { Some(numeric_values) }
}

fn numeric_from_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn favorites_map(value: &Value) -> Option<HashMap<String, String>> {
    let map = value.as_object()?;
    let mut favorites = HashMap::new();

    for (key, entry) in map {
        if key.starts_with('@') {
            continue;
        }
        if let Some(text) = value_to_text(entry) {
            favorites.insert(key.clone(), text);
        }
    }

    if favorites.is_empty() { None } else { Some(favorites) }
}

fn non_empty_list_at(value: &Value, path: &[&str]) -> Option<Vec<String>> {
    let values = value_at_path(value, path).map(list_from_value)?;
    if values.is_empty() { None } else { Some(values) }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod normalize_tests;
