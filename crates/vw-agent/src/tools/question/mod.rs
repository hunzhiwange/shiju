//! 用户交互提问工具
//!
//! 在执行过程中向用户提问以收集偏好、澄清模糊指令或获取决策。
//! 支持预设选项列表、多选和自定义输入。

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::question;
use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use vw_api_tools::ToolResultContentDto;

const MAX_HEADER_CHARS: usize = 12;
const MAX_QUESTIONS: usize = 4;
const MIN_OPTIONS: usize = 2;
const MAX_OPTIONS: usize = 4;
const CUSTOM_ANSWER_PREFIX: &str = "__custom__:";

/// 单个选项的输入结构
///
/// 表示用户可选的一个具体选项，包含显示标签和详细描述。
/// 用于构建选择题的选项列表。
#[derive(Debug, Clone, Deserialize, Serialize)]
struct OptionInput {
    /// 选项的显示标签（1-5个单词，简洁）
    label: String,
    /// 选项的详细说明
    description: String,
    /// 宿主支持时用于辅助比较的预览内容
    #[serde(default)]
    preview: Option<String>,
}

/// 单个问题的输入结构
///
/// 定义一个向用户提问的问题，包含问题内容、标题、选项列表、
/// 是否允许多选以及是否允许自定义输入等配置。
#[derive(Debug, Clone, Deserialize, Serialize)]
struct QuestionInput {
    /// 完整的问题文本
    question: String,
    /// 简短标签（最多30个字符），用于界面显示
    #[serde(default)]
    header: Option<String>,
    /// 可选的选项列表，用于构建选择题
    #[serde(default)]
    options: Option<Vec<OptionInput>>,
    /// 是否允许选择多个选项
    #[serde(default, rename = "multiSelect", alias = "multiple")]
    multi_select: Option<bool>,
    /// 是否允许用户自定义输入（默认启用时会自动添加"其他"选项）
    #[serde(default)]
    custom: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AnnotationInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct MetadataInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

/// 工具参数结构
///
/// 包含要向用户提问的问题列表。
/// 是 QuestionTool 的输入参数 schema 的 Rust 表示。
#[derive(Debug, Clone, Deserialize)]
struct ArgsInput {
    /// 要提问的问题列表
    questions: Vec<QuestionInput>,
    /// 已采集答案的回填内容
    #[serde(default)]
    answers: Option<BTreeMap<String, String>>,
    /// 每个问题的附加标注
    #[serde(default)]
    annotations: Option<BTreeMap<String, AnnotationInput>>,
    /// 额外元信息
    #[serde(default)]
    metadata: Option<MetadataInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NormalizedOption {
    label: String,
    description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preview: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NormalizedQuestion {
    question: String,
    header: String,
    options: Vec<NormalizedOption>,
    #[serde(rename = "multiSelect")]
    multi_select: bool,
    custom: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NormalizedArgs {
    questions: Vec<NormalizedQuestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    answers: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    annotations: Option<BTreeMap<String, AnnotationInput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<MetadataInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OutputQuestion {
    question: String,
    header: String,
    options: Vec<NormalizedOption>,
    #[serde(rename = "multiSelect")]
    multi_select: bool,
}

#[derive(Debug, Clone)]
struct CollectedAnswers {
    answers: BTreeMap<String, String>,
    annotations: Option<BTreeMap<String, AnnotationInput>>,
}

/// 用户交互提问工具
///
/// 该工具允许代理在执行过程中向用户提问，用于：
/// - 收集用户偏好或需求
/// - 澄清模糊的指令
/// - 获取关键决策
/// - 在多个执行路径中选择
///
/// 支持多种问题类型：
/// - 开放式问题（允许自定义输入）
/// - 单选题（从预设选项中选择一个）
/// - 多选题（从预设选项中选择多个）
///
/// # 示例
///
/// ```ignore
/// let tool = QuestionTool::new("session-123".to_string());
/// let result = tool.execute(args).await?;
/// ```
#[derive(Clone)]
pub struct QuestionTool {
    /// 当前会话ID，用于标识提问来源和路由回答
    session_id: String,
}

impl QuestionTool {
    /// 创建新的提问工具实例
    ///
    /// # 参数
    ///
    /// * `session_id` - 会话标识符，用于关联提问和回答
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 QuestionTool 实例
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    /// 生成工具参数的 JSON Schema
    ///
    /// 定义了工具接受的参数结构，包括问题数组、选项数组等。
    /// 该 schema 用于：
    /// - 验证输入参数
    /// - 为 LLM 提供工具调用规范
    ///
    /// # 返回值
    ///
    /// 返回 JSON Schema 格式的参数定义
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_QUESTIONS,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "question": { "type": "string" },
                            "header": { "type": "string" },
                            "options": {
                                "type": "array",
                                "minItems": MIN_OPTIONS,
                                "maxItems": MAX_OPTIONS,
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" },
                                        "preview": { "type": "string" }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multiSelect": { "type": "boolean" },
                            "custom": { "type": "boolean" }
                        },
                        "required": ["question"]
                    }
                },
                "answers": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                },
                "annotations": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "preview": { "type": "string" },
                            "notes": { "type": "string" }
                        }
                    }
                },
                "metadata": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "source": { "type": "string" }
                    }
                }
            },
            "required": ["questions"]
        })
    }

    fn normalize_args(args: ArgsInput) -> anyhow::Result<NormalizedArgs> {
        if args.questions.is_empty() || args.questions.len() > MAX_QUESTIONS {
            bail!("questions must contain 1-4 items");
        }

        if args.annotations.is_some() && args.answers.is_none() {
            bail!("annotations require answers");
        }

        let mut seen_questions = HashSet::new();
        let mut questions = Vec::with_capacity(args.questions.len());
        for question in args.questions {
            let question_text = question.question.trim().to_string();
            if question_text.is_empty() {
                bail!("question text must not be empty");
            }
            if !seen_questions.insert(question_text.clone()) {
                bail!("Question texts must be unique");
            }

            let header = normalize_header(question.header.as_deref(), &question_text);
            let multi_select = question.multi_select.unwrap_or(false);
            let custom = question.custom.unwrap_or(true);
            let options = normalize_options(
                &question_text,
                question.options.unwrap_or_default(),
                multi_select,
            )?;

            if options.is_empty() && !custom {
                bail!(
                    "Question \"{question_text}\" must allow custom input when no options are provided"
                );
            }
            if multi_select && options.is_empty() {
                bail!("Question \"{question_text}\" cannot enable multiSelect without options");
            }

            questions.push(NormalizedQuestion {
                question: question_text,
                header,
                options,
                multi_select,
                custom,
            });
        }

        let answers = normalize_answer_map(args.answers, &questions)?;
        let annotations = normalize_annotations(args.annotations, &questions)?;
        let metadata = args.metadata.and_then(|metadata| {
            normalize_optional_text(metadata.source)
                .map(|source| MetadataInput { source: Some(source) })
        });

        Ok(NormalizedArgs { questions, answers, annotations, metadata })
    }

    fn build_questions(args: &NormalizedArgs) -> Vec<question::Info> {
        args.questions
            .iter()
            .map(|q| question::Info {
                header: q.header.clone(),
                question: q.question.clone(),
                options: q
                    .options
                    .iter()
                    .map(|o| question::OptionInfo {
                        label: o.label.clone(),
                        description: o.description.clone(),
                        preview: o.preview.clone(),
                    })
                    .collect(),
                multiple: Some(q.multi_select),
                custom: Some(q.custom),
            })
            .collect()
    }

    fn build_output_questions(args: &NormalizedArgs) -> Vec<OutputQuestion> {
        args.questions
            .iter()
            .map(|question| OutputQuestion {
                question: question.question.clone(),
                header: question.header.clone(),
                options: question.options.clone(),
                multi_select: question.multi_select,
            })
            .collect()
    }

    async fn collect_answers(
        &self,
        args: &NormalizedArgs,
    ) -> Result<CollectedAnswers, question::Error> {
        if let Some(answers) = args.answers.clone() {
            return Ok(CollectedAnswers {
                answers,
                annotations: args.annotations.clone().filter(|items| !items.is_empty()),
            });
        }

        let raw_answers = question::ask(question::AskInput {
            session_id: self.session_id.clone(),
            questions: Self::build_questions(args),
            tool: None,
        })
        .await?;

        Ok(CollectedAnswers {
            answers: build_answer_map(&args.questions, &raw_answers),
            annotations: inferred_annotations(&args.questions, &raw_answers),
        })
    }

    fn build_success_result(
        args: &NormalizedArgs,
        collected: CollectedAnswers,
    ) -> anyhow::Result<ToolCallResult> {
        let question_count = args.questions.len();
        let answer_count = collected.answers.len();
        let mut data = json!({
            "questions": Self::build_output_questions(args),
            "answers": collected.answers,
        });

        if let Some(annotations) = collected.annotations.filter(|items| !items.is_empty()) {
            if let Some(object) = data.as_object_mut() {
                object.insert("annotations".to_string(), serde_json::to_value(&annotations)?);
            }
        }

        let mut metadata = json!({
            "question_count": question_count,
            "answer_count": answer_count,
        });
        if let Some(source) = args.metadata.as_ref().and_then(|metadata| metadata.source.clone())
            && let Some(object) = metadata.as_object_mut()
        {
            object.insert("source".to_string(), Value::String(source));
        }

        let model_text = model_result_text(
            data.get("answers").and_then(Value::as_object),
            data.get("annotations").and_then(Value::as_object),
        );

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(model_text),
            content_blocks: vec![ToolResultContentDto::Json { value: data }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::ASK_USER_QUESTION_TOOL_ID.to_string()),
                kind: Some("ask_user_question".to_string()),
                summary: Some(format!("Collected {} answer(s)", answer_count)),
                metadata,
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }
}

