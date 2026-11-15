//! 飞书消息确认反应模块
//!
//! 本模块提供飞书(Lark)通道的消息确认(ACK)反应功能，根据用户的语言环境
//! 智能选择合适的emoji表情作为消息确认标识。
//!
//! # 主要功能
//!
//! - **语言环境检测**：从消息载荷(payload)或文本内容中自动检测用户的语言环境
//! - **智能表情选择**：根据检测到的语言从对应的表情池中随机选择一个确认表情
//! - **多语言支持**：支持简体中文、繁体中文、英文、日文四种语言
//!
//! # 语言检测策略
//!
//! 1. 优先从消息元数据中提取locale信息
//! 2. 其次从消息内容的JSON结构中推断
//! 3. 最后通过文本特征（字符编码范围、繁简体特征）进行启发式检测
//!
//! # 使用示例
//!
//! ```ignore
//! use channel_lark::ack::{random_lark_ack_reaction, detect_lark_ack_locale};
//!
//! // 从消息载荷中检测语言并获取随机确认表情
//! let reaction = random_lark_ack_reaction(Some(&payload), "fallback text");
//!
//! // 仅检测语言环境
//! let locale = detect_lark_ack_locale(Some(&payload), "文本内容");
//! ```

use super::constants::{
    LARK_ACK_REACTIONS_EN, LARK_ACK_REACTIONS_JA, LARK_ACK_REACTIONS_ZH_CN,
    LARK_ACK_REACTIONS_ZH_TW,
};

/// 飞书确认反应支持的语言环境枚举
///
/// 定义了四种语言环境，用于选择对应语言的确认表情池。
/// 每种语言环境对应一组预设的emoji表情。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LarkAckLocale {
    /// 简体中文 (zh-CN)
    ZhCn,
    /// 繁体中文 (zh-TW/HK/MO)
    ZhTw,
    /// 英文 (en)
    En,
    /// 日文 (ja)
    Ja,
}

/// 从指定长度范围内均匀随机选择一个索引
///
/// 使用拒绝采样(rejection sampling)算法确保完全均匀分布，
/// 避免简单取模运算在小范围值上产生的微小偏差。
///
/// # 参数
///
/// - `len`: 可选索引的上界（不含）
///
/// # 返回值
///
/// 返回 `[0, len)` 范围内的随机索引
///
/// # Panics
///
/// 在debug模式下，如果 `len` 为0会触发断言失败
///
/// # 算法说明
///
/// 标准的 `rand % len` 方法会导致高位值出现概率略高，产生非均匀分布。
/// 本函数通过拒绝采样：仅接受小于 `reject_threshold` 的随机值，
/// 确保每个输出索引的概率完全相等。
#[allow(clippy::cast_possible_truncation)]
fn pick_uniform_index(len: usize) -> usize {
    debug_assert!(len > 0);
    let upper = len as u64;
    // 计算拒绝阈值：最大的能被upper整除的值
    // 超过此阈值的随机值会被拒绝并重新采样
    let reject_threshold = (u64::MAX / upper) * upper;

    loop {
        let value = rand::random::<u64>();
        if value < reject_threshold {
            return (value % upper) as usize;
        }
    }
}

/// 从表情池中随机选择一个表情
///
/// 使用均匀随机算法从给定的表情字符串数组中选择一个元素。
///
/// # 参数
///
/// - `pool`: 静态字符串切片数组，包含候选表情
///
/// # 返回值
///
/// 返回池中随机选择的一个表情字符串的静态引用
fn random_from_pool(pool: &'static [&'static str]) -> &'static str {
    pool[pick_uniform_index(pool.len())]
}

/// 根据语言环境获取对应的确认表情池
///
/// 将语言环境枚举映射到预定义的表情池常量。
///
/// # 参数
///
/// - `locale`: 语言环境枚举值
///
/// # 返回值
///
/// 返回对应语言的静态表情字符串数组
fn lark_ack_pool(locale: LarkAckLocale) -> &'static [&'static str] {
    match locale {
        LarkAckLocale::ZhCn => LARK_ACK_REACTIONS_ZH_CN,
        LarkAckLocale::ZhTw => LARK_ACK_REACTIONS_ZH_TW,
        LarkAckLocale::En => LARK_ACK_REACTIONS_EN,
        LarkAckLocale::Ja => LARK_ACK_REACTIONS_JA,
    }
}

