//! 按 `tasks.json` 批次依赖构造执行阶段。

use super::PlanTaskEntry;
use std::collections::{BTreeMap, BTreeSet};

/// 将任务按批次升序分组为可并发执行的阶段。
///
/// 校验规则：
/// - 任务集合非空
/// - 依赖批次必须严格小于当前批次（禁止自依赖与前向依赖）
/// - 依赖批次必须在文档中存在
pub fn build_batch_stages(tasks: &[PlanTaskEntry]) -> Result<Vec<Vec<usize>>, String> {
    if tasks.is_empty() {
        return Err("批次调度失败: 任务集合为空".to_string());
    }

    let existing_batches: BTreeSet<u32> = tasks.iter().map(|task| task.batch).collect();

    let mut stages: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, task) in tasks.iter().enumerate() {
        let batch = task.batch;
        for dependency in &task.depends_on_batches {
            if *dependency >= batch {
                return Err(format!(
                    "批次调度失败: 任务 {} 的批次 {} 依赖了非更早批次 {}（不允许自依赖或前向依赖）",
                    task.id, batch, dependency
                ));
            }
            if !existing_batches.contains(dependency) {
                return Err(format!(
                    "批次调度失败: 任务 {} 依赖了不存在的批次 {}",
                    task.id, dependency
                ));
            }
        }
        stages.entry(batch).or_default().push(index);
    }

    Ok(stages.into_values().collect())
}