/// 从问题文本生成简短标题
///
/// 如果用户未提供 header 字段，则从完整问题文本中提取前 12 个字符作为简短标题。
/// 用于在界面中显示简洁的问题标识。
///
/// # 参数
///
/// * `question` - 完整的问题文本
///
/// # 返回值
///
/// 返回最多 12 个字符的问题前缀作为标题
pub(crate) fn header_from_question(question: &str) -> String {
    question.chars().take(MAX_HEADER_CHARS).collect()
}

fn normalize_header(header: Option<&str>, question: &str) -> String {
    let explicit = header.and_then(|value| normalize_optional_text(Some(value.to_string())));
    let source = explicit.as_deref().unwrap_or(question);
    header_from_question(source)
}

fn normalize_options(
    question: &str,
    options: Vec<OptionInput>,
    multi_select: bool,
) -> anyhow::Result<Vec<NormalizedOption>> {
    if options.is_empty() {
        return Ok(Vec::new());
    }
    if options.len() < MIN_OPTIONS || options.len() > MAX_OPTIONS {
        bail!("Question \"{question}\" must have 2-4 options");
    }

    let mut seen_labels = HashSet::new();
    let mut normalized = Vec::with_capacity(options.len());
    for option in options {
        let label = option.label.trim().to_string();
        if label.is_empty() {
            bail!("Question \"{question}\" contains an empty option label");
        }
        if !seen_labels.insert(label.clone()) {
            bail!("Option labels must be unique within each question");
        }

        let preview = normalize_optional_text(option.preview);
        if multi_select && preview.is_some() {
            bail!("Question \"{question}\" cannot use previews when multiSelect is enabled");
        }

        normalized.push(NormalizedOption {
            label,
            description: option.description.trim().to_string(),
            preview,
        });
    }

    Ok(normalized)
}

