//! ApiQL 实体元数据注册表。
//!
//! Rust 侧不能反射 PHP Entity，因此这里用显式配置完成实体 key、表、字段、
//! 搜索字段和虚拟字段依赖的校验与查找。

use std::collections::HashMap;
#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use std::time::{Duration, Instant};

#[cfg(not(test))]
use once_cell::sync::Lazy;
#[cfg(not(test))]
use parking_lot::Mutex;
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;
use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;

pub(crate) const APIQL_ENTITY_SETTINGS_NAMESPACE: &str = "apiql.entities";
#[cfg(not(test))]
const APIQL_ENTITY_REGISTRY_CACHE_TTL: Duration = Duration::from_secs(2);

#[cfg(not(test))]
static APIQL_ENTITY_REGISTRY_CACHE: Lazy<Mutex<Option<ApiQlEntityRegistryCache>>> =
    Lazy::new(|| Mutex::new(None));

#[cfg(not(test))]
#[derive(Debug, Clone)]
struct ApiQlEntityRegistryCache {
    tenant: TenantScope,
    loaded_at: Instant,
    registry: Arc<ApiQlEntityRegistry>,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiQlEntityRegistry {
    configs: HashMap<String, ApiQlEntityConfig>,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiQlEntityConfigStore {
    settings_store: GatewaySettingsStore,
    tenant: TenantScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiQlSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiQlOrderBy {
    pub(crate) field: String,
    pub(crate) direction: ApiQlSortDirection,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ApiQlResolvedEntity<'a> {
    config: &'a ApiQlEntityConfig,
}

impl ApiQlEntityRegistry {
    pub(crate) fn from_configs(configs: Vec<ApiQlEntityConfig>) -> Result<Self, ApiError> {
        let mut by_key = HashMap::new();
        for config in configs {
            validate_entity_key(&config.entity_class)?;
            validate_entity_config(&config)?;
            if by_key.insert(config.entity_class.clone(), config).is_some() {
                return Err(ApiError::bad_request("Duplicate ApiQL entity config"));
            }
        }
        Ok(Self { configs: by_key })
    }

    pub(crate) fn resolve(&self, entity_key: &str) -> Result<ApiQlResolvedEntity<'_>, ApiError> {
        validate_entity_key(entity_key)?;
        self.configs.get(entity_key).map(|config| ApiQlResolvedEntity { config }).ok_or_else(|| {
            ApiError::not_found(format!("ApiQL entity not registered: {entity_key}"))
        })
    }
}

impl ApiQlEntityConfigStore {
    pub(crate) fn new(database: GlobalDatabase, tenant: TenantScope) -> Result<Self, ApiError> {
        Ok(Self {
            settings_store: GatewaySettingsStore::new(database).map_err(internal_storage_error)?,
            tenant,
        })
    }

    pub(crate) fn load_configs(&self) -> Result<Vec<ApiQlEntityConfig>, ApiError> {
        self.settings_store
            .list(&self.tenant, APIQL_ENTITY_SETTINGS_NAMESPACE)
            .map_err(internal_storage_error)?
            .into_iter()
            .map(|setting| {
                validate_entity_key(&setting.config_key)?;
                let config: ApiQlEntityConfig = serde_json::from_str(&setting.payload_json)
                    .map_err(|error| {
                        ApiError::internal(format!(
                            "Invalid ApiQL entity config in shared database: {error}"
                        ))
                    })?;
                if config.entity_class != setting.config_key {
                    return Err(ApiError::bad_request(format!(
                        "ApiQL entity config key does not match entity_class: expected {}, got {}",
                        setting.config_key, config.entity_class
                    )));
                }
                Ok(config)
            })
            .collect()
    }

    pub(crate) fn save_configs(&self, configs: &[ApiQlEntityConfig]) -> Result<(), ApiError> {
        for config in configs {
            validate_entity_key(&config.entity_class)?;
            let payload = serde_json::to_string(config).map_err(|error| {
                ApiError::internal(format!("Failed to encode ApiQL entity config: {error}"))
            })?;
            self.settings_store
                .upsert(
                    &self.tenant,
                    APIQL_ENTITY_SETTINGS_NAMESPACE,
                    &config.entity_class,
                    &payload,
                )
                .map_err(internal_storage_error)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn delete_config(&self, entity_class: &str) -> Result<(), ApiError> {
        validate_entity_key(entity_class)?;
        self.settings_store
            .delete(&self.tenant, APIQL_ENTITY_SETTINGS_NAMESPACE, entity_class)
            .map_err(internal_storage_error)?;
        Ok(())
    }
}

impl<'a> ApiQlResolvedEntity<'a> {
    pub(crate) fn entity_key(&self) -> &'a str {
        &self.config.entity_class
    }

    pub(crate) fn config(&self) -> &'a ApiQlEntityConfig {
        self.config
    }

    pub(crate) fn search_field_names(&self) -> Vec<&'a str> {
        if self.config.search_fields.is_empty() {
            return self
                .config
                .fields
                .iter()
                .filter(|field| field.searchable)
                .map(|field| field.name.as_str())
                .collect();
        }

        self.config.search_fields.iter().map(String::as_str).collect()
    }

    pub(crate) fn virtual_field_dependencies(
        &self,
        field_name: &str,
    ) -> Result<Vec<&'a str>, ApiError> {
        let mut dependencies = Vec::new();
        self.collect_virtual_field_dependencies(field_name, &mut dependencies, &mut Vec::new())?;
        Ok(dependencies)
    }

