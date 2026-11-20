CREATE TABLE IF NOT EXISTS system_config_tenants (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    PRIMARY KEY (platform_id, company_id)
);

CREATE TABLE IF NOT EXISTS system_config_sections (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    section_key TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, section_key),
    FOREIGN KEY (platform_id, company_id) REFERENCES system_config_tenants (platform_id, company_id)
);

CREATE TABLE IF NOT EXISTS project_configs (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_path TEXT NOT NULL CHECK (trim(project_path) <> ''),
    payload_json TEXT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_path)
);