fn normalize_answer_map(
    answers: Option<BTreeMap<String, String>>,
    questions: &[NormalizedQuestion],
) -> anyhow::Result<Option<BTreeMap<String, String>>> {
    let Some(answers) = answers else {
        return Ok(None);
    };

    if answers.len() != questions.len() {
        bail!("answers must include one entry for each question");
    }

    let mut normalized = BTreeMap::new();
    for question in questions {
        let Some(answer) = answers.get(question.question.as_str()) else {
            bail!("answers must be keyed by the exact question text");
        };
        let answer = answer.trim();
        if answer.is_empty() {
            bail!("answers must not contain empty values");
        }
        normalized.insert(question.question.clone(), answer.to_string());
    }

    Ok(Some(normalized))
}

fn normalize_annotations(
    annotations: Option<BTreeMap<String, AnnotationInput>>,
    questions: &[NormalizedQuestion],
) -> anyhow::Result<Option<BTreeMap<String, AnnotationInput>>> {
    let Some(annotations) = annotations else {
        return Ok(None);
    };

    let valid_questions =
        questions.iter().map(|question| question.question.as_str()).collect::<HashSet<_>>();
    let mut normalized = BTreeMap::new();
    for (question, annotation) in annotations {
        if !valid_questions.contains(question.as_str()) {
            bail!("annotations keys must match the provided question text");
        }

        let preview = normalize_optional_text(annotation.preview);
        let notes = normalize_optional_text(annotation.notes);
        if preview.is_none() && notes.is_none() {
            continue;
        }

        normalized.insert(question, AnnotationInput { preview, notes });
    }

    Ok((!normalized.is_empty()).then_some(normalized))
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed.to_string())
    })
}

