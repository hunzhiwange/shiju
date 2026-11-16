//! Agent 上下文构建模块
//!
//! 本模块提供基于记忆检索的上下文构建功能，用于为 Agent 对话生成相关的前置上下文。
//! 主要功能包括：
//! - 从记忆系统中检索与当前消息相关的记忆条目
//! - 基于相关性评分过滤低质量记忆
//! - 构建格式化的上下文字符串供 Agent 使用

use agent_memory::{self as memory, Memory};
use std::fmt::Write;

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;

/// 构建上下文前导内容，通过搜索记忆系统查找相关条目
///
/// 该函数从记忆系统中检索与用户消息最相关的条目，并根据相关性评分进行过滤。
/// 相关性评分低于阈值的条目会被丢弃，以防止无关记忆干扰对话上下文。
///
/// # 参数
///
/// * `mem` - 记忆系统实现，实现了 `Memory` trait 的对象引用
/// * `user_msg` - 用户输入的消息文本，用于作为检索关键词
/// * `min_relevance_score` - 最小相关性评分阈值，低于此值的记忆条目将被过滤
///
/// # 返回值
///
/// 返回格式化的上下文字符串，包含筛选后的相关记忆条目。
/// 如果没有找到相关记忆，则返回空字符串。
///
/// # 示例
///
/// ```ignore
/// let context = build_context(&memory_store, "用户的问题", 0.5).await;
/// // context 可能包含："[Memory context]\n- key1: content1\n- key2: content2\n"
/// ```
pub async fn build_context(mem: &dyn Memory, user_msg: &str, min_relevance_score: f64) -> String {
    let mut context = String::new();

    // 从记忆系统检索与当前消息相关的记忆条目
    // 限制最多返回 5 条最相关的记忆
    if let Ok(entries) = mem.recall(user_msg, 5, None).await {
        // 基于相关性评分过滤记忆条目
        // 只保留评分达到阈值的条目，避免无关记忆污染上下文
        let relevant: Vec<_> = entries
            .iter()
            .filter(|e| match e.score {
                Some(score) => score >= min_relevance_score,
                None => true, // 如果条目没有评分，默认保留
            })
            .collect();

        // 如果存在相关记忆，构建上下文字符串
        if !relevant.is_empty() {
            context.push_str("[Memory context]\n");

            // 遍历所有相关记忆条目，格式化为列表
            for entry in &relevant {
                // 跳过 Assistant 自动保存的记忆条目
                // 这些通常是系统自动生成的，不应作为用户上下文
                if memory::is_assistant_autosave_key(&entry.key) {
                    continue;
                }
                // 将记忆条目格式化为 "- key: content" 的形式
                let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
            }

            // 检查是否成功添加了有效的记忆条目
            // 如果只有标题没有内容（所有条目都被跳过），则清空上下文
            if context == "[Memory context]\n" {
                context.clear();
            } else {
                // 在上下文末尾添加空行，便于与其他内容分隔
                context.push('\n');
            }
        }
    }

    context
}