/// 将语言标签字符串映射为语言环境枚举
///
/// 解析各种格式的语言标签（如 "zh-CN", "zh_TW", "en-US", "ja-JP" 等），
/// 并转换为内部使用的语言环境枚举。
///
/// # 参数
///
/// - `tag`: 语言标签字符串，支持多种格式（带连字符或下划线）
///
/// # 返回值
///
/// - `Some(LarkAckLocale)`: 成功识别的语言环境
/// - `None`: 无法识别的标签
///
/// # 匹配规则
///
/// 1. **日文**：以 "ja" 开头
/// 2. **英文**：以 "en" 开头
/// 3. **繁体中文**：包含 "hant"，或以 "zh_tw"/"zh_hk"/"zh_mo" 开头
/// 4. **简体中文**：以 "zh" 开头（其他中文标签）
///
/// # 示例
///
/// ```ignore
/// assert_eq!(map_locale_tag("zh-CN"), Some(LarkAckLocale::ZhCn));
/// assert_eq!(map_locale_tag("zh-TW"), Some(LarkAckLocale::ZhTw));
/// assert_eq!(map_locale_tag("en-US"), Some(LarkAckLocale::En));
/// assert_eq!(map_locale_tag("ja_JP"), Some(LarkAckLocale::Ja));
/// ```
pub(crate) fn map_locale_tag(tag: &str) -> Option<LarkAckLocale> {
    // 标准化标签：去除首尾空白、转小写、统一连字符为下划线
    let normalized = tag.trim().to_ascii_lowercase().replace('-', "_");
    if normalized.is_empty() {
        return None;
    }

    // 按优先级匹配语言标签
    if normalized.starts_with("ja") {
        return Some(LarkAckLocale::Ja);
    }
    if normalized.starts_with("en") {
        return Some(LarkAckLocale::En);
    }
    // 繁体中文：包含hant脚本标签，或为港台澳地区
    if normalized.contains("hant")
        || normalized.starts_with("zh_tw")
        || normalized.starts_with("zh_hk")
        || normalized.starts_with("zh_mo")
    {
        return Some(LarkAckLocale::ZhTw);
    }
    // 简体中文：其他以zh开头的标签
    if normalized.starts_with("zh") {
        return Some(LarkAckLocale::ZhCn);
    }
    None
}

/// 从JSON值中递归查找语言环境提示信息
///
/// 在JSON对象或数组中搜索常见的语言环境字段名，
/// 并返回第一个找到的语言标签值。
///
/// # 参数
///
/// - `value`: 要搜索的JSON值
///
/// # 返回值
///
/// - `Some(String)`: 找到的语言标签字符串
/// - `None`: 未找到任何语言标签
///
/// # 搜索的字段名
///
/// 按顺序尝试以下字段名：
/// "locale", "language", "lang", "i18n_locale", "user_locale", "locale_id"
///
/// # 递归行为
///
/// 如果顶层未找到，会递归搜索所有子对象和数组元素。
fn find_locale_hint(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            // 按优先级尝试常见的语言环境字段名
            for key in ["locale", "language", "lang", "i18n_locale", "user_locale", "locale_id"] {
                if let Some(locale) = map.get(key).and_then(serde_json::Value::as_str) {
                    return Some(locale.to_string());
                }
            }

            // 递归搜索所有子对象
            for child in map.values() {
                if let Some(locale) = find_locale_hint(child) {
                    return Some(locale);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            // 递归搜索数组中的所有元素
            for child in items {
                if let Some(locale) = find_locale_hint(child) {
                    return Some(locale);
                }
            }
            None
        }
        _ => None,
    }
}