fn build_answer_map(
    questions: &[NormalizedQuestion],
    answers: &[question::Answer],
) -> BTreeMap<String, String> {
    questions
        .iter()
        .enumerate()
        .map(|(index, question)| {
            let answer = answers
                .get(index)
                .map(|answer| normalize_selected_answers(answer).join(", "))
                .unwrap_or_default();
            (question.question.clone(), answer)
        })
        .collect()
}

fn normalize_selected_answers(answers: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for answer in answers {
        let value = answer.strip_prefix(CUSTOM_ANSWER_PREFIX).unwrap_or(answer.as_str()).trim();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.to_string()) {
            normalized.push(value.to_string());
        }
    }

    normalized
}

fn inferred_annotations(
    questions: &[NormalizedQuestion],
    answers: &[question::Answer],
) -> Option<BTreeMap<String, AnnotationInput>> {
    let mut annotations = BTreeMap::new();

    for (index, question) in questions.iter().enumerate() {
        if question.multi_select {
            continue;
        }

        let selected =
            answers.get(index).map(|answer| normalize_selected_answers(answer)).unwrap_or_default();
        if selected.len() != 1 {
            continue;
        }

        let Some(preview) = question
            .options
            .iter()
            .find(|option| option.label == selected[0])
            .and_then(|option| option.preview.clone())
        else {
            continue;
        };

        annotations.insert(
            question.question.clone(),
            AnnotationInput { preview: Some(preview), notes: None },
        );
    }

    (!annotations.is_empty()).then_some(annotations)
}

fn model_result_text(
    answers: Option<&serde_json::Map<String, Value>>,
    annotations: Option<&serde_json::Map<String, Value>>,
) -> String {
    let Some(answers) = answers else {
        return "User has answered your questions. You can now continue with the user's answers in mind.".to_string();
    };

    let mut parts = Vec::new();
    for (question, answer) in answers {
        let mut entry = vec![format!("\"{question}\"=\"{}\"", answer.as_str().unwrap_or_default())];
        if let Some(annotation) =
            annotations.and_then(|annotations| annotations.get(question)).and_then(Value::as_object)
        {
            if let Some(preview) = annotation.get("preview").and_then(Value::as_str) {
                entry.push(format!("selected preview:\n{preview}"));
            }
            if let Some(notes) = annotation.get("notes").and_then(Value::as_str) {
                entry.push(format!("user notes: {notes}"));
            }
        }
        parts.push(entry.join(" "));
    }

    if parts.is_empty() {
        return "User has answered your questions. You can now continue with the user's answers in mind.".to_string();
    }

    format!(
        "User has answered your questions: {}. You can now continue with the user's answers in mind.",
        parts.join(", ")
    )
}

