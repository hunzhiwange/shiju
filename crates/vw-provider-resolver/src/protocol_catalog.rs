//! Independently loaded, dated exact-model protocol evidence.
use super::Provider;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use vw_shared::provider::protocols::{ProtocolEndpoint, ProtocolModelScope, deserialize_protocols};

#[derive(Deserialize)]
struct Catalog {
    providers: HashMap<String, Supplement>,
}

#[derive(Deserialize)]
struct Supplement {
    default_api: Option<String>,
    deployments: HashMap<String, Deployment>,
}

#[derive(Deserialize)]
struct Deployment {
    #[serde(deserialize_with = "deserialize_protocols")]
    protocols: Option<Vec<ProtocolEndpoint>>,
}

static CATALOG: Lazy<Catalog> = Lazy::new(|| {
    serde_json::from_str(include_str!("../assets/model/protocols.json"))
        .expect("bundled protocol facts must satisfy the shared endpoint contract")
});

pub(super) fn merge(providers: &mut HashMap<String, Provider>) {
    for (id, provider) in providers {
        let Some(supplement) = CATALOG.providers.get(id) else { continue };
        if provider.api.is_none() {
            provider.api.clone_from(&supplement.default_api);
        }
        for model in provider.models.values_mut() {
            if model.protocols.is_some()
                || model.provider.as_ref().is_some_and(|value| value.protocols.is_some())
                || provider.protocols.is_some()
            {
                continue;
            }
            let Some(base) = model
                .provider
                .as_ref()
                .and_then(|value| value.api.as_deref())
                .or(provider.api.as_deref())
            else {
                continue;
            };
            let Some(deployment) = supplement.deployments.get(base.trim_end_matches('/')) else {
                continue;
            };
            if let Some(endpoints) = &deployment.protocols {
                let candidates: Vec<_> = endpoints
                    .iter()
                    .filter(|endpoint| endpoint.models.contains(&model.id))
                    .map(|endpoint| ProtocolEndpoint {
                        protocol: endpoint.protocol,
                        url: endpoint.url.clone(),
                        deployment: endpoint.deployment.clone(),
                        authentication: endpoint.authentication.clone(),
                        // Avoid copying the whole provider model scope into each model.
                        models: ProtocolModelScope::Models { ids: vec![model.id.clone()] },
                        evidence: endpoint.evidence.clone(),
                    })
                    .collect();
                if !candidates.is_empty() {
                    model.protocols = Some(candidates);
                }
            }
        }
    }
}
