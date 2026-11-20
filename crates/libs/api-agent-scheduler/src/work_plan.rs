//! Work plan publication contract and create-task-plans prompt parsing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum WorkPlanPublication {
    Generated {
        planning_attempt_id: String,
        subtasks: Vec<TaskPlanSubTask>,
    },
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanSubTask {
    pub title: String,
    pub boundary: String,
    pub acceptance_criteria: Vec<String>,
    pub target_files: Vec<String>,
    /// 任务所属批次，来自 `prompt.md` 标题中的 `【批次 N】`。
    pub batch: u32,
    /// 依赖批次列表，已去重并升序排序；无依赖时为空。
    pub depends_on_batches: Vec<u32>,
    /// 来自 `prompt.md` 中紧随标题的 ```text 代码块的完整可复制执行提示。
    pub execution_prompt: String,
}


#[derive(Default, Deserialize)]
#[serde(default)]
struct PlanDetails {
    boundary: String,
    acceptance_criteria: Vec<String>,
    target_files: Vec<String>,
}

/// 解析 `## 任务 N: <标题> 【批次 N】 依赖批次 X,Y` 标题行，返回标题、批次与依赖批次。
///
/// 缺失批次或依赖批次非法（自依赖）时返回明确错误，不伪造默认值。
pub fn parse_task_heading(heading: &str) -> Result<(String, u32, Vec<u32>), String> {
    // 去掉 `## 任务 N:` 前缀，保留标题与批次/依赖批次文本。
    let after_marker = heading
        .trim_start()
        .trim_start_matches('#')
        .trim_start()
        .strip_prefix("任务")
        .ok_or_else(|| format!("任务标题缺少“任务”前缀: {heading}"))?;
    // 去掉任务编号与冒号：`3: 标题 ...` -> `标题 ...`。
    let rest = match after_marker.split_once(':').or_else(|| after_marker.split_once('：')) {
        Some((_, rest)) => rest.trim(),
        None => after_marker.trim(),
    };

    let batch_start =
        rest.find("【批次").ok_or_else(|| format!("任务标题缺少“【批次 N】”: {heading}"))?;
    let title = rest[..batch_start].trim().to_string();
    if title.is_empty() {
        return Err(format!("任务标题为空: {heading}"));
    }

    let batch_section = &rest[batch_start..];
    let batch_inner = batch_section
        .strip_prefix("【批次")
        .and_then(|value| value.split_once('】'))
        .map(|(inner, _)| inner.trim())
        .ok_or_else(|| format!("任务标题“【批次 N】”格式不正确: {heading}"))?;
    let batch: u32 =
        batch_inner.parse().map_err(|_| format!("无法解析批次编号“{batch_inner}”: {heading}"))?;

    if batch == 0 {
        return Err(format!("批次编号必须为正整数: {heading}"));
    }

    let depends_text = batch_section.split_once('】').map(|(_, tail)| tail.trim()).unwrap_or("");
    let depends_on_batches = parse_depends_on_batches(depends_text, batch, heading)?;

    Ok((title, batch, depends_on_batches))
}

/// 解析 `依赖批次 X,Y` 文本：去重、升序排序，并校验不自依赖。
fn parse_depends_on_batches(
    depends_text: &str,
    batch: u32,
    heading: &str,
) -> Result<Vec<u32>, String> {
    let body = depends_text.trim().strip_prefix("依赖批次").map(str::trim).unwrap_or("");
    if body.is_empty() || body == "无" {
        return Ok(Vec::new());
    }

    let mut batches = Vec::new();
    for part in body.split([',', '，', ' ', '、']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let value: u32 =
            part.parse().map_err(|_| format!("无法解析依赖批次“{part}”: {heading}"))?;
        if value == 0 {
            return Err(format!("依赖批次必须为正整数: {heading}"));
        }
        if value == batch {
            return Err(format!("依赖批次不能包含当前批次 {batch}: {heading}"));
        }
        if value > batch {
            return Err(format!("依赖批次 {value} 不能大于当前批次 {batch}: {heading}"));
        }
        if !batches.contains(&value) {
            batches.push(value);
        }
    }
    batches.sort_unstable();
    Ok(batches)
}

