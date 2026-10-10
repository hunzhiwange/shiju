//! 网关客户端请求辅助模块，封装同步 HTTP 请求、JSON 解析和统一错误映射。

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::GatewayClient;
#[cfg(not(target_arch = "wasm32"))]
use crate::endpoint::GatewayEndpoint;
use crate::http::{
    apply_request_context, log_request, log_request_succeeded, parse_json_response, response_error,
    transport_error,
};

impl GatewayClient {
    /// 提供 get json 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub(super) async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, String> {
        log_request::<()>("GET", &self.endpoint, path, query, None);
        let request = self.client.get(format!("{}{}", self.endpoint.base_url(), path)).query(query);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("GET", &self.endpoint, path, err))?;
        parse_json_response("GET", &self.endpoint, path, response).await
    }

    /// 提供 post json 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub(super) async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        log_request("POST", &self.endpoint, path, query, Some(body));
        let request = self
            .client
            .post(format!("{}{}", self.endpoint.base_url(), path))
            .query(query)
            .json(body);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("POST", &self.endpoint, path, err))?;
        parse_json_response("POST", &self.endpoint, path, response).await
    }

    /// 提供 patch json 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub(super) async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        log_request("PATCH", &self.endpoint, path, query, Some(body));
        let request = self
            .client
            .patch(format!("{}{}", self.endpoint.base_url(), path))
            .query(query)
            .json(body);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("PATCH", &self.endpoint, path, err))?;
        parse_json_response("PATCH", &self.endpoint, path, response).await
    }

    /// 提供 put json 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub(super) async fn put_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        log_request("PUT", &self.endpoint, path, query, Some(body));
        let request = self
            .client
            .put(format!("{}{}", self.endpoint.base_url(), path))
            .query(query)
            .json(body);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("PUT", &self.endpoint, path, err))?;
        parse_json_response("PUT", &self.endpoint, path, response).await
    }

    /// 删除 empty 数据。
    ///
    /// 删除失败会通过 I/O 错误返回给调用方，避免静默丢失状态。
    pub(super) async fn delete_empty(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<(), String> {
        log_request::<()>("DELETE", &self.endpoint, path, query, None);
        let request =
            self.client.delete(format!("{}{}", self.endpoint.base_url(), path)).query(query);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("DELETE", &self.endpoint, path, err))?;
        if !response.status().is_success() {
            return Err(response_error("DELETE", &self.endpoint, path, response).await);
        }
        log_request_succeeded("DELETE", &self.endpoint, path);
        Ok(())
    }

    /// 删除 json 数据。
    ///
    /// 删除失败会通过 I/O 错误返回给调用方，避免静默丢失状态。
    pub(super) async fn delete_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        log_request("DELETE", &self.endpoint, path, query, Some(body));
        let request = self
            .client
            .delete(format!("{}{}", self.endpoint.base_url(), path))
            .query(query)
            .json(body);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("DELETE", &self.endpoint, path, err))?;
        parse_json_response("DELETE", &self.endpoint, path, response).await
    }

    /// 提供 get json with 404 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub(super) async fn get_json_with_404<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Option<T>, String> {
        log_request::<()>("GET", &self.endpoint, path, query, None);
        let request = self.client.get(format!("{}{}", self.endpoint.base_url(), path)).query(query);
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| transport_error("GET", &self.endpoint, path, err))?;

        if response.status().is_success() {
            response.json::<Option<T>>().await.map_err(|err| {
                let msg = err.to_string();
                tracing::error!(
                    target: "vw_gateway_client",
                    method = "GET",
                    endpoint = %self.endpoint.describe(),
                    path = path,
                    error = %msg,
                    "gateway response JSON decode failed"
                );
                msg
            })
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Err(response_error("GET", &self.endpoint, path, response).await)
        }
    }
}

/// 提供 get json blocking 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn get_json_blocking<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    path: &str,
    query: &[(String, String)],
) -> Result<T, String> {
    use crate::http::{apply_blocking_request_context, response_error_blocking};

    log_request::<()>("GET", endpoint, path, query, None);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(crate::http::REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|err| transport_error("GET", endpoint, path, err))?;
    let request = client.get(format!("{}{}", endpoint.base_url(), path)).query(query);
    let response = apply_blocking_request_context(request, endpoint)
        .send()
        .map_err(|err| transport_error("GET", endpoint, path, err))?;
    if !response.status().is_success() {
        return Err(response_error_blocking("GET", endpoint, path, response));
    }
    log_request_succeeded("GET", endpoint, path);
    response.json().map_err(|err| {
        let msg = err.to_string();
        tracing::error!(
            target: "vw_gateway_client",
            method = "GET",
            endpoint = %endpoint.describe(),
            path = path,
            error = %msg,
            "gateway response JSON decode failed"
        );
        msg
    })
}

#[cfg(test)]
#[path = "request_helpers_tests.rs"]
mod request_helpers_tests;
