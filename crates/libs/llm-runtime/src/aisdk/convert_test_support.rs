//! Shared model fixture for request conversion tests.

use model_types as provider;
use serde_json::json;

pub(super) fn test_model(
    provider_id: &str,
    adapter: &str,
    api_url: &str,
    model_id: &str,
    name: &str,
) -> provider::Model {
    serde_json::from_value(json!({
        "id": model_id,
        "providerID": provider_id,
        "api": {
            "id": model_id,
            "url": api_url,
            "adapter": adapter
        },
        "name": name,
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}