    pub(crate) fn parse_default_order(
        &self,
        order_by: &str,
    ) -> Result<Vec<ApiQlOrderBy>, ApiError> {
        let mut parsed = Vec::new();
        for part in order_by.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut tokens = trimmed.split_whitespace();
            let field = tokens
                .next()
                .ok_or_else(|| ApiError::bad_request("Invalid ApiQL default order"))?;
            let direction = match tokens.next().unwrap_or("asc").to_ascii_lowercase().as_str() {
                "asc" => ApiQlSortDirection::Asc,
                "desc" => ApiQlSortDirection::Desc,
                _ => return Err(ApiError::bad_request("Invalid ApiQL default order direction")),
            };
            if tokens.next().is_some() {
                return Err(ApiError::bad_request("Invalid ApiQL default order"));
            }
            let field_config = self.field_config(field).ok_or_else(|| {
                ApiError::bad_request(format!("Unknown ApiQL order field: {field}"))
            })?;
            if !field_config.sortable {
                return Err(ApiError::bad_request(format!("ApiQL field is not sortable: {field}")));
            }
            parsed.push(ApiQlOrderBy { field: field_config.name.clone(), direction });
        }
        Ok(parsed)
    }

    fn field_config(&self, field_name: &str) -> Option<&'a ApiQlFieldConfig> {
        let normalized = field_name.trim().replace('.', "_");
        let camel = to_camel_case(&normalized);
        self.config.fields.iter().find(|field| {
            field.name == field_name
                || field.name == normalized
                || field.name == camel
                || to_snake_case(&field.name) == normalized
                || field.column.as_deref() == Some(field_name)
                || field.column.as_deref() == Some(normalized.as_str())
        })
    }

    fn virtual_field_config(&self, field_name: &str) -> Option<&'a ApiQlFieldConfig> {
        let normalized = field_name.trim().replace('.', "_");
        let camel = to_camel_case(&normalized);
        self.config.virtual_fields.iter().find(|field| {
            field.name == field_name
                || field.name == normalized
                || field.name == camel
                || to_snake_case(&field.name) == normalized
        })
    }

    fn collect_virtual_field_dependencies(
        &self,
        field_name: &str,
        dependencies: &mut Vec<&'a str>,
        visiting: &mut Vec<&'a str>,
    ) -> Result<(), ApiError> {
        let virtual_field = self.virtual_field_config(field_name).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL virtual field: {field_name}"))
        })?;
        self.collect_virtual_field_config_dependencies(
            virtual_field,
            field_name,
            dependencies,
            visiting,
        )
    }

    fn collect_virtual_field_config_dependencies(
        &self,
        virtual_field: &'a ApiQlFieldConfig,
        field_name: &str,
        dependencies: &mut Vec<&'a str>,
        visiting: &mut Vec<&'a str>,
    ) -> Result<(), ApiError> {
        if visiting.contains(&virtual_field.name.as_str()) {
            return Err(ApiError::bad_request(format!(
                "Circular ApiQL virtual field dependency: {field_name}"
            )));
        }
        visiting.push(virtual_field.name.as_str());

        for dependency in virtual_field_dependencies(virtual_field) {
            if self.field_config(dependency).is_some() {
                if !dependencies.contains(&dependency) {
                    dependencies.push(dependency);
                }
                continue;
            }
            if let Some(virtual_dependency) = self.virtual_field_config_by_name(dependency) {
                self.collect_virtual_field_config_dependencies(
                    virtual_dependency,
                    dependency,
                    dependencies,
                    visiting,
                )?;
                continue;
            }
            return Err(ApiError::bad_request(format!(
                "Unknown ApiQL virtual field dependency: {dependency}"
            )));
        }

        visiting.pop();
        Ok(())
    }

    fn virtual_field_config_by_name(&self, field_name: &str) -> Option<&'a ApiQlFieldConfig> {
        let normalized = field_name.trim().replace('.', "_");
        let camel = to_camel_case(&normalized);
        self.config.virtual_fields.iter().find(|field| {
            field.name == field_name
                || field.name == normalized
                || field.name == camel
                || to_snake_case(&field.name) == normalized
        })
    }
}

pub(crate) async fn load_entity_configs(
    tenant: TenantScope,
) -> Result<Vec<ApiQlEntityConfig>, ApiError> {
    crate::global_database::run_isolated(move || local_entity_config_store(tenant)?.load_configs())
        .await
        .map_err(internal_storage_error)?
}

