//! Separation of the persisted configuration's top-level JSON fields.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

// The tenant row distinguishes an empty configuration from an uninitialized tenant.
pub(super) const READ_CONFIG: &str = "
    SELECT section.section_key, section.payload_json
    FROM system_config_tenants AS tenant
    LEFT JOIN system_config_sections AS section USING (platform_id, company_id)
    WHERE tenant.platform_id = $1 AND tenant.company_id = $2
    ORDER BY section_key";

pub(super) struct ConfigSnapshot {
    sections: Option<Map<String, Value>>,
}

pub(super) struct ConfigChanges {
    pub initialize: bool,
    pub upsert: Vec<(String, String)>,
    pub delete: Vec<String>,
}

impl ConfigSnapshot {
    pub fn from_rows<E: std::error::Error + Send + Sync + 'static>(
        rows: impl IntoIterator<Item = Result<(Option<String>, Option<String>), E>>,
    ) -> Result<Self> {
        let mut snapshot = Self { sections: None };
        for row in rows {
            let (key, payload) = row.context("read config section row")?;
            match (key, payload) {
                (Some(key), Some(payload)) => {
                    let value = serde_json::from_str(&payload).context("decode config section")?;
                    snapshot.sections.get_or_insert_with(Map::new).insert(key, value);
                }
                (None, None) => {
                    snapshot.sections = Some(Map::new());
                }
                _ => bail!("invalid persisted system config row"),
            }
        }
        Ok(snapshot)
    }

    pub fn payload(&self) -> Result<Option<String>> {
        self.sections
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("assemble system config sections")
    }

    pub fn changes(&self, payload: &str) -> Result<ConfigChanges> {
        let next = parse_sections(payload)?;
        let previous = self.sections.as_ref();
        if let Some(previous_servers) = previous
            .and_then(|sections| sections.get("mcpServers"))
            .and_then(Value::as_array)
        {
            let next_servers =
                next.get("mcpServers").and_then(Value::as_array).map(Vec::as_slice);
            crate::schema::config_validate::validate_builtin_mcp_servers_retained(
                previous_servers,
                next_servers.unwrap_or_default(),
            )?;
        }
        let mut changes = ConfigChanges {
            initialize: previous.is_none(),
            upsert: Vec::new(),
            delete: Vec::new(),
        };
        for (key, value) in &next {
            if previous.and_then(|sections| sections.get(key)) != Some(value) {
                changes.upsert.push((key.clone(), serde_json::to_string(value)?));
            }
        }
        if let Some(previous) = previous {
            changes.delete.extend(previous.keys().filter(|key| !next.contains_key(*key)).cloned());
        }
        Ok(changes)
    }
}

fn parse_sections(payload: &str) -> Result<Map<String, Value>> {
    let value =
        serde_json::from_str(payload).context("Failed to parse tenant system config payload")?;
    let Value::Object(sections) = value else {
        bail!("tenant system config payload root must be an object");
    };
    Ok(sections)
}

// These statements use parameter positions supported by both SQL backends.
pub(super) const INSERT_TENANT: &str =
    "INSERT INTO system_config_tenants (platform_id, company_id) VALUES ($1, $2)";

pub(super) const UPSERT_SECTION: &str = "
    INSERT INTO system_config_sections (
        platform_id, company_id, section_key, payload_json, updated_at_ms
    ) VALUES ($1, $2, $3, $4, $5)
    ON CONFLICT (platform_id, company_id, section_key) DO UPDATE SET
        payload_json = excluded.payload_json,
        updated_at_ms = excluded.updated_at_ms";

pub(super) const DELETE_SECTION: &str = "
    DELETE FROM system_config_sections
    WHERE platform_id = $1 AND company_id = $2 AND section_key = $3";
