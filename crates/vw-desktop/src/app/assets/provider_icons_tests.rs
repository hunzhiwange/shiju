use super::*;

const EXPECTED_PROVIDER_IDS: &[&str] = &[
    "abacus",
    "agent",
    "aihubmix",
    "alibaba",
    "alibaba-cn",
    "amazon-bedrock",
    "anthropic",
    "azure",
    "azure-cognitive-services",
    "bailing",
    "baseten",
    "cerebras",
    "chutes",
    "cloudflare-ai-gateway",
    "cloudflare-workers-ai",
    "cohere",
    "cortecs",
    "deepinfra",
    "deepseek",
    "fastrouter",
    "fireworks-ai",
    "friendli",
    "github-copilot",
    "github-models",
    "google",
    "google-vertex",
    "google-vertex-anthropic",
    "groq",
    "helicone",
    "huggingface",
    "iflowcn",
    "inception",
    "inference",
    "io-net",
    "kimi-for-coding",
    "llama",
    "lmstudio",
    "lucidquery",
    "minimax",
    "minimax-cn",
    "mistral",
    "modelscope",
    "moonshotai",
    "moonshotai-cn",
    "morph",
    "nano-gpt",
    "nebius",
    "nvidia",
    "ollama-cloud",
    "openai",
    "openrouter",
    "ovhcloud",
    "perplexity",
    "poe",
    "requesty",
    "sap-ai-core",
    "scaleway",
    "siliconflow",
    "siliconflow-cn",
    "submodel",
    "synthetic",
    "togetherai",
    "upstage",
    "v0",
    "venice",
    "vercel",
    "vultr",
    "wandb",
    "xai",
    "xiaomi",
    "zai",
    "zai-coding-plan",
    "zenmux",
    "zhipuai",
    "zhipuai-coding-plan",
];

#[test]
fn provider_icons_map_contains_all_expected_provider_ids() {
    assert_eq!(PROVIDER_ICONS.len(), EXPECTED_PROVIDER_IDS.len());

    for &provider_id in EXPECTED_PROVIDER_IDS {
        assert!(PROVIDER_ICONS.contains_key(provider_id), "{provider_id}");
    }
}

#[test]
fn get_provider_icon_returns_known_provider_icons() {
    for &provider_id in EXPECTED_PROVIDER_IDS {
        assert_eq!(
            get_provider_icon(provider_id),
            PROVIDER_ICONS.get(provider_id).cloned().expect("expected provider icon")
        );
    }
}

#[test]
fn get_provider_icon_falls_back_to_agent_for_unknown_blank_or_case_mismatched_ids() {
    let fallback_icon = PROVIDER_ICONS.get("agent").cloned().expect("agent icon should be present");

    for provider_id in ["unknown-provider", "", "OPENAI"] {
        assert_eq!(get_provider_icon(provider_id), fallback_icon);
    }
}

#[test]
fn provider_icons_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("provider_icons_tests.rs"));
}
