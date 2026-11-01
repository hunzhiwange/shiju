//! User-owned MCP form replies. Never synthesize authorization from model output.
use crate::{question, session::ui_store};
use rmcp::model::{
    CreateElicitationRequestParam, CreateElicitationResult, ElicitationAction, ErrorData,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct FormOwner {
    pub(crate) session: String,
    pub(crate) tenant: vw_api_types::id::TenantScope,
}
fn invalid() -> ErrorData {
    ErrorData::invalid_params("Invalid MCP form schema or user response", None)
}
fn result(action: ElicitationAction) -> CreateElicitationResult {
    CreateElicitationResult { action, content: None }
}

pub(crate) fn validate(schema: &Value, content: &Value) -> Result<(), ErrorData> {
    let fields = schema["properties"].as_object().ok_or_else(invalid)?;
    let values = content.as_object().ok_or_else(invalid)?;
    if values.keys().any(|k| !fields.contains_key(k)) {
        return Err(invalid());
    }
    if let Some(required) = schema["required"].as_array() {
        for name in required {
            if !values.contains_key(name.as_str().ok_or_else(invalid)?) {
                return Err(invalid());
            }
        }
    }
    for (name, value) in values {
        let field = &fields[name];
        match field["type"].as_str() {
            Some("string") => {
                let text = value.as_str().ok_or_else(invalid)?;
                let len = text.chars().count() as u64;
                if field["minLength"].as_u64().is_some_and(|n| len < n)
                    || field["maxLength"].as_u64().is_some_and(|n| len > n)
                    || field["enum"].as_array().is_some_and(|xs| !xs.contains(value))
                {
                    return Err(invalid());
                }
                if let Some(format) = field["format"].as_str() {
                    let valid = match format {
                        "uri" => url::Url::parse(text).is_ok(),
                        "date" => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok(),
                        "date-time" => chrono::DateTime::parse_from_rfc3339(text).is_ok(),
                        // Conservative mailbox syntax; ambiguous/quoted addresses fail closed.
                        "email" => text.split_once('@').is_some_and(|(local, domain)| {
                            !local.is_empty()
                                && !local.starts_with('.')
                                && !local.ends_with('.')
                                && !local.contains("..")
                                && local.bytes().all(|c| {
                                    c.is_ascii_alphanumeric()
                                        || b".!#$%&'*+-/=?^_`{|}~".contains(&c)
                                })
                                && !domain.is_empty()
                                && domain.split('.').all(|label| {
                                    !label.is_empty()
                                        && !label.starts_with('-')
                                        && !label.ends_with('-')
                                        && label
                                            .bytes()
                                            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                                })
                        }),
                        _ => false,
                    };
                    if !valid {
                        return Err(invalid());
                    }
                }
            }
            Some("integer") => {
                let number = value
                    .as_i64()
                    .map(i128::from)
                    .or_else(|| value.as_u64().map(i128::from))
                    .ok_or_else(invalid)?;
                if field["minimum"].as_i64().is_some_and(|n| number < i128::from(n))
                    || field["maximum"].as_i64().is_some_and(|n| number > i128::from(n))
                {
                    return Err(invalid());
                }
            }
            Some("number") => {
                let number = value.as_f64().ok_or_else(invalid)?;
                if field["minimum"].as_f64().is_some_and(|n| number < n)
                    || field["maximum"].as_f64().is_some_and(|n| number > n)
                {
                    return Err(invalid());
                }
            }
            Some("boolean") if value.is_boolean() => {}
            _ => return Err(invalid()),
        }
    }
    Ok(())
}

pub(crate) async fn request(
    owner: FormOwner,
    server: String,
    request: CreateElicitationRequestParam,
    cancelled: CancellationToken,
    closed: CancellationToken,
    wire_schema: Option<Value>,
) -> Result<CreateElicitationResult, ErrorData> {
    let schema = match wire_schema {
        Some(schema) => schema,
        None => serde_json::to_value(&request.requested_schema).map_err(|_| invalid())?,
    };
    if schema["type"] != "object" {
        return Err(invalid());
    }
    let allowed = ["type", "title", "description", "properties", "required"];
    if schema.as_object().ok_or_else(invalid)?.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid());
    }
    let fields = schema["properties"].as_object().ok_or_else(invalid)?;
    if schema["required"].as_array().is_some_and(|names| {
        names.iter().any(|name| name.as_str().is_none_or(|key| !fields.contains_key(key)))
    }) {
        return Err(invalid());
    }
    for field in fields.values() {
        let allowed = [
            "type",
            "title",
            "description",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "format",
            "enum",
            "enumNames",
            "default",
        ];
        if field.as_object().ok_or_else(invalid)?.keys().any(|key| !allowed.contains(&key.as_str()))
        {
            return Err(invalid());
        }
    }
    let mut questions = vec![];
    for (name, field) in fields {
        let options = match field["type"].as_str() {
            Some("boolean") => vec!["true".to_owned(), "false".to_owned()],
            Some("string") => field["enum"]
                .as_array()
                .map(|xs| xs.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default(),
            Some("number" | "integer") => vec![],
            _ => return Err(invalid()),
        };
        questions.push(question::Info {
            question: format!(
                "{}\n{}\n{}",
                request.message,
                name,
                field["description"].as_str().unwrap_or("")
            ),
            header: server.clone(),
            options: options
                .into_iter()
                .map(|label| question::OptionInfo {
                    label,
                    description: String::new(),
                    preview: None,
                })
                .collect(),
            multiple: Some(false),
            custom: Some(true),
        });
    }
    questions.push(question::Info {
        question: "Submit this information to the MCP server?".into(),
        header: server.clone(),
        options: ["accept", "decline"]
            .into_iter()
            .map(|label| question::OptionInfo {
                label: label.into(),
                description: String::new(),
                preview: None,
            })
            .collect(),
        multiple: Some(false),
        custom: Some(false),
    });
    let future = ui_store::with_tenant_scope(
        owner.tenant,
        question::ask_sensitive(
            question::AskInput { session_id: owner.session, questions, tool: None },
            vw_shared::question::McpForm { server, schema: schema.clone() },
        ),
    );
    let answers = tokio::select! {
        biased;
        _ = cancelled.cancelled() => return Ok(result(ElicitationAction::Cancel)),
        _ = closed.cancelled() => return Ok(result(ElicitationAction::Cancel)),
        _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => return Ok(result(ElicitationAction::Cancel)),
        answers = future => answers,
    };
    let answers = match answers {
        Ok(answers) => answers,
        Err(question::Error::Rejected(_)) => return Ok(result(ElicitationAction::Cancel)),
        Err(_) => return Err(ErrorData::internal_error("MCP form interaction failed", None)),
    };
    if answers.len() != fields.len() + 1 {
        return Err(invalid());
    }
    match answers.last().map(Vec::as_slice) {
        Some([action]) if action == "decline" => return Ok(result(ElicitationAction::Decline)),
        Some([action]) if action == "accept" => {}
        _ => return Err(invalid()),
    }
    let mut content = serde_json::Map::new();
    for ((name, field), answer) in fields.iter().zip(&answers) {
        if answer.is_empty() {
            continue;
        }
        let [text] = answer.as_slice() else {
            return Err(invalid());
        };
        let value = if field["type"] == "string" {
            json!(text)
        } else {
            serde_json::from_str(text).map_err(|_| invalid())?
        };
        content.insert(name.clone(), value);
    }
    let content = Value::Object(content);
    validate(&schema, &content)?;
    Ok(CreateElicitationResult { action: ElicitationAction::Accept, content: Some(content) })
}
