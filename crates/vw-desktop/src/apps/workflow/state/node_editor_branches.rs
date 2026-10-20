//! # Workflow 分支节点编辑
//!
//! 该模块处理条件分支与知识检索节点的可视化草稿字段编辑更新。

use super::*;

impl WorkflowState {
    pub fn add_node_editor_classifier_class(&mut self) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::QuestionClassifier { classes, .. } = draft {
                let next = classes.len() + 1;
                classes.push(WorkflowClassifierClassDraft {
                    id: format!("class-{next}"),
                    name: format!("分类 {next}"),
                    ..WorkflowClassifierClassDraft::default()
                });
                true
            } else {
                false
            }
        });
    }

    pub fn remove_node_editor_classifier_class(&mut self, index: usize) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::QuestionClassifier { classes, .. } = draft
                && index < classes.len()
            {
                classes.remove(index);
                return true;
            }
            false
        });
    }

    pub fn set_node_editor_classifier_class_id(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::QuestionClassifier { classes, .. } = draft
                && let Some(class) = classes.get_mut(index)
            {
                class.id = value;
                return true;
            }
            false
        });
    }

    pub fn set_node_editor_classifier_class_name(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::QuestionClassifier { classes, .. } = draft
                && let Some(class) = classes.get_mut(index)
            {
                class.name = value;
                return true;
            }
            false
        });
    }

    pub fn add_node_editor_extractor_parameter(&mut self) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft {
                parameters.push(WorkflowParameterDraft {
                    value_type: "string".to_string(),
                    ..WorkflowParameterDraft::default()
                });
                true
            } else {
                false
            }
        });
    }

    pub fn remove_node_editor_extractor_parameter(&mut self, index: usize) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft
                && index < parameters.len()
            {
                parameters.remove(index);
                return true;
            }
            false
        });
    }

    pub fn edit_node_editor_structured_field(&mut self, edit: WorkflowStructuredNodeEdit) {
        self.update_node_visual_draft(|draft| apply_structured_node_edit(draft, edit));
    }

    pub fn add_node_editor_if_else_case(&mut self) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                cases.push(default_if_else_case_draft());
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_if_else_case_logical_operator(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(case) = cases.get_mut(index) {
                    case.logical_operator = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn add_node_editor_if_else_condition(&mut self, case_index: usize) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(case) = cases.get_mut(case_index) {
                    case.conditions.push(default_if_else_condition_draft());
                    return true;
                }
            }
            false
        });
    }

    pub fn remove_node_editor_if_else_condition(
        &mut self,
        case_index: usize,
        condition_index: usize,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(case) = cases.get_mut(case_index) {
                    if condition_index < case.conditions.len() {
                        case.conditions.remove(condition_index);
                        return true;
                    }
                }
            }
            false
        });
    }

    pub fn set_node_editor_if_else_condition_selector(
        &mut self,
        case_index: usize,
        condition_index: usize,
        value: String,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(condition) = cases
                    .get_mut(case_index)
                    .and_then(|case| case.conditions.get_mut(condition_index))
                {
                    condition.variable_selector_input = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_if_else_condition_operator(
        &mut self,
        case_index: usize,
        condition_index: usize,
        value: String,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(condition) = cases
                    .get_mut(case_index)
                    .and_then(|case| case.conditions.get_mut(condition_index))
                {
                    condition.comparison_operator = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_if_else_condition_value(
        &mut self,
        case_index: usize,
        condition_index: usize,
        value: String,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(condition) = cases
                    .get_mut(case_index)
                    .and_then(|case| case.conditions.get_mut(condition_index))
                {
                    condition.compare_value = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_if_else_condition_var_type(
        &mut self,
        case_index: usize,
        condition_index: usize,
        value: String,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::IfElse { cases } = draft {
                if let Some(condition) = cases
                    .get_mut(case_index)
                    .and_then(|case| case.conditions.get_mut(condition_index))
                {
                    condition.var_type = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_knowledge_query_selector(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { query_selector_input, .. } = draft
            {
                *query_selector_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_query_attachment_selector(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval {
                query_attachment_selector_input,
                ..
            } = draft
            {
                *query_attachment_selector_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_dataset_ids(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { dataset_ids_input, .. } = draft {
                *dataset_ids_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_retrieval_mode(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { retrieval_mode, .. } = draft {
                *retrieval_mode = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_top_k(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { top_k_input, .. } = draft {
                *top_k_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_score_threshold_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { score_threshold_enabled, .. } =
                draft
            {
                *score_threshold_enabled = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_score_threshold(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { score_threshold_input, .. } = draft
            {
                *score_threshold_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_reranking_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { reranking_enable, .. } = draft {
                *reranking_enable = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_knowledge_reranking_model_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { reranking_model_editor, .. } =
                draft
            {
                reranking_model_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_metadata_filtering_mode(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { metadata_filtering_mode, .. } =
                draft
            {
                *metadata_filtering_mode = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_knowledge_metadata_model_config_action(
        &mut self,
        action: text_editor::Action,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval {
                metadata_model_config_editor,
                ..
            } = draft
            {
                metadata_model_config_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_single_model_provider(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { single_model_provider, .. } = draft
            {
                *single_model_provider = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_single_model_name(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { single_model_name, .. } = draft {
                *single_model_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_knowledge_single_model_mode(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::KnowledgeRetrieval { single_model_mode, .. } = draft {
                *single_model_mode = value;
                true
            } else {
                false
            }
        });
    }
}

fn apply_structured_node_edit(
    draft: &mut WorkflowNodeVisualDraft,
    edit: WorkflowStructuredNodeEdit,
) -> bool {
    let (query, model, instruction) = match draft {
        WorkflowNodeVisualDraft::QuestionClassifier {
            query, model, instruction_editor, ..
        }
        | WorkflowNodeVisualDraft::ParameterExtractor {
            query, model, instruction_editor, ..
        } => (query, model, instruction_editor),
        _ => return false,
    };
    match edit {
        WorkflowStructuredNodeEdit::Query(value) => {
            *query = WorkflowVariableSelectorDraft::from_input(&value)
        }
        WorkflowStructuredNodeEdit::ModelProvider(value) => model.provider = value,
        WorkflowStructuredNodeEdit::ModelName(value) => model.name = value,
        WorkflowStructuredNodeEdit::ModelMode(value) => model.mode = value,
        WorkflowStructuredNodeEdit::Instruction(action) => instruction.perform(action),
        WorkflowStructuredNodeEdit::ReasoningMode(value) => {
            if let WorkflowNodeVisualDraft::ParameterExtractor { reasoning_mode, .. } = draft {
                *reasoning_mode = value;
            } else {
                return false;
            }
        }
        WorkflowStructuredNodeEdit::ParameterName(index, value) => {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft
                && let Some(parameter) = parameters.get_mut(index)
            {
                parameter.name = value;
            } else {
                return false;
            }
        }
        WorkflowStructuredNodeEdit::ParameterType(index, value) => {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft
                && let Some(parameter) = parameters.get_mut(index)
            {
                parameter.value_type = value;
            } else {
                return false;
            }
        }
        WorkflowStructuredNodeEdit::ParameterDescription(index, value) => {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft
                && let Some(parameter) = parameters.get_mut(index)
            {
                parameter.description = value;
            } else {
                return false;
            }
        }
        WorkflowStructuredNodeEdit::ParameterRequired(index, value) => {
            if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = draft
                && let Some(parameter) = parameters.get_mut(index)
            {
                parameter.required = value;
            } else {
                return false;
            }
        }
    }
    true
}
