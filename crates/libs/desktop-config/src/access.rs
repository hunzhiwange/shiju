use vw_config_types::{config::Config, security::IdentityConfig};
#[cfg(not(target_arch="wasm32"))]
use std::sync::OnceLock;
fn normalize_identity_format(raw: Option<&str>) -> String {
    let _ = raw;
    "openclaw".to_string()
}

/// 将 `main` 代理的显式配置覆盖到桌面端默认模型与工具选择。
pub fn apply_main_agent_overrides(config: &mut Config) {
    let Some(main) = config.agents.get("main") else {
        return;
    };

    let provider = main.provider.trim();
    if !provider.is_empty() {
        config.default_provider = Some(provider.to_string());
    }

    let model = main.model.trim();
    if !provider.is_empty() && !model.is_empty() {
        config.default_model = Some(format!("{provider}/{model}"));
    }

    if let Some(temperature) = main.temperature {
        config.default_temperature = temperature;
    }

    if main.identity_format.is_some() {
        config.identity = IdentityConfig {
            format: normalize_identity_format(main.identity_format.as_deref()),
            aieos_path: None,
            aieos_inline: None,
        };
    }
}

/// 在桌面配置 JSON 中创建缺失的对象层级并写入叶节点。
pub fn set_config_value_at_path(
    root: &mut serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) {
    if path.is_empty() {
        *root = value;
        return;
    }

    if !root.is_object() {
        *root = serde_json::json!({});
    }

    let mut current = root;
    for key in &path[..path.len() - 1] {
        if current.get(*key).and_then(|v| v.as_object()).is_none()
            && let Some(obj) = current.as_object_mut()
        {
            obj.insert((*key).to_string(), serde_json::json!({}));
        }
        current =
            current.as_object_mut().and_then(|obj| obj.get_mut(*key)).expect("path object exists");
    }

    if let Some(obj) = current.as_object_mut() {
        obj.insert(path[path.len() - 1].to_string(), value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_blocking_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

    match RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .map_err(|err| err.to_string())
    }) {
        Ok(runtime) => Ok(runtime),
        Err(err) => Err(err.clone()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_gateway_call<T: Send>(
    future: impl std::future::Future<Output = Result<T, String>> + Send,
) -> Result<T, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(|| gateway_blocking_runtime()?.block_on(future))
                .join()
                .unwrap_or_else(|_| Err("gateway blocking thread panicked".to_string()))
        }),
        Err(_) => gateway_blocking_runtime()?.block_on(future),
    }
}

#[cfg(target_arch = "wasm32")]
/// WASM keeps this shim only for legacy synchronous load sites and returns
/// `Default` immediately; synchronous save flows now run through
/// `spawn_gateway_task` and the async gateway APIs.
pub fn run_gateway_call<T: Default>(
    _future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tracing::warn!(target: "vw_desktop", "sync gateway call on wasm, returning default");
    Ok(T::default())
}


#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "access_tests.rs"]
mod tests;
