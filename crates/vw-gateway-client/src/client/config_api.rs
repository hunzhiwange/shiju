use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use vw_api_types::id::TenantScope;

use super::GatewayClient;
use crate::http::directory_query;

impl GatewayClient {
    /// 读取指定目录生效的配置视图。
    pub async fn config_get(&self, directory: Option<&str>) -> Result<Value, String> {
        self.get_json("/v1/config", &directory_query(directory)).await
    }

    /// 按补丁方式更新指定目录的配置，并返回更新后的配置。
    pub async fn config_patch(
        &self,
        directory: Option<&str>,
        patch: &Value,
    ) -> Result<Value, String> {
        self.patch_json("/v1/config", &directory_query(directory), patch).await
    }

    /// 读取指定目录可用的 provider 配置目录。
    pub async fn config_providers(&self, directory: Option<&str>) -> Result<Value, String> {
        self.get_json("/v1/config/providers", &directory_query(directory)).await
    }

    /// 读取 dashboard 使用的完整配置视图。
    pub async fn dashboard_config_get(&self) -> Result<Value, String> {
        self.get_json("/v1/dashboard/config", &[]).await
    }

    /// 替换 dashboard 使用的完整配置。
    pub async fn dashboard_config_put(&self, config: &Value) -> Result<Value, String> {
        self.put_json("/v1/dashboard/config", &[], config).await
    }

    /// 读取全局配置。
    pub async fn global_config_get(&self) -> Result<Value, String> {
        self.get_json("/v1/global/config", &[]).await
    }

    /// 按补丁方式更新全局配置。
    pub async fn global_config_patch(&self, patch: &Value) -> Result<(), String> {
        let _: Value = self.patch_json("/v1/global/config", &[], patch).await?;
        Ok(())
    }

    /// 读取显式平台和公司的租户基础配置。
    pub async fn tenant_global_config_get(&self, tenant: &TenantScope) -> Result<Value, String> {
        self.ensure_endpoint_tenant_scope(tenant)?;
        self.get_json("/v1/global/config", &[]).await
    }

    /// 按补丁方式更新显式平台和公司的租户基础配置。
    pub async fn tenant_global_config_patch(
        &self,
        tenant: &TenantScope,
        patch: &Value,
    ) -> Result<(), String> {
        self.ensure_endpoint_tenant_scope(tenant)?;
        let _: Value = self.patch_json("/v1/global/config", &[], patch).await?;
        Ok(())
    }

    /// 读取并反序列化 ACP 全局配置。
    pub async fn global_acp_config_get<T>(&self) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let value = self.get_json("/v1/global/config/acp", &[]).await?;
        serde_json::from_value(value).map_err(|err| err.to_string())
    }

    pub(crate) async fn global_config_get_path(
        &self,
        path: &[&str],
    ) -> Result<Option<Value>, String> {
        let mut current = self.global_config_get().await?;
        for key in path {
            let Some(next) = current.get(*key) else {
                return Ok(None);
            };
            current = next.clone();
        }
        Ok(Some(current))
    }

    pub(crate) async fn global_config_patch_path(
        &self,
        path: &[&str],
        value: Value,
    ) -> Result<(), String> {
        let mut patch = value;
        for key in path.iter().rev() {
            let mut object = Map::new();
            object.insert((*key).to_string(), patch);
            patch = Value::Object(object);
        }
        self.global_config_patch(&patch).await
    }

    fn ensure_endpoint_tenant_scope(&self, tenant: &TenantScope) -> Result<(), String> {
        if tenant != self.endpoint().tenant_scope() {
            return Err("tenant scope does not match gateway endpoint tenant scope".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "config_api_tests.rs"]
mod config_api_tests;
