//! Exhaustive source identities, independently selected through the production catalog.
use crate::{models, protocol_routes::*};
use serde_json::{Value, json};
use vw_shared::provider::protocols::{ModelProtocol as P, ProtocolAuthentication as A};

const PROTOCOLS: [P; 4] = [P::Chat, P::Messages, P::Responses, P::Gemini];

fn matrix() -> Value {
    let inventory: Value = serde_json::from_str(include_str!(
        "../../../docs/dev/model-protocols/catalog-inventory.json"
    ))
    .unwrap();
    let catalog = models::bundled_models("offline_matrix");
    let research: Vec<Value> = [
        include_str!("../../../docs/dev/model-protocols/research/group-01.json"),
        include_str!("../../../docs/dev/model-protocols/research/group-02.json"),
        include_str!("../../../docs/dev/model-protocols/research/group-03.json"),
        include_str!("../../../docs/dev/model-protocols/research/group-04.json"),
        include_str!("../../../docs/dev/model-protocols/research/group-05.json"),
        include_str!("../../../docs/dev/model-protocols/research/group-06.json"),
    ]
    .into_iter()
    .map(|text| serde_json::from_str(text).unwrap())
    .collect();
    let assessments: std::collections::HashMap<_, _> = research
        .iter()
        .flat_map(|report| report["records"].as_array().unwrap())
        .map(|r| {
            (
                (
                    r["source"].as_str().unwrap(),
                    r["provider_id"].as_str().unwrap(),
                    r["model_key"].as_str().unwrap(),
                ),
                r,
            )
        })
        .collect();
    let mut records = Vec::new();
    for row in inventory["records"].as_array().unwrap() {
        let provider_id = row["provider_id"].as_str().unwrap();
        let key = row["model_key"].as_str().unwrap();
        let assessment = assessments[&(row["source"].as_str().unwrap(), provider_id, key)];
        let model = catalog.get(provider_id).and_then(|p| p.models.get(key));
        let endpoints = model.and_then(|m| {
            m.protocols
                .as_deref()
                .or_else(|| m.provider.as_ref().and_then(|p| p.protocols.as_deref()))
                .or_else(|| catalog[provider_id].protocols.as_deref())
        });
        let applicable = row["runtime_catalog_state"] == "active"
            && row["text_acp_applicability"] == "text_candidate_requires_protocol_review"
            && !matches!(
                assessment["conclusion"].as_str(),
                Some("不适用于文本ACP" | "明确不支持" | "实现缺口")
            );
        let mut outcomes = serde_json::Map::new();
        for source in PROTOCOLS {
            let mut attempts = Vec::new();
            let deployments: std::collections::BTreeSet<_> =
                endpoints.into_iter().flatten().map(|e| e.deployment.as_str()).collect();
            for deployment in deployments {
                let binding = RouteBinding {
                    provider: provider_id.into(),
                    model: model.unwrap().id.clone(),
                    deployment: deployment.into(),
                    account: "offline-selected-account".into(),
                };
                let caps = [RouteCapability::Text];
                let grants: Vec<_> = endpoints
                    .unwrap()
                    .iter()
                    .filter(|e| match &e.authentication {
                        A::None {} | A::Bearer {} | A::OAuthBearer {} => true,
                        A::ApiKeyHeader { name } => matches!(
                            name.to_ascii_lowercase().as_str(),
                            "x-api-key" | "x-goog-api-key" | "api-key"
                        ),
                        _ => false,
                    })
                    .map(|endpoint| EndpointGrant {
                        binding: &binding,
                        endpoint,
                        capabilities: &caps,
                    })
                    .collect();
                // Basic-text directions implemented by the relay. HTTP/JSON/SSE and
                // richer capability contracts are exercised by the agent evidence suite.
                let capabilities = &caps[..];
                let conversions: Vec<_> = PROTOCOLS
                    .into_iter()
                    .flat_map(|source| {
                        PROTOCOLS.into_iter().filter_map(move |target| {
                            (source != target).then_some(ConversionSupport {
                                source,
                                target,
                                capabilities,
                            })
                        })
                    })
                    .collect();
                let result = select_route(&RouteRequest {
                    binding: &binding,
                    source,
                    required: &caps,
                    endpoints,
                    grants: if applicable { &grants } else { &[] },
                    conversions: &conversions,
                });
                attempts.push(match result {
                    Ok(route) => {
                        assert_eq!(route.binding, binding);
                        assert_eq!(route.source, source);
                        assert_eq!(route.conversion_required, source != route.target);
                        json!({"deployment":deployment,"status":if route.conversion_required {"conversion"} else {"native"},
                            "target":route.target,"endpoint":route.endpoint.url})
                    }
                    Err(error) => json!({"deployment":deployment,"status":"rejected","reasons":error.reasons.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>()}),
                });
            }
            if attempts.is_empty() {
                attempts.push(json!({"status":"rejected","reasons":[if applicable {"UnknownProtocols"} else {"NotApplicableOrNotLoaded"}]}));
            }
            outcomes.insert(
                serde_json::to_value(source).unwrap().as_str().unwrap().into(),
                json!(attempts),
            );
        }
        records.push(json!({"source":row["source"],"provider_id":row["provider_id"],"model_key":row["model_key"],
            "model_api_id":row["model_api_id"],"canonical_model_id":row["canonical_model_id"],
            "source_declarations":row["effective_declarations"],"capabilities":row["capabilities"],
            "applicability":row["text_acp_applicability"],"runtime_catalog_state":row["runtime_catalog_state"],
            "research_conclusion":assessment["conclusion"],"research_reason":assessment["reason"],
            "research_evidence_ids":assessment["evidence_ids"],
            "candidate_endpoints":endpoints,"protocols":outcomes,"evidence_type":"offline_selector"}));
    }
    json!({"schema_version":1,"checked_at":"2026-11-12","source_hashes":inventory["sources"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),v["sha256"].clone())).collect::<serde_json::Map<_,_>>(),
        "scope":"Basic text selection under explicit offline grants; no credentials or external service calls. Unknown facts reject. Capability and wire evidence is separate.","records":records})
}

#[test]
fn model_protocol_020_every_source_row_matches_four_protocol_selection() {
    let actual = matrix();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/dev/model-protocols/offline-matrix.json");
    if std::env::var_os("SHIJU_UPDATE_PROTOCOL_MATRIX").is_some() {
        let mut header = actual.as_object().unwrap().clone();
        let records = header.remove("records").unwrap();
        let mut text = serde_json::to_string_pretty(&header).unwrap();
        text.truncate(text.len() - 2);
        text.push_str(",\n  \"records\": [\n");
        text.push_str(
            &records
                .as_array()
                .unwrap()
                .iter()
                .map(|r| format!("    {}", serde_json::to_string(r).unwrap()))
                .collect::<Vec<_>>()
                .join(",\n"),
        );
        text.push_str("\n  ]\n}\n");
        std::fs::write(&path, text).unwrap();
    }
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|_| "{}".into()))
            .unwrap();
    assert_eq!(actual["records"].as_array().unwrap().len(), 8949);
    assert!(actual == expected, "regenerate the reviewed matrix after catalog/selector changes");
    eprintln!("source_rows=8949 protocol_cells=35796; exact bindings and outcomes verified");
}
