use iced::widget::svg;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Provider 提供商图标缓存映射表
///
/// 存储 AI 模型提供商的图标，使用提供商 ID 字符串作为键。
/// 支持多种主流 AI 服务提供商，包括：
/// - OpenAI、Anthropic、Google、Azure 等大型云服务
/// - DeepSeek、Moonshot、智谱 AI 等中国本土服务
/// - Ollama、LMStudio 等本地部署服务
///
/// # 性能
///
/// 使用 `Lazy` 延迟加载，通过宏简化重复的插入操作。
static PROVIDER_ICONS: Lazy<HashMap<&'static str, svg::Handle>> = Lazy::new(|| {
    /// 插入 Provider 图标的辅助宏
    ///
    /// 自动拼接文件路径并从嵌入的资源中加载 SVG 图标。
    ///
    /// # 参数
    ///
    /// - `$m`: HashMap 可变引用
    /// - `$id`: Provider ID 字符串字面量
    macro_rules! insert_provider {
        ($m:ident, $id:literal) => {
            $m.insert(
                $id,
                svg::Handle::from_memory(include_bytes!(concat!(
                    "../../../../../assets/icons/provider/",
                    $id,
                    ".svg"
                ))),
            );
        };
    }

    let mut m: HashMap<&'static str, svg::Handle> = HashMap::new();

    // 插入所有 Provider 图标
    insert_provider!(m, "abacus");
    insert_provider!(m, "agent");
    insert_provider!(m, "aihubmix");
    insert_provider!(m, "alibaba");
    insert_provider!(m, "alibaba-cn");
    insert_provider!(m, "amazon-bedrock");
    insert_provider!(m, "anthropic");
    insert_provider!(m, "azure");
    insert_provider!(m, "azure-cognitive-services");
    insert_provider!(m, "bailing");
    insert_provider!(m, "baseten");
    insert_provider!(m, "cerebras");
    insert_provider!(m, "chutes");
    insert_provider!(m, "cloudflare-ai-gateway");
    insert_provider!(m, "cloudflare-workers-ai");
    insert_provider!(m, "cohere");
    insert_provider!(m, "cortecs");
    insert_provider!(m, "deepinfra");
    insert_provider!(m, "deepseek");
    insert_provider!(m, "fastrouter");
    insert_provider!(m, "fireworks-ai");
    insert_provider!(m, "friendli");
    insert_provider!(m, "github-copilot");
    insert_provider!(m, "github-models");
    insert_provider!(m, "google");
    insert_provider!(m, "google-vertex");
    insert_provider!(m, "google-vertex-anthropic");
    insert_provider!(m, "groq");
    insert_provider!(m, "helicone");
    insert_provider!(m, "huggingface");
    insert_provider!(m, "iflowcn");
    insert_provider!(m, "inception");
    insert_provider!(m, "inference");
    insert_provider!(m, "io-net");
    insert_provider!(m, "kimi-for-coding");
    insert_provider!(m, "llama");
    insert_provider!(m, "lmstudio");
    insert_provider!(m, "lucidquery");
    insert_provider!(m, "minimax");
    insert_provider!(m, "minimax-cn");
    insert_provider!(m, "mistral");
    insert_provider!(m, "modelscope");
    insert_provider!(m, "moonshotai");
    insert_provider!(m, "moonshotai-cn");
    insert_provider!(m, "morph");
    insert_provider!(m, "nano-gpt");
    insert_provider!(m, "nebius");
    insert_provider!(m, "nvidia");
    insert_provider!(m, "ollama-cloud");
    insert_provider!(m, "openai");
    insert_provider!(m, "openrouter");
    insert_provider!(m, "ovhcloud");
    insert_provider!(m, "perplexity");
    insert_provider!(m, "poe");
    insert_provider!(m, "requesty");
    insert_provider!(m, "sap-ai-core");
    insert_provider!(m, "scaleway");
    insert_provider!(m, "siliconflow");
    insert_provider!(m, "siliconflow-cn");
    insert_provider!(m, "submodel");
    insert_provider!(m, "synthetic");
    insert_provider!(m, "togetherai");
    insert_provider!(m, "upstage");
    insert_provider!(m, "v0");
    insert_provider!(m, "venice");
    insert_provider!(m, "vercel");
    insert_provider!(m, "vultr");
    insert_provider!(m, "wandb");
    insert_provider!(m, "xai");
    insert_provider!(m, "xiaomi");
    insert_provider!(m, "zai");
    insert_provider!(m, "zai-coding-plan");
    insert_provider!(m, "zenmux");
    insert_provider!(m, "zhipuai");
    insert_provider!(m, "zhipuai-coding-plan");

    m
});

/// 获取指定 Provider 的图标句柄
///
/// 根据 Provider ID 从缓存中获取对应的图标。
/// 如果找不到指定的 Provider 图标，则返回默认的 "agent" 图标。
///
/// # 参数
///
/// - `provider_id`: Provider 标识符字符串（如 "openai"、"anthropic"、"deepseek" 等）
///
/// # 返回值
///
/// 返回对应的 SVG 图标句柄。如果指定的 Provider 不存在，返回默认图标。
///
/// # Panic
///
/// 仅在 "agent" 默认图标也不存在时 panic，这表示资源映射表配置错误。
///
/// # 示例
///
/// ```ignore
/// use desktop_assets::assets::get_provider_icon;
///
/// let openai_icon = get_provider_icon("openai");
/// let unknown_icon = get_provider_icon("unknown-provider"); // 返回 "agent" 图标
/// ```
pub fn get_provider_icon(provider_id: &str) -> svg::Handle {
    PROVIDER_ICONS
        .get(provider_id)
        .cloned()
        .or_else(|| PROVIDER_ICONS.get("agent").cloned())
        .expect("Provider icon missing in assets map")
}
#[cfg(test)]
#[path = "provider_icons_tests.rs"]
mod provider_icons_tests;
