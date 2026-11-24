//! 将数据目录与当前用户的绑定按公司实例合并。
use super::super::{ApiEnvelope, business_error_status};
use super::{REQUEST_FAILED, ShijuAccountClient, ShijuAccountError, endpoint};
use gateway_shiju_connector::{
    ShijuConnector, ShijuConnectorCatalogEntry, ShijuConnectorCatalogScope,
    ShijuConnectorCatalogStatus as Status, deserialize_connector_id,
};
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};

const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 20;

#[derive(Deserialize)]
struct CatalogPage<T> {
    list: Option<Vec<T>>,
    total: Option<usize>,
}

#[derive(Deserialize)]
struct Platform {
    #[serde(deserialize_with = "deserialize_connector_id")]
    platform_id: u64,
    platform_name: String,
    platform_code: Option<String>,
    description: Option<String>,
    status: Option<String>,
    account_binding_mode: Option<String>,
}

#[derive(Deserialize)]
struct CompanyPlatform {
    #[serde(deserialize_with = "deserialize_connector_id")]
    platform_id: u64,
    #[serde(deserialize_with = "deserialize_connector_id")]
    company_platform_id: u64,
    company_platform_name: Option<String>,
    company_platform_code: Option<String>,
    status: Option<String>,
}

impl ShijuAccountClient {
    pub async fn connector_catalog(
        &self,
        credential: &str,
        scope: ShijuConnectorCatalogScope,
    ) -> Result<Vec<ShijuConnectorCatalogEntry>, ShijuAccountError> {
        let filters: &[(&str, &str)] = match scope {
            ShijuConnectorCatalogScope::System => {
                &[("platform_scope", "SYSTEM"), ("status", "enabled")]
            }
            ShijuConnectorCatalogScope::Custom => &[("platform_scope", "COMPANY")],
        };
        let platforms: Vec<Platform> = self.catalog_pages(credential, "platforms", filters).await?;
        if platforms.is_empty() {
            return Ok(Vec::new());
        }
        let personal = self.connectors(credential).await?;
        let instances: Vec<CompanyPlatform> =
            match self.catalog_pages(credential, "company-platforms", &[]).await {
                Ok(instances) => instances,
                // 非管理员仍可从个人目录获取其有权绑定的实例。
                Err(error) if error.status() == Some(StatusCode::FORBIDDEN) => Vec::new(),
                Err(error) => return Err(error),
            };
        Ok(catalog_entries(platforms, instances, personal, scope))
    }

    async fn catalog_pages<T: DeserializeOwned>(
        &self,
        credential: &str,
        resource: &str,
        filters: &[(&str, &str)],
    ) -> Result<Vec<T>, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        let url = endpoint(&self.gateway_base, &format!("data/api/v1/{resource}"))?;
        let mut rows = Vec::new();
        for page in 1..=MAX_PAGES {
            let response = self
                .http
                .get(url.clone())
                .query(filters)
                .query(&[("page", page), ("page_size", PAGE_SIZE)])
                .header("X-Api-Key", credential)
                .send()
                .await
                .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
            if !response.status().is_success() {
                return Err(catalog_error(response.status()));
            }
            let envelope = response
                .json::<ApiEnvelope<serde_json::Value>>()
                .await
                .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
            if envelope.code != 200 {
                return Err(catalog_error(business_error_status(envelope.code)));
            }
            let data: CatalogPage<T> = serde_json::from_value(
                envelope.data.ok_or(ShijuAccountError::Message(REQUEST_FAILED))?,
            )
            .map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
            let list = data.list.unwrap_or_default();
            let count = list.len();
            rows.extend(list);
            if count == 0 || data.total.map_or(count < PAGE_SIZE, |total| rows.len() >= total) {
                return Ok(rows);
            }
        }
        Err(ShijuAccountError::Message("SHIJU connector catalog exceeded its page limit"))
    }
}

fn catalog_error(status: StatusCode) -> ShijuAccountError {
    ShijuAccountError::Upstream { status, message: REQUEST_FAILED.to_string() }
}

fn catalog_entries(
    platforms: Vec<Platform>,
    instances: Vec<CompanyPlatform>,
    personal: Vec<ShijuConnector>,
    scope: ShijuConnectorCatalogScope,
) -> Vec<ShijuConnectorCatalogEntry> {
    let mut known_instances = BTreeMap::<u64, BTreeMap<u64, CompanyPlatform>>::new();
    for instance in instances {
        known_instances
            .entry(instance.platform_id)
            .or_default()
            .insert(instance.company_platform_id, instance);
    }
    let mut accounts = BTreeMap::<u64, BTreeMap<u64, ShijuConnector>>::new();
    for account in personal {
        accounts
            .entry(account.platform_id)
            .or_default()
            .insert(account.company_platform_id, account);
    }

    let mut entries = Vec::new();
    for platform in platforms {
        let instances = known_instances.remove(&platform.platform_id).unwrap_or_default();
        let mut accounts = accounts.remove(&platform.platform_id).unwrap_or_default();
        let ids: BTreeSet<_> = instances.keys().chain(accounts.keys()).copied().collect();
        if ids.is_empty() {
            entries.push(catalog_entry(&platform, None, None));
            continue;
        }
        let rows = ids
            .into_iter()
            .map(|id| catalog_entry(&platform, instances.get(&id), accounts.remove(&id)));
        match scope {
            ShijuConnectorCatalogScope::System => entries.push(
                rows.min_by_key(|entry| match entry.status {
                    Status::Bindable
                        if entry
                            .account
                            .as_ref()
                            .and_then(|a| a.binding.as_ref())
                            .is_some_and(|binding| binding.status == "enabled") =>
                    {
                        0
                    }
                    Status::Bindable => 1,
                    Status::NoBinding => 2,
                    Status::NoAccess => 3,
                    Status::Disabled => 4,
                    Status::Unavailable => 5,
                })
                .unwrap_or_else(|| catalog_entry(&platform, None, None)),
            ),
            ShijuConnectorCatalogScope::Custom => entries.extend(rows),
        }
    }
    entries
}

fn catalog_entry(
    platform: &Platform,
    instance: Option<&CompanyPlatform>,
    account: Option<ShijuConnector>,
) -> ShijuConnectorCatalogEntry {
    let status = if platform.status.as_deref() == Some("disabled")
        || instance.is_some_and(|row| row.status.as_deref() == Some("disabled"))
    {
        Status::Disabled
    } else if instance.is_none() && account.is_none() {
        Status::Unavailable
    } else if platform.account_binding_mode.as_deref() == Some("NONE") {
        Status::NoBinding
    } else if account.is_some() {
        Status::Bindable
    } else {
        Status::NoAccess
    };
    ShijuConnectorCatalogEntry {
        platform_id: platform.platform_id,
        name: instance
            .and_then(|row| row.company_platform_name.as_ref())
            .filter(|name| !name.is_empty())
            .or_else(|| account.as_ref().map(|row| &row.platform_name))
            .unwrap_or(&platform.platform_name)
            .clone(),
        code: instance
            .and_then(|row| row.company_platform_code.as_ref())
            .filter(|code| !code.is_empty())
            .or(platform.platform_code.as_ref())
            .cloned()
            .unwrap_or_default(),
        description: platform.description.clone().unwrap_or_default(),
        status,
        account: account.filter(|_| status == Status::Bindable),
    }
}
