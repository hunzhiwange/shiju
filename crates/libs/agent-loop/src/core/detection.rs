use regex::Regex;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "detection_tests.rs"]
mod detection_tests;

/// 动作完成提示词正则表达式
static ACTION_COMPLETION_CUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)\b(done|completed?|finished|successfully|i(?:'ve|\s+have)|we(?:'ve|\s+have))\b",
    )
    .unwrap()
});

/// 副作用动作动词正则表达式
static SIDE_EFFECT_ACTION_VERB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)\b(create|created|write|wrote|run|ran|execute|executed|update|updated|delete|deleted|remove|removed|rename|renamed|move|moved|moving|install|installed|save|saved|make|made)\b",
    )
    .unwrap()
});

/// 副作用动作对象正则表达式
static SIDE_EFFECT_ACTION_OBJECT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)\b(file|files|folder|folders|directory|directories|workspace|cwd|current\s+working\s+directory|command|commands|script|scripts|path|paths)\b",
    )
    .unwrap()
});

/// 检测文本是否看起来像是一个未经工具验证的动作完成声明
pub fn looks_like_unverified_action_completion_without_tool_call(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    ACTION_COMPLETION_CUE_REGEX.is_match(trimmed)
        && SIDE_EFFECT_ACTION_VERB_REGEX.is_match(trimmed)
        && SIDE_EFFECT_ACTION_OBJECT_REGEX.is_match(trimmed)
}
