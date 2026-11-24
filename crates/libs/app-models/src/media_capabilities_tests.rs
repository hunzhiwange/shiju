use std::collections::HashMap;

use provider_model_catalog::Provider;
use serde_json::json;

use super::media_capabilities::MediaCapability::*;
use super::media_capabilities::{
    MediaCapability, MediaDeployment, MediaDeploymentEndpoints, MediaEndpointGrant, MediaModelBinding,
    project_media_catalog,
};

const HOSTED: MediaModelBinding<'static> = MediaModelBinding {
    provider: "shiju",
    deployment: "subscription",
    model: "shared",
    credential_binding: "tenant-subscription",
};
const DIRECT: MediaModelBinding<'static> = MediaModelBinding {
    provider: "vendor",
    deployment: "direct",
    model: "shared",
    credential_binding: "tenant-byok",
};
const CUSTOM: MediaModelBinding<'static> = MediaModelBinding {
    provider: "custom",
    deployment: "custom-eu",
    model: "shared",
    credential_binding: "tenant-custom",
};

fn fixture() -> HashMap<String, Provider> {
    serde_json::from_value(json!({
        "vendor": {"models": {
            "shared": {"modalities": {"input": ["text"], "output": ["audio"]}}
        }},
        "shiju": {"models": {
            "shared": {"canonical_model_id": "vendor/shared"}
        }},
        "custom": {"models": {
            "unknown": {},
            "empty": {"modalities": {"input": [], "output": []}},
            "override": {
                "canonical_model_id": "vendor/shared",
                "modalities": {"input": ["text"], "output": ["text"]}
            },
            "image": {"modalities": {"input": ["text", "image"], "output": ["image"]}},
            "video": {"modalities": {"input": ["text", "image"], "output": ["video"]}},
            "vision": {"modalities": {"input": ["text", "image"], "output": ["text"]}},
            "native": {"modalities": {"input": ["text", "video"], "output": ["text"]}},
            "asr": {"modalities": {"input": ["audio"], "output": ["text"]}}
        }}
    }))
    .unwrap()
}

fn grant<'a>(
    binding: MediaModelBinding<'a>,
    endpoint_type: &'a str,
    endpoint_id: &'a str,
    capabilities: &'a [MediaCapability],
) -> MediaEndpointGrant<'a> {
    MediaEndpointGrant { binding, endpoint_type, endpoint_id, capabilities }
}

#[test]
fn media_catalog_keeps_same_named_hosted_and_direct_bindings() {
    let catalog = fixture();
    let hosted_grant = grant(HOSTED, "audio-speech", "router-speech", &[TextToSpeech]);
    let direct_grant = grant(DIRECT, "audio-speech", "vendor-speech", &[TextToSpeech]);
    let advertised = ["audio-speech".to_owned()];
    let hosted = MediaDeployment {
        binding: HOSTED,
        endpoints: MediaDeploymentEndpoints::Managed {
            supported_endpoint_types: Some(&advertised),
        },
        grants: &[hosted_grant],
    };
    let direct = MediaDeployment {
        binding: DIRECT,
        endpoints: MediaDeploymentEndpoints::Direct,
        grants: &[direct_grant],
    };

    let routes = project_media_catalog(&catalog, &[hosted, direct], TextToSpeech);
    assert_eq!(routes.len(), 2);
    assert_eq!((routes[0].binding, routes[0].endpoint_id), (HOSTED, "router-speech"));
    assert_eq!((routes[1].binding, routes[1].endpoint_id), (DIRECT, "vendor-speech"));

    // A grant attached to the other deployment cannot authorize the hosted model.
    let mixed = MediaDeployment { grants: &[direct_grant], ..hosted };
    let routes = project_media_catalog(&catalog, &[mixed, direct], TextToSpeech);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].binding, DIRECT);

    let different_credential = MediaModelBinding { credential_binding: "other", ..DIRECT };
    let mismatched = MediaDeployment { binding: different_credential, ..direct };
    assert!(project_media_catalog(&catalog, &[mismatched], TextToSpeech).is_empty());

    let ungranted = MediaDeployment { grants: &[], ..direct };
    let routes = project_media_catalog(&catalog, &[hosted, ungranted], TextToSpeech);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].binding, HOSTED);
}

