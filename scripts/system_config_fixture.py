"""Seed the system configuration tables in an isolated SQLite fixture."""
import json
from pathlib import Path


SCHEMA = Path(__file__).resolve().parents[1] / "crates/libs/agent-config/src/store/schema.sql"


def seed_system_config(connection, config, *, platform="10000", company="100000000"):
    connection.executescript(SCHEMA.read_text(encoding="utf-8"))
    connection.execute(
        "INSERT INTO system_config_tenants (platform_id, company_id) VALUES (?, ?)",
        (platform, company),
    )
    connection.executemany(
        """INSERT INTO system_config_sections (
               platform_id, company_id, section_key, payload_json, updated_at_ms
           ) VALUES (?, ?, ?, ?, 1)""",
        ((platform, company, key, json.dumps(value)) for key, value in config.items()),
    )