#[cfg(not(test))]
pub(crate) async fn load_entity_registry(
    tenant: TenantScope,
) -> Result<Arc<ApiQlEntityRegistry>, ApiError> {
    if let Some(cached) = APIQL_ENTITY_REGISTRY_CACHE.lock().clone()
        && cached.tenant == tenant
        && cached.loaded_at.elapsed() <= APIQL_ENTITY_REGISTRY_CACHE_TTL
    {
        return Ok(cached.registry);
    }

    let registry =
        Arc::new(ApiQlEntityRegistry::from_configs(load_entity_configs(tenant.clone()).await?)?);
    *APIQL_ENTITY_REGISTRY_CACHE.lock() = Some(ApiQlEntityRegistryCache {
        tenant,
        loaded_at: Instant::now(),
        registry: registry.clone(),
    });
    Ok(registry)
}

#[cfg(test)]
pub(crate) async fn load_entity_registry(
    tenant: TenantScope,
) -> Result<ApiQlEntityRegistry, ApiError> {
    ApiQlEntityRegistry::from_configs(load_entity_configs(tenant).await?)
}

pub(crate) async fn save_entity_configs(
    tenant: TenantScope,
    configs: &[ApiQlEntityConfig],
) -> Result<(), ApiError> {
    let configs = configs.to_vec();
    crate::global_database::run_isolated(move || {
        local_entity_config_store(tenant)?.save_configs(&configs)
    })
    .await
    .map_err(internal_storage_error)??;
    #[cfg(not(test))]
    {
        *APIQL_ENTITY_REGISTRY_CACHE.lock() = None;
    }
    Ok(())
}

fn validate_entity_config(config: &ApiQlEntityConfig) -> Result<(), ApiError> {
    if config.connection_id.trim().is_empty() {
        return Err(ApiError::bad_request(format!(
            "ApiQL entity has empty connection_id: {}",
            config.entity_class
        )));
    }
    if config.table.trim().is_empty() {
        return Err(ApiError::bad_request(format!(
            "ApiQL entity has empty table: {}",
            config.entity_class
        )));
    }
    for field in &config.fields {
        validate_field_name(&field.name)?;
    }
    for field_name in &config.search_fields {
        if !config.fields.iter().any(|field| field.name == *field_name) {
            return Err(ApiError::bad_request(format!("Unknown ApiQL search field: {field_name}")));
        }
    }
    for field in &config.virtual_fields {
        validate_field_name(&field.name)?;
    }
    Ok(())
}

pub(crate) fn local_entity_config_store(
    tenant: TenantScope,
) -> Result<ApiQlEntityConfigStore, ApiError> {
    let database =
        GlobalDatabase::for_home(&crate::global::paths().home).map_err(internal_storage_error)?;
    ApiQlEntityConfigStore::new(database, tenant)
}

fn internal_storage_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

fn validate_entity_key(entity_key: &str) -> Result<(), ApiError> {
    let mut parts = entity_key.split(':');
    let Some(namespace) = parts.next() else {
        return Err(ApiError::bad_request("Invalid ApiQL entity key"));
    };
    let Some(name) = parts.next() else {
        return Err(ApiError::bad_request("Invalid ApiQL entity key"));
    };
    if parts.next().is_some() || !is_entity_key_segment(namespace) || !is_entity_key_segment(name) {
        return Err(ApiError::bad_request(format!("Invalid ApiQL entity key: {entity_key}")));
    }
    Ok(())
}

fn validate_field_name(field_name: &str) -> Result<(), ApiError> {
    if field_name.is_empty()
        || !field_name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(ApiError::bad_request(format!("Invalid ApiQL field name: {field_name}")));
    }
    Ok(())
}

fn is_entity_key_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn virtual_field_dependencies(field: &ApiQlFieldConfig) -> Vec<&str> {
    field
        .column
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|dependency| !dependency.is_empty())
        .collect()
}

fn to_snake_case(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    for (index, ch) in chars.iter().copied().enumerate() {
        if matches!(ch, '-' | ' ') {
            push_separator(&mut output);
            continue;
        }
        if ch == '_' {
            push_separator(&mut output);
            continue;
        }
        if ch.is_ascii_uppercase() {
            let previous = index.checked_sub(1).and_then(|previous| chars.get(previous)).copied();
            let next = chars.get(index + 1).copied();
            let needs_separator = previous.is_some_and(|previous| {
                previous != '_'
                    && (previous.is_ascii_lowercase()
                        || previous.is_ascii_digit()
                        || next.is_some_and(|next| next.is_ascii_lowercase()))
            });
            if needs_separator {
                push_separator(&mut output);
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

fn to_camel_case(value: &str) -> String {
    if !value.contains('_') && !value.contains('-') && !value.contains(' ') {
        return value.to_string();
    }
    let mut output = String::with_capacity(value.len());
    let mut upper_next = false;
    for ch in value.chars() {
        if matches!(ch, '_' | '-' | ' ') {
            upper_next = !output.is_empty();
            continue;
        }
        if output.is_empty() {
            output.push(ch.to_ascii_lowercase());
        } else if upper_next {
            output.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn push_separator(output: &mut String) {
    if !output.is_empty() && !output.ends_with('_') {
        output.push('_');
    }
}
