use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAppEditorMode {
    Create,
    Edit(String),
}

#[derive(Debug, Clone)]
pub struct WorkflowAppEditorDraft {
    pub mode: WorkflowAppEditorMode,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub use_icon_as_answer_icon: bool,
    pub max_active_requests_input: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowNodeEditorMode {
    Create,
    Edit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowNodeEditorTab {
    Basic,
    Description,
    Visual,
    AdvancedDsl,
}

#[derive(Debug)]
pub struct WorkflowNodeEditorDraft {
    pub mode: WorkflowNodeEditorMode,
    pub active_tab: WorkflowNodeEditorTab,
    pub block_type: String,
    pub title: String,
    pub description: String,
    pub description_editor: text_editor::Content,
    pub position: Point,
    pub visual_draft: Option<WorkflowNodeVisualDraft>,
    pub validation: WorkflowNodeEditorValidation,
    pub show_raw_data_editor: bool,
    pub raw_data_editor: text_editor::Content,
    pub hovered_start_variable_index: Option<usize>,
    pub start_variable_focus_index: Option<usize>,
    pub start_variable_editor: Option<WorkflowStartVariableEditorDraft>,
}

#[derive(Debug)]
pub enum WorkflowNodeVisualDraft {
    Start {
        variables: Vec<WorkflowStartVariableDraft>,
    },
    Llm {
        provider: String,
        model_name: String,
        model_mode: String,
        enable_thinking: bool,
        completion_params_editor: text_editor::Content,
        structured_output_editor: text_editor::Content,
        memory_editor: text_editor::Content,
        context_enabled: bool,
        context_selector_input: String,
        system_prompt_editor: text_editor::Content,
        user_prompt_editor: text_editor::Content,
        vision_enabled: bool,
        vision_selector_input: String,
    },
    Answer {
        answer_editor: text_editor::Content,
    },
    End {
        outputs: Vec<WorkflowNamedValueDraft>,
    },
    TemplateTransform {
        variables: Vec<WorkflowNamedValueDraft>,
        template_editor: text_editor::Content,
    },
    DocumentExtractor {
        variable_selector: WorkflowVariableSelectorDraft,
    },
    QuestionClassifier {
        query: WorkflowVariableSelectorDraft,
        model: WorkflowModelDraft,
        classes: Vec<WorkflowClassifierClassDraft>,
        instruction_editor: text_editor::Content,
    },
    ParameterExtractor {
        query: WorkflowVariableSelectorDraft,
        model: WorkflowModelDraft,
        parameters: Vec<WorkflowParameterDraft>,
        reasoning_mode: String,
        instruction_editor: text_editor::Content,
    },
    HttpRequest {
        method: String,
        url: String,
        auth_type: String,
        auth_username: String,
        auth_password: String,
        auth_token: String,
        headers_editor: text_editor::Content,
        params_editor: text_editor::Content,
        body_type: String,
        body_editor: text_editor::Content,
        timeout_connect: String,
        timeout_read: String,
        timeout_write: String,
        ssl_verify: bool,
        retry_config: WorkflowNodeRetryDraft,
        error_strategy: String,
    },
    VariableAssigner {
        items: Vec<WorkflowAssignmentDraft>,
    },
    VariableAggregator {
        variables: Vec<String>,
        output_type: String,
    },
    ListOperator {
        variable: String,
        conditions: Vec<WorkflowListConditionDraft>,
        filter_enabled: bool,
        order_enabled: bool,
        order_key: String,
        order_direction: String,
        limit_enabled: bool,
        limit_size: String,
    },
    Iteration {
        iterator_selector: String,
        output_selector: String,
        output_type: String,
        is_parallel: bool,
        parallel_nums: String,
        error_handle_mode: String,
    },
    Loop {
        variables: Vec<WorkflowLoopVariableDraft>,
        break_conditions: Vec<WorkflowLoopConditionDraft>,
        logical_operator: String,
        max_loop_count: String,
    },
    IfElse {
        cases: Vec<WorkflowIfElseCaseDraft>,
    },
    KnowledgeRetrieval {
        query_selector_input: String,
        query_attachment_selector_input: String,
        dataset_ids_input: String,
        retrieval_mode: String,
        top_k_input: String,
        score_threshold_enabled: bool,
        score_threshold_input: String,
        reranking_enable: bool,
        reranking_model_editor: text_editor::Content,
        metadata_filtering_mode: String,
        metadata_model_config_editor: text_editor::Content,
        single_model_provider: String,
        single_model_name: String,
        single_model_mode: String,
    },
    Tool {
        provider_id: String,
        provider_type: String,
        provider_name: String,
        tool_name: String,
        tool_label: String,
        tool_description: String,
        credential_id: String,
        plugin_unique_identifier: String,
        tool_parameters_editor: text_editor::Content,
        tool_configurations_editor: text_editor::Content,
    },
    Agent {
        strategy_provider_name: String,
        strategy_name: String,
        strategy_label: String,
        plugin_unique_identifier: String,
        output_schema_editor: text_editor::Content,
        parameters_editor: text_editor::Content,
        memory_enabled: bool,
        memory_window_size_input: String,
        memory_prompt_editor: text_editor::Content,
    },
    Code {
        language: String,
        inputs: Vec<WorkflowCodeVariableDraft>,
        code_editor: text_editor::Content,
        outputs: Vec<WorkflowCodeOutputDraft>,
        retry_config: WorkflowNodeRetryDraft,
        error_strategy: String,
        default_value_editor: text_editor::Content,
    },
}

#[derive(Debug, Clone)]
pub struct WorkflowCodeVariableDraft {
    pub variable: String,
    pub value_type: String,
    pub selector: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowCodeOutputDraft {
    pub key: String,
    pub value_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowVariableSelectorDraft {
    pub path: Vec<String>,
}

impl WorkflowVariableSelectorDraft {
    pub fn from_input(input: &str) -> Self {
        Self {
            path: input
                .split('.')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }

    pub fn to_input(&self) -> String {
        self.path.join(".")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowNamedValueDraft {
    pub id: String,
    pub name: String,
    pub value_type: String,
    pub selector: WorkflowVariableSelectorDraft,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowKeyValueDraft {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowModelDraft {
    pub provider: String,
    pub name: String,
    pub mode: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowClassifierClassDraft {
    pub raw_class: Value,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowParameterDraft {
    pub raw_parameter: Value,
    pub name: String,
    pub value_type: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowAssignmentDraft {
    pub raw_item: Value,
    pub variable_selector: WorkflowVariableSelectorDraft,
    pub variable_selector_input: String,
    pub operation: String,
    pub input_type: String,
    pub value: Value,
    pub value_input: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowListConditionDraft {
    pub raw_condition: Value,
    pub key: String,
    pub comparison_operator: String,
    pub value: Value,
    pub value_input: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowLoopVariableDraft {
    pub raw_variable: Value,
    pub id: String,
    pub label: String,
    pub value_type: String,
    pub initial_value: Value,
    pub initial_value_input: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowLoopConditionDraft {
    pub raw_condition: Value,
    pub selector: String,
    pub comparison_operator: String,
    pub value: Value,
    pub value_input: String,
}

#[derive(Debug, Clone)]
pub enum WorkflowDataNodeEdit {
    AssignmentAdd,
    AssignmentRemove(usize),
    AssignmentTarget(usize, String),
    AssignmentOperation(usize, String),
    AssignmentInputType(usize, String),
    AssignmentValue(usize, String),
    AggregatorAdd,
    AggregatorRemove(usize),
    AggregatorVariable(usize, String),
    AggregatorOutputType(String),
    ListVariable(String),
    ListFilterEnabled(bool),
    ListConditionAdd,
    ListConditionRemove(usize),
    ListConditionKey(usize, String),
    ListConditionOperator(usize, String),
    ListConditionValue(usize, String),
    ListOrderEnabled(bool),
    ListOrderKey(String),
    ListOrderDirection(String),
    ListLimitEnabled(bool),
    ListLimitSize(String),
    IterationIterator(String),
    IterationOutput(String),
    IterationOutputType(String),
    IterationParallel(bool),
    IterationParallelNums(String),
    IterationErrorMode(String),
    LoopVariableAdd,
    LoopVariableRemove(usize),
    LoopVariableId(usize, String),
    LoopVariableLabel(usize, String),
    LoopVariableType(usize, String),
    LoopVariableValue(usize, String),
    LoopConditionAdd,
    LoopConditionRemove(usize),
    LoopConditionSelector(usize, String),
    LoopConditionOperator(usize, String),
    LoopConditionValue(usize, String),
    LoopLogicalOperator(String),
    LoopMaxCount(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowNodeFieldEdit {
    VariableSelectorChanged(String),
    NamedValueAdded,
    NamedValueRemoved(usize),
    NamedValueNameChanged(usize, String),
    NamedValueTypeChanged(usize, String),
    NamedValueSelectorChanged(usize, String),
    KeyValueAdded,
    KeyValueRemoved(usize),
    KeyValueKeyChanged(usize, String),
    KeyValueValueChanged(usize, String),
    ModelProviderChanged(String),
    ModelNameChanged(String),
    ModelModeChanged(String),
    RetryEnabledChanged(bool),
    RetryMaxRetriesChanged(u8),
    RetryIntervalChanged(u16),
}

#[derive(Debug, Clone)]
pub enum WorkflowStructuredNodeEdit {
    Query(String),
    ModelProvider(String),
    ModelName(String),
    ModelMode(String),
    Instruction(text_editor::Action),
    ReasoningMode(String),
    ParameterName(usize, String),
    ParameterType(usize, String),
    ParameterDescription(usize, String),
    ParameterRequired(usize, bool),
}

#[derive(Debug, Clone)]
pub enum WorkflowHttpRequestEdit {
    Method(String),
    Url(String),
    AuthType(String),
    AuthUsername(String),
    AuthPassword(String),
    AuthToken(String),
    Headers(text_editor::Action),
    Params(text_editor::Action),
    BodyType(String),
    Body(text_editor::Action),
    TimeoutConnect(String),
    TimeoutRead(String),
    TimeoutWrite(String),
    SslVerify(bool),
    RetryEnabled(bool),
    RetryMaxRetries(u8),
    RetryInterval(u16),
    ErrorStrategy(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowNodeRetryDraft {
    pub enabled: bool,
    pub max_retries: u8,
    pub retry_interval: u16,
}

#[derive(Debug, Clone)]
pub struct WorkflowStartVariableDraft {
    pub raw_variable: Value,
    pub label: String,
    pub variable: String,
    pub input_type: String,
    pub required: bool,
    pub hidden: bool,
    pub options: Vec<String>,
    pub allowed_file_types: Vec<String>,
    pub allowed_file_extensions: Vec<String>,
    pub allowed_file_extensions_input: String,
    pub allowed_file_upload_methods: Vec<String>,
    pub default_value: String,
    pub default_file_values: Vec<String>,
    pub placeholder: String,
    pub hint: String,
    pub max_length_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStartVariableEditorMode {
    Create,
    Edit(usize),
}

#[derive(Debug, Clone)]
pub struct WorkflowStartVariableEditorDraft {
    pub mode: WorkflowStartVariableEditorMode,
    pub variable: WorkflowStartVariableDraft,
    pub default_value_editor: text_editor::Content,
    pub default_file_url_input: String,
    pub show_default_file_url_input: bool,
}

#[derive(Debug)]
pub struct WorkflowIfElseCaseDraft {
    pub raw_case: Value,
    pub case_id: String,
    pub logical_operator: String,
    pub conditions: Vec<WorkflowIfElseConditionDraft>,
}

#[derive(Debug)]
pub struct WorkflowIfElseConditionDraft {
    pub raw_condition: Value,
    pub variable_selector_input: String,
    pub comparison_operator: String,
    pub compare_value: String,
    pub var_type: String,
}

#[derive(Debug, Default)]
pub struct WorkflowNodeEditorValidation {
    pub field_errors: Vec<WorkflowNodeValidationError>,
}

#[derive(Debug)]
pub struct WorkflowNodeValidationError {
    pub path: String,
    pub message: desktop_i18n::workflow::Feedback,
}

impl WorkflowNodeEditorValidation {
    pub fn has_errors(&self) -> bool {
        !self.field_errors.is_empty()
    }

    pub fn localized_error_for(&self, i18n: desktop_i18n::I18n, path: &str) -> Option<String> {
        self.field_errors
            .iter()
            .find(|item| item.path == path)
            .map(|item| item.message.render(i18n))
    }

    pub fn first_error_for(&self, path: &str) -> Option<&str> {
        self.field_errors.iter().find(|item| item.path == path).map(|item| item.message.as_str())
    }
}
