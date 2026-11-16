//! Unified prompt rendering and capability decisions.
use std::collections::BTreeSet;
use agent_project::instance;
const PROMPT_UNIFIED: &str = include_str!("prompt/unified.txt");
pub const UNIFIED_PROMPT_MARKER: &str =
    "You are ShiJu, a general-purpose autonomous assistant";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimePromptModes {
    pub goal: bool,
    pub workflow: bool,
}

impl RuntimePromptModes {
    pub fn from_options(options: &serde_json::Value) -> Self {
        Self {
            goal: options.get("goal_mode").and_then(serde_json::Value::as_bool).unwrap_or(false),
            workflow: options
                .get("workflow_mode")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        }
    }
}

fn has_tool(tool_ids: &BTreeSet<String>, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| tool_ids.contains(*candidate))
}

fn runtime_capability_instructions(available_tool_ids: &[String], recursive_policy: &str) -> String {
    let tool_ids = available_tool_ids
        .iter()
        .map(|tool| tool.trim().to_ascii_lowercase())
        .filter(|tool| !tool.is_empty())
        .collect::<BTreeSet<_>>();
    let mut sections = Vec::new();

    let mut file_rules = Vec::new();
    if has_tool(&tool_ids, &["file_read"]) {
        file_rules.push(
            "Use `file_read` to inspect text. If the result is truncated or paginated, continue reading the required content using offset/limit.",
        );
    }
    if has_tool(&tool_ids, &["glob"]) || has_tool(&tool_ids, &["grep"]) {
        let search_rule = match (has_tool(&tool_ids, &["glob"]), has_tool(&tool_ids, &["grep"])) {
            (true, true) => {
                "Use `glob` to find paths and `grep` to search contents. After finding matches, read the required context."
            }
            (true, false) => "Use `glob` to find files by path pattern.",
            (false, true) => {
                "Use `grep` to search file contents. After finding matches, read the required context."
            }
            (false, false) => unreachable!(),
        };
        file_rules.push(search_rule);
    }
    if has_tool(&tool_ids, &["apply_patch"]) {
        file_rules.push(
            "When modifying existing text, prefer `apply_patch` for focused patches and preserve unrelated changes.",
        );
    }
    if has_tool(&tool_ids, &["file_edit"]) {
        file_rules.push(
            "Read the file before invoking `file_edit`. By default, `old_string` must be unique; for multiple replacements, provide more specific context or explicitly set `replace_all`.",
        );
    }
    if has_tool(&tool_ids, &["file_write"]) {
        file_rules.push(
            "Use `file_write` only for new files or when a complete replacement is genuinely necessary. Read an existing file before overwriting it.",
        );
    }
    if !file_rules.is_empty() {
        sections.push(format!("## Files and Search\n- {}", file_rules.join("\n- ")));
    }

    if has_tool(&tool_ids, &["bash", "shell"]) {
        sections.push(
            "## Command Execution\n- Use `bash` for project CLIs, builds, tests, and Git. Check the exit code of every result; when a command fails, investigate the cause first and do not treat failed output as valid evidence.\n- Do not use shell commands such as `cat`, `find`, or `grep` to bypass available specialized file tools unless project protocols explicitly require it."
                .to_string(),
        );
    }

    let skill_available = has_tool(&tool_ids, &["skill"]);
    if skill_available {
        sections.push(
            "## Skills\n- Available skills are listed by name and description in a temporary `<system-reminder>`. When a skill matches the task or the user names one, invoke the Skill Tool first. After it succeeds, follow the injected skill instructions in full; do not guess paths or preload instructions."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["todoread"]) && has_tool(&tool_ids, &["todowrite"]) {
        sections.push(
            "## Task Tracking\n- Use `TodoWrite` for complex, multi-step work or when the user requests a plan. Skip task lists for simple questions and single-step work; do not add filler steps. Keep each step short, concrete, and verifiable.\n- Create the list once, then prefer `merge:true` with existing ids and only changed fields. Keep at most one item in_progress. In one call, mark finished steps completed and the next step in_progress; batch genuinely completed steps when appropriate. Do not repeatedly read a known list with `TodoRead`.\n- Base completion on actual results and required verification. Leave blocked work unfinished and explain the blocker; never invent completion to clear the list. Resume from unfinished work when asked to continue, and synchronize statuses before reporting completion.\n- The tool displays the list; do not repeat the entire plan in ordinary text. Briefly explain meaningful plan changes or blockers."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["agenttool"]) {
        sections.push(format!(
            "## Subagents\n{}",
            recursive_policy
        ));
    }

    if has_tool(&tool_ids, &["create_thread"]) {
        sections.push(
            "## Visible Task Sessions\n- Use `create_thread` when the user explicitly requests a user-visible independent task in the current project. Use `AgentTool` for a hidden subtask and keep plan-run work inside its owning workflow.\n- After creation, record the thread ID and coordinate it with the available task tools only when necessary. Do not take over the target task locally; keep monitoring until it completes or needs user action."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["process"]) {
        sections.push(
            "## Background Tasks\n- Run long-running commands in the background with `process` and record the task ID. Continue independent work in the meantime; do not busy-wait with `sleep`.\n- Before the final response, read the output of tasks that remain relevant, confirm their exit status, and terminate background tasks that no longer provide value. Running does not mean validation passed."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["batch"]) {
        sections.push(
            "## Parallel Tool Calls\n- Use `batch` only to combine calls that are independent, concurrency-safe, and do not contend for the same state. Run calls separately when they have sequential dependencies, approval requirements, or write conflicts."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["askuserquestion", "question"]) {
        sections.push(
            "## Asking the User Questions\n- Use the question tool only when a critical ambiguity, missing credentials, or a high-risk decision still blocks progress after you inspect the context. Complete all unblocked work first and ask only one specific question."
                .to_string(),
        );
    }

    if has_tool(
        &tool_ids,
        &[
            "websearch",
            "web_search",
            "websearchtool",
            "webfetch",
            "web_fetch",
            "browser",
            "browseropen",
            "http_request",
        ],
    ) {
        sections.push(
            "## Network and Browser\n- Use the currently available search, fetch, or browser tools for current information, external pages, and user-requested verification. Distinguish search summaries from page evidence and cite relevant sources in the final response.\n- The browser does not implicitly provide page state. Obtain the current snapshot or interactive state before acting, and revalidate the target page after a refresh or reconstruction."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["list_mcp_resources"])
        || has_tool(&tool_ids, &["list_mcp_resource_templates"])
    {
        sections.push(
            "## MCP Resources\n- When external systems provide structured resources or templates through MCP, first discover and read the corresponding resources. Do not substitute web search for authoritative context that is directly accessible."
                .to_string(),
        );
    }

    sections.join("\n\n")
}

fn runtime_mode_instructions(modes: RuntimePromptModes) -> String {
    let mut rules = Vec::new();
    if modes.goal {
        rules.push(
            "Goal mode is enabled: treat the original request as an ongoing goal. Continue until the completion criteria and necessary validation are satisfied; do not claim completion merely because a turn ended, part of the task succeeded, or you provided a verbal summary. Only external dependencies, permissions, or security risks that you cannot resolve yourself constitute a genuine blocker.",
        );
    }
    if modes.workflow {
        rules.push(
            "Workflow mode is enabled: the current session is part of a planning, batch-execution, or goal-acceptance workflow. Strictly complete the phase and task boundaries specified by the current prompt; do not skip dependency batches, take over other tasks, or change ownership of the outer plan.",
        );
    }
    if rules.is_empty() {
        String::new()
    } else {
        format!("## Current Runtime Mode\n- {}", rules.join("\n- "))
    }
}

pub fn render_unified_prompt_with_modes(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    runtime_modes: RuntimePromptModes,
    recursive_policy: &str,
) -> String {
    let model_id = model_id.trim();
    let model_id = if model_id.is_empty() { "unknown" } else { model_id };
    let working_directory = working_directory.trim();
    let working_directory = if working_directory.is_empty() { "." } else { working_directory };
    PROMPT_UNIFIED
        .replace("{{model_id}}", model_id)
        .replace("{{working_directory}}", working_directory)
        .replace("{{runtime_capabilities}}", &runtime_capability_instructions(available_tool_ids, recursive_policy))
        .replace("{{runtime_modes}}", &runtime_mode_instructions(runtime_modes))
        .trim()
        .to_string()
}

pub fn render_unified_prompt(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> String {
    render_unified_prompt_with_modes(
        model_id,
        working_directory,
        available_tool_ids,
        RuntimePromptModes::default(),
        recursive_policy,
    )
}

pub fn render_unified_prompt_from_ref(
    model: Option<&str>,
    root: Option<&str>,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> String {
    render_unified_prompt_from_ref_with_modes(
        model,
        root,
        available_tool_ids,
        RuntimePromptModes::default(),
        recursive_policy,
    )
}

pub fn render_unified_prompt_from_ref_with_modes(
    model: Option<&str>,
    root: Option<&str>,
    available_tool_ids: &[String],
    runtime_modes: RuntimePromptModes,
    recursive_policy: &str,
) -> String {
    render_unified_prompt_with_modes(
        model.unwrap_or("unknown"),
        &resolve_cwd(root),
        available_tool_ids,
        runtime_modes,
        recursive_policy,
    )
}

pub fn resolve_cwd(root: Option<&str>) -> String {
    // 优先使用传入的 root 参数（过滤空白字符串）
    if let Some(root) = root.filter(|s| !s.trim().is_empty()) {
        return root.trim().to_string();
    }

    // 尝试从项目实例获取目录
    let cwd = instance::directory();
    if !cwd.trim().is_empty() {
        return cwd;
    }

    // 回退到系统当前目录
    std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
}

#[cfg(test)]
#[path = "system_prompt_tests.rs"]
mod system_prompt_tests;
