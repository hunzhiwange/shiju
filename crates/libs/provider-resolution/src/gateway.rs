//! wasm 网关组装。
use crate::State;
use provider_config_reader as config;
use std::collections::HashMap;

/// 通过网关加载 provider 状态。
///
/// wasm 环境不直接访问本地配置与认证文件，因此改由网关提供聚合结果。
pub(crate) async fn load_state_impl(apply_enabled_filter: bool) -> State {
    let Ok(client) = config::gateway_client() else {
        return State { providers: HashMap::new() };
    };
    let Ok(response) = client.provider_list(None).await else {
        return State { providers: HashMap::new() };
    };

    let providers = response
        .all
        .into_iter()
        .filter_map(|mut provider| {
            if apply_enabled_filter {
                provider.models.retain(|_, model| model.status == "active");
            }
            if provider.models.is_empty() {
                return None;
            }
            Some((provider.id.clone(), provider))
        })
        .collect();

    State { providers }
}
