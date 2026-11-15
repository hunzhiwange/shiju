use super::*;

pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, OmpProfile>, WriteError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return invalid();
    }
    let yaml: serde_yaml::Value =
        serde_yaml::from_slice(bytes).map_err(|_| WriteError::InvalidResource)?;
    let doc = serde_json::to_value(yaml).map_err(|_| WriteError::InvalidResource)?;
    let doc = if doc.is_null() { serde_json::json!({}) } else { doc };
    if !doc.is_object() {
        return invalid();
    }
    let mut rows = BTreeMap::new();
    if let Some(v) = doc.get("providers").filter(|v| !v.is_null()) {
        let map = v.as_object().ok_or(WriteError::InvalidResource)?;
        for (id, v) in map {
            super::super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            let p = OmpProfile {
                name: v.get("name").and_then(Value::as_str).unwrap_or(id).into(),
                declaration: v.clone(),
            };
            if !p.declaration.is_object() {
                return invalid();
            }
            rows.insert(id.clone(), p);
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<OmpState, WriteError> {
    Ok(OmpState { members: declarations(pre)?.into_keys().collect() })
}
/// Add never replaces another declaration. IfMember edits cannot enable a disabled row.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    declaration: Option<&Value>,
    activation: OmpActivation,
) -> Result<Vec<u8>, WriteError> {
    super::super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let rows = declarations(pre)?;
    if let Some(declaration) = declaration {
        validate(&OmpProfile { name: id.into(), declaration: declaration.clone() })?;
        match activation {
            OmpActivation::CatalogOnly => return invalid(),
            OmpActivation::Add => {
                if let Some(existing) = rows.get(id) {
                    return if existing.declaration == *declaration {
                        Ok(pre.unwrap_or_default().to_vec())
                    } else {
                        Err(WriteError::Conflict)
                    };
                }
            }
            OmpActivation::IfMember if !rows.contains_key(id) => {
                return Ok(pre.unwrap_or_default().to_vec());
            }
            OmpActivation::IfMember => {}
        }
    } else if !rows.contains_key(id) {
        return Ok(pre.unwrap_or_default().to_vec());
    }
    super::super::live::patch_yaml::patch_provider(
        pre,
        super::super::live::patch_yaml::ProviderSection::Providers,
        id,
        declaration,
    )
    .map_err(|_| WriteError::InvalidResource)
}