/// 解析 create-task-plans 生成的 `prompt.md`，把每个 `## 任务 N` 转换为带批次依赖的子任务。
///
/// 子任务顺序与 `prompt.md` 顺序一致；完整执行提示来自紧随标题的 ```text 代码块。
/// 没有任务、缺失代码块或批次/依赖非法时返回明确错误，不伪造成功子任务。
pub fn parse_create_task_plans_prompt(prompt_md: &str) -> Result<Vec<TaskPlanSubTask>, String> {
    let lines: Vec<&str> = prompt_md.lines().collect();
    let mut subtasks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if !is_task_heading(line) {
            index += 1;
            continue;
        }

        let (title, batch, depends_on_batches) = parse_task_heading(line)?;

        // 在下一个任务标题之前寻找紧随其后的 ```text 代码块。
        let mut cursor = index + 1;
        let mut details = PlanDetails::default();
        let mut execution_prompt: Option<String> = None;
        while cursor < lines.len() {
            let candidate = lines[cursor];
            if is_task_heading(candidate) {
                break;
            }
            if let Some(value) = candidate.trim_start().strip_prefix("计划元数据:") {
                details = serde_json::from_str(value.trim())
                    .map_err(|error| format!("任务“{title}”的计划元数据无效: {error}"))?;
            }
            if candidate.trim_start().starts_with("```") {
                let mut body = String::new();
                cursor += 1;
                let mut closed = false;
                while cursor < lines.len() {
                    if lines[cursor].trim_start().starts_with("```") {
                        closed = true;
                        cursor += 1;
                        break;
                    }
                    body.push_str(lines[cursor]);
                    body.push('\n');
                    cursor += 1;
                }
                if !closed {
                    return Err(format!("任务“{title}”的代码块未闭合"));
                }
                let trimmed = body.trim();
                if trimmed.is_empty() {
                    return Err(format!("任务“{title}”的执行提示代码块为空"));
                }
                execution_prompt = Some(trimmed.to_string());
                break;
            }
            cursor += 1;
        }

        let execution_prompt =
            execution_prompt.ok_or_else(|| format!("任务“{title}”缺少执行提示代码块"))?;

        subtasks.push(TaskPlanSubTask {
            title,
            boundary: details.boundary,
            acceptance_criteria: details.acceptance_criteria,
            target_files: details.target_files,
            batch,
            depends_on_batches,
            execution_prompt,
        });

        index = cursor;
    }

    if subtasks.is_empty() {
        return Err("prompt.md 中没有任何“## 任务 N”任务".to_string());
    }

    Ok(subtasks)
}

/// 判断一行是否为 `## 任务 N` 编号任务标题（“任务”后必须紧跟数字编号）。
pub fn is_task_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return false;
    }
    let body = trimmed.trim_start_matches('#').trim_start();
    let Some(after) = body.strip_prefix("任务") else {
        return false;
    };
    after.trim_start().starts_with(|ch: char| ch.is_ascii_digit())
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPlanPublishRequest {
    pub plan: WorkPlanPublication,
    #[serde(default)]
    pub dispatch_mode: crate::AgentSchedulerDispatchModeDto,
    #[serde(default)]
    pub max_concurrent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPlanPublishResponse {
    pub child_task_ids: Vec<String>,
}

/// 旧任务池将子任务列表写入任务元数据。
pub fn has_legacy_work_plan(metadata: &serde_json::Value) -> bool {
    metadata.get("subtasks").and_then(serde_json::Value::as_array)
        .is_some_and(|subtasks| !subtasks.is_empty())
}

pub const LEGACY_WORK_PLAN_RESPLIT_REASON: &str = "旧子任务计划无法继续执行，请重新拆分";