/// 从飞书帖子消息内容中检测语言环境
///
/// 飞书富文本消息的内容是JSON格式，其顶层key可能包含语言标识。
/// 本函数尝试从这些key中识别语言环境。
///
/// # 参数
///
/// - `content`: 飞书帖子消息的JSON内容字符串
///
/// # 返回值
///
/// - `Some(LarkAckLocale)`: 成功识别的语言环境
/// - `None`: 解析失败或无法识别
///
/// # 示例
///
/// 飞书帖子内容可能如下结构：
/// ```json
/// {
///   "zh_cn": { "title": "标题", "content": [...] },
///   "en_us": { "title": "Title", "content": [...] }
/// }
/// ```
/// 函数会尝试将这些顶层key映射为语言环境。
fn detect_locale_from_post_content(content: &str) -> Option<LarkAckLocale> {
    let parsed = serde_json::from_str::<serde_json::Value>(content).ok()?;
    let obj = parsed.as_object()?;
    // 遍历JSON对象的key，尝试将其作为语言标签识别
    for key in obj.keys() {
        if let Some(locale) = map_locale_tag(key) {
            return Some(locale);
        }
    }
    None
}

/// 判断字符是否为日文假名（平假名或片假名）
///
/// 检查字符是否属于以下Unicode范围：
/// - 平假名 (Hiragana): U+3040 - U+309F
/// - 片假名 (Katakana): U+30A0 - U+30FF
/// - 片假名音标扩展: U+31F0 - U+31FF
///
/// # 参数
///
/// - `ch`: 要检查的字符
///
/// # 返回值
///
/// 如果是假名字符返回 `true`，否则返回 `false`
fn is_japanese_kana(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3040..=0x309F | // 平假名 (Hiragana)
        0x30A0..=0x30FF | // 片假名 (Katakana)
        0x31F0..=0x31FF // 片假名音标扩展 (Katakana Phonetic Extensions)
    )
}

/// 判断字符是否为CJK统一汉字或扩展A区
///
/// 检查字符是否属于以下Unicode范围：
/// - CJK扩展A区: U+3400 - U+4DBF
/// - CJK统一汉字: U+4E00 - U+9FFF
///
/// # 参数
///
/// - `ch`: 要检查的字符
///
/// # 返回值
///
/// 如果是CJK汉字字符返回 `true`，否则返回 `false`
fn is_cjk_han(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | // CJK扩展A区 (CJK Extension A)
        0x4E00..=0x9FFF // CJK统一汉字 (CJK Unified Ideographs)
    )
}

/// 判断字符是否为繁体中文特有汉字
///
/// 检查字符是否属于仅存在于繁体中文的汉字集合。
/// 这些字符在简体中文中有对应的简化形式。
///
/// # 参数
///
/// - `ch`: 要检查的字符
///
/// # 返回值
///
/// 如果是繁体中文特有字符返回 `true`，否则返回 `false`
///
/// # 字符列表
///
/// 包含以下繁体特有字：奮、鬥、強、體、國、臺、萬、與、為、這、學、機、開、裡
fn is_traditional_only_han(ch: char) -> bool {
    matches!(
        ch,
        '奮' | '鬥'
            | '強'
            | '體'
            | '國'
            | '臺'
            | '萬'
            | '與'
            | '為'
            | '這'
            | '學'
            | '機'
            | '開'
            | '裡'
    )
}

/// 判断字符是否为简体中文特有汉字
///
/// 检查字符是否属于仅存在于简体中文的汉字集合。
/// 这些字符是繁体中文对应字的简化形式。
///
/// # 参数
///
/// - `ch`: 要检查的字符
///
/// # 返回值
///
/// 如果是简体中文特有字符返回 `true`，否则返回 `false`
///
/// # 字符列表
///
/// 包含以下简体特有字：奋、斗、强、体、国、台、万、与、为、这、学、机、开、里
fn is_simplified_only_han(ch: char) -> bool {
    matches!(
        ch,
        '奋' | '斗'
            | '强'
            | '体'
            | '国'
            | '台'
            | '万'
            | '与'
            | '为'
            | '这'
            | '学'
            | '机'
            | '开'
            | '里'
    )
}

