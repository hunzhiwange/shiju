use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElicitationField {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) required: bool,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationKind {
    Form { fields: Vec<ElicitationField> },
    Url { url: String, confirmed: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElicitationRequest {
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) kind: ElicitationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationResponse {
    Accept { values: BTreeMap<String, String> },
    Reject,
}

impl ElicitationRequest {
    pub(crate) fn response(&self, accept: bool) -> Result<ElicitationResponse, String> {
        if !accept {
            return Ok(ElicitationResponse::Reject);
        }
        match &self.kind {
            ElicitationKind::Form { fields } => {
                let mut values = BTreeMap::new();
                for field in fields {
                    if field.required && field.value.trim().is_empty() {
                        return Err(format!("required elicitation field is empty: {}", field.name));
                    }
                    values.insert(field.name.clone(), field.value.clone());
                }
                Ok(ElicitationResponse::Accept { values })
            }
            ElicitationKind::Url { url, confirmed } => {
                if !is_safe_url(url) {
                    return Err("elicitation URL must use http or https".to_string());
                }
                if !confirmed {
                    return Err("elicitation URL requires explicit user confirmation".to_string());
                }
                Ok(ElicitationResponse::Accept { values: BTreeMap::new() })
            }
        }
    }
}

fn is_safe_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
