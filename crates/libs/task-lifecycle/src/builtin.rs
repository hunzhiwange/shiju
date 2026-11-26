//! 内置工作流只提供状态目录及有序步骤，负责人需要按项目配置。
use vw_api_task_lifecycle::{
    TaskLifecycleStatusCategoryDto as Category, TaskLifecycleStatusDto, TaskLifecycleStepDto,
    TaskLifecycleStepHandoffDto, TaskLifecycleWorkflowDto,
};

pub const DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID: &str = "generic_task";

struct WorkflowSpec {
    id: &'static str,
    label: &'static str,
    statuses: &'static [(&'static str, &'static str, Category)],
}

const DEV_LIFECYCLE_STATUSES: &[(&str, &str, Category)] = &[
    ("pool", "任务池", Category::Unstarted),
    ("pending", "待执行", Category::Unstarted),
    ("running", "执行中", Category::Started),
    ("failed", "执行失败", Category::Started),
    ("paused", "暂停中", Category::Started),
    ("code_complete", "代码完成", Category::Started),
    ("pr_submitted", "合并代码", Category::Started),
    ("completed", "任务完成", Category::Done),
    ("archived", "任务归档", Category::Closed),
];

const CONTENT_PUBLISHING_STATUSES: &[(&str, &str, Category)] = &[
    ("content_idea", "选题池", Category::Unstarted),
    ("content_queued", "待写作", Category::Unstarted),
    ("content_outline", "资料与大纲", Category::Started),
    ("content_writing", "写作中", Category::Started),
    ("content_revision", "退回修改", Category::Started),
    ("content_paused", "暂缓发布", Category::Started),
    ("content_review_ready", "待审稿", Category::Started),
    ("content_reviewing", "审稿中", Category::Started),
    ("content_publishing", "多平台发布", Category::Started),
    ("content_published", "已发布", Category::Done),
    ("content_archived", "归档复盘", Category::Closed),
];

const MARKETING_CAMPAIGN_STATUSES: &[(&str, &str, Category)] = &[
    ("campaign_ideas", "活动想法", Category::Unstarted),
    ("campaign_queued", "待策划", Category::Unstarted),
    ("campaign_planning", "方案设计", Category::Started),
    ("campaign_running", "执行投放", Category::Started),
    ("campaign_blocked", "投放异常", Category::Started),
    ("campaign_paused", "暂停活动", Category::Started),
    ("campaign_review_ready", "待复盘", Category::Started),
    ("campaign_reviewing", "复盘中", Category::Started),
    ("campaign_closing", "结案同步", Category::Started),
    ("campaign_done", "已结案", Category::Done),
    ("campaign_archived", "资料归档", Category::Closed),
];

const CUSTOMER_SUPPORT_STATUSES: &[(&str, &str, Category)] = &[
    ("support_new", "新工单", Category::Unstarted),
    ("support_queued", "待处理", Category::Unstarted),
    ("support_triage", "问题定位", Category::Started),
    ("support_handling", "处理中", Category::Started),
    ("support_blocked", "处理受阻", Category::Started),
    ("support_waiting", "等待客户", Category::Started),
    ("support_followup_ready", "待回访", Category::Started),
    ("support_following", "回访中", Category::Started),
    ("support_closing", "关闭确认", Category::Started),
    ("support_resolved", "已解决", Category::Done),
    ("support_archived", "工单归档", Category::Closed),
];

const RECRUITING_STATUSES: &[(&str, &str, Category)] = &[
    ("recruiting_pool", "人才池", Category::Unstarted),
    ("recruiting_queued", "待筛选", Category::Unstarted),
    ("recruiting_screening", "简历初筛", Category::Started),
    ("recruiting_interviewing", "面试中", Category::Started),
    ("recruiting_blocked", "流程异常", Category::Started),
    ("recruiting_on_hold", "暂缓", Category::Started),
    ("recruiting_review_ready", "待评审", Category::Started),
    ("recruiting_reviewing", "评审中", Category::Started),
    ("recruiting_offer", "Offer 跟进", Category::Started),
    ("recruiting_hired", "已入职", Category::Done),
    ("recruiting_archived", "候选人归档", Category::Closed),
];

const GENERIC_TASK_STATUSES: &[(&str, &str, Category)] = &[
    ("pool", "任务池", Category::Unstarted),
    ("pending", "待执行", Category::Unstarted),
    ("running", "执行中", Category::Started),
    ("failed", "执行失败", Category::Started),
    ("paused", "暂停中", Category::Started),
    ("completed", "任务完成", Category::Done),
    ("archived", "任务归档", Category::Closed),
];

const BUILT_IN_WORKFLOW_SPECS: &[WorkflowSpec] = &[
    WorkflowSpec { id: "generic_task", label: "通用任务", statuses: GENERIC_TASK_STATUSES },
    WorkflowSpec {
        id: "dev_lifecycle", label: "开发生命周期", statuses: DEV_LIFECYCLE_STATUSES
    },
    WorkflowSpec {
        id: "content_publishing",
        label: "内容创作与多平台发布",
        statuses: CONTENT_PUBLISHING_STATUSES,
    },
    WorkflowSpec {
        id: "marketing_campaign",
        label: "营销活动执行",
        statuses: MARKETING_CAMPAIGN_STATUSES,
    },
    WorkflowSpec {
        id: "customer_support",
        label: "客服工单处理",
        statuses: CUSTOMER_SUPPORT_STATUSES,
    },
    WorkflowSpec {
        id: "recruiting_pipeline", label: "招聘流程", statuses: RECRUITING_STATUSES
    },
];

pub fn built_in_workflows() -> Vec<TaskLifecycleWorkflowDto> {
    BUILT_IN_WORKFLOW_SPECS.iter().map(workflow_from_spec).collect()
}

pub fn built_in_workflow_by_id(id: &str) -> Option<TaskLifecycleWorkflowDto> {
    BUILT_IN_WORKFLOW_SPECS.iter().find(|workflow| workflow.id == id).map(workflow_from_spec)
}

pub fn is_built_in_workflow_id(id: &str) -> bool {
    BUILT_IN_WORKFLOW_SPECS.iter().any(|workflow| workflow.id == id)
}

fn workflow_from_spec(spec: &WorkflowSpec) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: spec.id.into(),
        label: spec.label.into(),
        built_in: true,
        initial_status_key: spec.statuses[0].0.into(),
        statuses: spec
            .statuses
            .iter()
            .map(|(key, label, category)| TaskLifecycleStatusDto {
                key: (*key).into(),
                label: (*label).into(),
                category: *category,
                enabled: true,
            })
            .collect(),
        steps: spec
            .statuses
            .iter()
            .map(|(key, _, _)| TaskLifecycleStepDto {
                status_key: (*key).into(),
                handoff: TaskLifecycleStepHandoffDto::KeepAssignee,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "builtin_tests.rs"]
mod builtin_tests;