/// 实现 Tool trait，使 QuestionTool 可作为代理工具使用
///
/// 提供工具的基本信息和执行逻辑，符合工具系统的标准接口。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for QuestionTool {
    /// 返回工具名称
    ///
    /// 工具名称用于在系统中注册和调用该工具。
    fn name(&self) -> &str {
        "question"
    }

    /// 返回工具描述
    ///
    /// 描述内容从外部文件 question.txt 加载，用于向 LLM 解释工具用途。
    fn description(&self) -> &str {
        include_str!("question.txt")
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义了调用该工具时所需的参数结构。
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::ASK_USER_QUESTION_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::ASK_USER_QUESTION_TOOL_ID)
        .with_aliases(vec![crate::app::agent::tools::QUESTION_TOOL_ALIAS.to_string()])
        .with_read_only(true)
        .with_destructive(false)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(true)
        .with_strict(true)
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        let args: ArgsInput = serde_json::from_value(input)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;
        let normalized = Self::normalize_args(args)?;
        serde_json::to_value(normalized)
            .map_err(|e| anyhow::anyhow!("Failed to normalize question parameters: {e}"))
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: NormalizedArgs = serde_json::from_value(input.clone())
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;
        let question_count = args.questions.len();

        match self.collect_answers(&args).await {
            Ok(collected) => Self::build_success_result(&args, collected),
            Err(question::Error::Rejected(err)) => {
                let mut result = ToolCallResult::from_legacy_result(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Denied: {err}")),
                });
                result.render_hint = Some(ToolRenderHint {
                    title: Some(crate::app::agent::tools::ASK_USER_QUESTION_TOOL_ID.to_string()),
                    kind: Some("ask_user_question".to_string()),
                    summary: Some("User declined to answer".to_string()),
                    metadata: json!({ "question_count": question_count }),
                });
                Ok(result)
            }
            Err(err) => {
                let mut result = ToolCallResult::from_legacy_result(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(err.to_string()),
                });
                result.render_hint = Some(ToolRenderHint {
                    title: Some(crate::app::agent::tools::ASK_USER_QUESTION_TOOL_ID.to_string()),
                    kind: Some("ask_user_question".to_string()),
                    summary: Some("Question failed".to_string()),
                    metadata: json!({ "question_count": question_count }),
                });
                Ok(result)
            }
        }
    }

    /// 执行提问操作
    ///
    /// 将问题发送给用户并等待回答。支持单选、多选和自定义输入。
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的参数，包含要提问的问题列表
    ///
    /// # 返回值
    ///
    /// 返回工具执行结果：
    /// - 成功时，`output` 包含用户回答的 JSON 字符串
    /// - 用户拒绝时，`success` 为 false，`error` 包含拒绝原因
    /// - 其他错误时，`error` 包含错误详情
    ///
    /// # 错误处理
    ///
    /// - 参数解析失败：返回错误消息
    /// - 用户拒绝回答：返回 ToolResult，success 为 false
    /// - 序列化失败：返回错误信息
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析并验证输入参数
        let args: NormalizedArgs = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        match self.collect_answers(&args).await {
            // 用户成功回答，序列化回答结果
            Ok(collected) => Ok(ToolResult {
                success: true,
                output: serde_json::to_string(&json!({
                    "questions": Self::build_output_questions(&args),
                    "answers": collected.answers,
                    "annotations": collected.annotations,
                }))
                .map_err(|e| anyhow::anyhow!("Failed to serialize answers: {e}"))?,
                error: None,
            }),
            // 用户明确拒绝回答
            Err(question::Error::Rejected(err)) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Denied: {err}")),
            }),
            // 其他错误情况
            Err(err) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(err.to_string()),
            }),
        }
    }
}

/// 单元测试模块
///
/// 测试 QuestionTool 的各种功能，包括参数解析、问题格式转换和执行逻辑。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