#[test]
fn media_catalog_needs_operation_support_on_the_advertised_protocol() {
    let catalog = fixture();
    let chat = ["openai".to_owned()];
    let speech = ["audio-speech".to_owned()];
    let grants = [
        grant(HOSTED, "openai", "router-chat", &[]),
        grant(HOSTED, "audio-speech", "router-speech", &[TextToSpeech]),
    ];

    for supported_endpoint_types in [None, Some(&[][..]), Some(chat.as_slice())] {
        let hosted = MediaDeployment {
            binding: HOSTED,
            endpoints: MediaDeploymentEndpoints::Managed { supported_endpoint_types },
            grants: &grants,
        };
        assert!(project_media_catalog(&catalog, &[hosted], TextToSpeech).is_empty());
    }

    let hosted = MediaDeployment {
        binding: HOSTED,
        endpoints: MediaDeploymentEndpoints::Managed { supported_endpoint_types: Some(&speech) },
        grants: &grants[..1],
    };
    assert!(project_media_catalog(&catalog, &[hosted], TextToSpeech).is_empty());

    let direct_grants = [grant(DIRECT, "openai", "vendor-chat", &[])];
    let direct = MediaDeployment {
        binding: DIRECT,
        endpoints: MediaDeploymentEndpoints::Direct,
        grants: &direct_grants,
    };
    assert!(project_media_catalog(&catalog, &[direct], TextToSpeech).is_empty());
}

#[test]
fn media_catalog_preserves_unknown_and_explicitly_unsupported_modalities() {
    let catalog = fixture();
    for (model, expected) in [
        ("unknown", true),
        ("unlisted", true),
        ("empty", false),
        ("override", false),
    ] {
        let binding = MediaModelBinding { model, ..CUSTOM };
        let grants = [grant(binding, "custom-speech", "custom-speech", &[TextToSpeech])];
        let deployment = MediaDeployment {
            binding,
            endpoints: MediaDeploymentEndpoints::Direct,
            grants: &grants,
        };
        assert_eq!(
            !project_media_catalog(&catalog, &[deployment], TextToSpeech).is_empty(),
            expected,
            "{model}"
        );
        let ungranted = MediaDeployment { grants: &[], ..deployment };
        assert!(project_media_catalog(&catalog, &[ungranted], TextToSpeech).is_empty(), "{model}");
    }
}

#[test]
fn media_catalog_checks_each_operation_and_keeps_execution_paths_distinct() {
    let catalog = fixture();
    let cases = [
        ("image", ImageGeneration, ImageEdit, true),
        ("image", ImageEdit, ImageGeneration, true),
        ("image", ImageUnderstanding, ImageUnderstanding, false),
        ("image", VideoGeneration, VideoGeneration, false),
        ("video", VideoGeneration, ImageToVideo, true),
        ("video", ImageToVideo, VideoGeneration, true),
        ("vision", ImageUnderstanding, VideoUnderstandingFrames, true),
        ("vision", VideoUnderstandingFrames, VideoUnderstandingNative, true),
        ("vision", VideoUnderstandingNative, VideoUnderstandingNative, false),
        ("native", VideoUnderstandingNative, VideoUnderstandingFrames, true),
        ("native", VideoUnderstandingFrames, VideoUnderstandingFrames, false),
        ("asr", FileAsr, RealtimeAsr, true),
        ("asr", RealtimeAsr, FileAsr, true),
        ("asr", TextToSpeech, TextToSpeech, false),
    ];
    for (model, supported, unavailable, expected) in cases {
        let binding = MediaModelBinding { model, ..CUSTOM };
        let capabilities = [supported];
        let grants = [grant(binding, "configured", "custom-endpoint", &capabilities)];
        let deployment = MediaDeployment {
            binding,
            endpoints: MediaDeploymentEndpoints::Direct,
            grants: &grants,
        };

        assert_eq!(
            !project_media_catalog(&catalog, &[deployment], supported).is_empty(),
            expected,
            "{model}/{supported:?}"
        );
        assert!(
            project_media_catalog(&catalog, &[deployment], unavailable).is_empty(),
            "{model}/{unavailable:?}"
        );
    }
}
