use regex::Regex;
use std::sync::LazyLock;

/// 随机名称生成用的形容词列表
///
/// 用于生成友好的 worktree 名称（形容词 + 名词 组合）
const ADJECTIVES: &[&str] = &[
    "brave", "calm", "clever", "cosmic", "crisp", "curious", "eager", "gentle", "glowing", "happy",
    "hidden", "jolly", "kind", "lucky", "mighty", "misty", "neon", "nimble", "playful", "proud",
    "quick", "quiet", "shiny", "silent", "stellar", "sunny", "swift", "tidy", "witty",
];

/// 随机名称生成用的名词列表
///
/// 用于生成友好的 worktree 名称（形容词 + 名词 组合）
const NOUNS: &[&str] = &[
    "cabin", "cactus", "canyon", "circuit", "comet", "eagle", "engine", "falcon", "forest",
    "garden", "harbor", "island", "knight", "lagoon", "meadow", "moon", "mountain", "nebula",
    "orchid", "otter", "panda", "pixel", "planet", "river", "rocket", "sailor", "squid", "star",
    "tiger", "wizard", "wolf",
];

/// 生成随机 worktree 名称
///
/// 使用当前时间戳作为种子，从预定义的形容词和名词列表中选择组合
///
/// # 返回值
///
/// 格式为 `{adjective}-{noun}` 的字符串，例如 `"brave-eagle"`
pub(super) fn random_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as usize;
    let adj = ADJECTIVES[seed % ADJECTIVES.len()];
    let noun = NOUNS[(seed / 97) % NOUNS.len()];
    format!("{}-{}", adj, noun)
}

/// 将输入字符串转换为 URL 安全的 slug 格式
///
/// 执行以下转换：
/// 1. 转换为小写
/// 2. 将非字母数字字符替换为连字符
/// 3. 移除首尾的连字符
pub(super) fn slug(input: &str) -> String {
    static RE_NON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").unwrap());
    static RE_LEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-+").unwrap());
    static RE_TRAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-+$").unwrap());

    let lower = input.trim().to_ascii_lowercase();
    let dashed = RE_NON.replace_all(&lower, "-").to_string();
    let dashed = RE_LEAD.replace_all(&dashed, "").to_string();
    RE_TRAIL.replace_all(&dashed, "").to_string()
}