/// 通过文本特征检测语言环境
///
/// 使用启发式规则从文本内容推断语言环境：
/// 1. 存在日文假名 → 日文
/// 2. 存在繁体中文特有字 → 繁体中文
/// 3. 存在简体中文特有字 → 简体中文
/// 4. 存在其他CJK汉字 → 简体中文（默认）
///
/// # 参数
///
/// - `text`: 要分析的文本字符串
///
/// # 返回值
///
/// - `Some(LarkAckLocale)`: 检测到的语言环境
/// - `None`: 无法确定语言环境（文本中未发现特征字符）
fn detect_locale_from_text(text: &str) -> Option<LarkAckLocale> {
    // 检查是否包含日文假名
    if text.chars().any(is_japanese_kana) {
        return Some(LarkAckLocale::Ja);
    }
    // 检查是否包含繁体中文特有字
    if text.chars().any(is_traditional_only_han) {
        return Some(LarkAckLocale::ZhTw);
    }
    // 检查是否包含简体中文特有字
    if text.chars().any(is_simplified_only_han) {
        return Some(LarkAckLocale::ZhCn);
    }
    // 检查是否包含其他CJK汉字，默认为简体中文
    if text.chars().any(is_cjk_han) {
        return Some(LarkAckLocale::ZhCn);
    }
    None
}

/// 综合检测飞书消息的语言环境
///
/// 这是语言检测的主入口函数，按优先级从多个来源尝试检测语言环境。
///
/// # 检测顺序（优先级从高到低）
///
/// 1. 从消息载荷的元数据字段中提取locale信息
/// 2. 从飞书帖子消息的JSON内容结构中推断
/// 3. 从回退文本内容进行启发式字符分析
///
/// # 参数
///
/// - `payload`: 可选的消息载荷JSON值，包含消息元数据和内容
/// - `fallback_text`: 回退文本，用于无法从载荷提取语言时的字符特征分析
///
/// # 返回值
///
/// 返回检测到的语言环境，如果所有方法都无法确定则默认返回英文(`LarkAckLocale::En`)
///
/// # 示例
///
/// ```ignore
/// let locale = detect_lark_ack_locale(Some(&json_payload), "这是简体中文");
/// assert_eq!(locale, LarkAckLocale::ZhCn);
/// ```
pub(crate) fn detect_lark_ack_locale(
    payload: Option<&serde_json::Value>,
    fallback_text: &str,
) -> LarkAckLocale {
    if let Some(payload) = payload {
        // 策略1：从消息元数据中提取语言标签
        if let Some(locale) = find_locale_hint(payload).and_then(|hint| map_locale_tag(&hint)) {
            return locale;
        }

        // 策略2：尝试从飞书帖子消息内容中检测语言
        // 支持 /message/content 和 /event/message/content 两种路径
        let message_content =
            payload.pointer("/message/content").and_then(serde_json::Value::as_str).or_else(|| {
                payload.pointer("/event/message/content").and_then(serde_json::Value::as_str)
            });

        if let Some(locale) = message_content.and_then(detect_locale_from_post_content) {
            return locale;
        }
    }

    // 策略3：从回退文本进行字符特征分析，失败则默认为英文
    detect_locale_from_text(fallback_text).unwrap_or(LarkAckLocale::En)
}

/// 获取随机的飞书确认表情反应
///
/// 这是模块的主要对外接口，自动检测语言环境并返回适合的随机确认表情。
///
/// # 参数
///
/// - `payload`: 可选的消息载荷JSON值，用于智能语言检测
/// - `fallback_text`: 回退文本，当载荷中无法检测语言时使用
///
/// # 返回值
///
/// 返回一个静态字符串引用，指向从对应语言表情池中随机选择的emoji表情
///
/// # 示例
///
/// ```ignore
/// // 从完整载荷中智能检测
/// let reaction = random_lark_ack_reaction(Some(&payload), "");
///
/// // 仅从文本检测
/// let reaction = random_lark_ack_reaction(None, "这是一条中文消息");
///
/// // 使用默认英文表情
/// let reaction = random_lark_ack_reaction(None, "Hello world");
/// ```
pub(crate) fn random_lark_ack_reaction(
    payload: Option<&serde_json::Value>,
    fallback_text: &str,
) -> &'static str {
    let locale = detect_lark_ack_locale(payload, fallback_text);
    random_from_pool(lark_ack_pool(locale))
}

#[cfg(test)]
#[path = "ack_tests.rs"]
mod ack_tests;
