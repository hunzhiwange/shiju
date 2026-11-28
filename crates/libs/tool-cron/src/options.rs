use super::{
    delivery::{DeliveryPatch, delivery_schema},
    schedule::{apply_schedule, schedule_properties},
};
use agent_cron::CronJobPatch;
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub(super) fn job_properties(update: bool) -> Map<String, Value> {
    let mut properties = schedule_properties();
    properties.extend(Map::from_iter([
        ("name".into(), json!({"type": "string"})),
        ("job_type".into(), json!({"type": "string", "enum": ["shell", "agent"]})),
        ("command".into(), json!({"type": "string", "description": "Shell command for a Shell task"})),
        ("prompt".into(), json!({"type": "string", "description": "Instructions for an Agent task"})),
        ("session_target".into(), json!({"type": "string", "enum": ["isolated", "main"]})),
        ("model".into(), json!({"type": "string", "description": "Primary Agent model; empty uses the default"})),
        ("fallbacks".into(), json!({"type": "array", "items": {"type": "string"}, "description": "Fallback Agent models in retry order; [] clears the list"})),
        ("agent".into(), json!({"type": "string", "description": "Delegate Agent configuration key; empty uses the default"})),
        ("acp_agent".into(), json!({"type": "string", "description": "ACP Agent backend key; empty selects the native Agent"})),
        ("project_path".into(), json!({"type": "string", "description": if update { "Absolute project directory; empty clears it" } else { "Absolute project directory; defaults to the current workspace" }})),
        ("full_access".into(), json!({"type": "boolean", "description": "Run the task with full access when permitted by the current session"})),
        ("task_pool".into(), json!({"type": "boolean", "description": "Submit the Agent task to the project task pool"})),
        ("wake".into(), json!({"type": "boolean", "description": "Wake or notify the desktop when this task runs"})),
        ("delivery".into(), delivery_schema(update)),
        ("delete_after_run".into(), json!({"type": "boolean", "description": "Delete the task after its scheduled run when applicable"})),
        ("enabled".into(), json!({"type": "boolean", "description": "Enable or pause scheduled execution"})),
    ]));
    properties
}

pub(super) fn parse_options(args: &Value) -> Result<(CronJobPatch, Option<DeliveryPatch>), String> {
    let mut fields = args.as_object().cloned().ok_or("Expected a job options object")?;
    fields.remove("schedule");
    fields.remove("starts_at");
    fields.remove("ends_at");
    let delivery = fields
        .remove("delivery")
        .filter(|value| !value.is_null())
        .map(|value| DeliveryPatch::deserialize(value).map_err(|_| "Invalid delivery config"))
        .transpose()?;
    let mut patch = CronJobPatch::deserialize(Value::Object(fields))
        .map_err(|error| format!("Invalid job options: {error}"))?;
    apply_schedule(args, &mut patch)?;
    Ok((patch, delivery))
}
