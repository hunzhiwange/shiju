use vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_api_types::staff::{
    StaffAutoSchedulePlanRequest, StaffAutoSchedulePlanResponse, StaffCommandRequest,
    StaffCommandResponse,
    StaffProjectCapabilityCatalogRequest, StaffProjectCapabilityCatalogResponse,
    StaffProjectOrchestrationRequest, StaffProjectOrchestrationResponse, StaffProjectionResponse,
    StaffStageCapabilitiesResponse, StaffWorkflowOrchestrationRequest,
    StaffWorkflowOrchestrationResponse,
};

use super::GatewayClient;

const STAFF_STAGE_CAPABILITIES_PATH: &str = "/v1/staff/stage-capabilities";

impl GatewayClient {
    /// 拉取 staff 阶段能力注册表。
    pub async fn staff_stage_capabilities(&self) -> Result<StaffStageCapabilitiesResponse, String> {
        self.get_json(STAFF_STAGE_CAPABILITIES_PATH, &[]).await
    }

    /// 拉取项目 staff projection。
    pub async fn staff_projection(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<StaffProjectionResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_projection_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 拉取项目能力目录。
    pub async fn staff_project_capabilities(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<StaffProjectCapabilityCatalogResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_project_capabilities_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 拉取项目 staff 编排状态。
    pub async fn staff_project_orchestration(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<StaffProjectOrchestrationResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_project_orchestration_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 保存项目 staff 编排状态。
    pub async fn save_staff_project_orchestration(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &StaffProjectOrchestrationRequest,
    ) -> Result<StaffProjectOrchestrationResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_project_orchestration_path(scope);
        self.put_json(&path, &[], request).await
    }

    /// 保存项目自定义能力目录。
    pub async fn save_staff_project_capabilities(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &StaffProjectCapabilityCatalogRequest,
    ) -> Result<StaffProjectCapabilityCatalogResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_project_capabilities_path(scope);
        self.put_json(&path, &[], request).await
    }

    /// 拉取工作流能力目录。
    pub async fn staff_workflow_capabilities(
        &self,
        scope: &AgentSchedulerScopeDto,
        workflow_id: &str,
    ) -> Result<StaffProjectCapabilityCatalogResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_workflow_capabilities_path(scope, workflow_id);
        self.get_json(&path, &[]).await
    }

    /// 拉取工作流 staff 编排状态。
    pub async fn staff_workflow_orchestration(
        &self,
        scope: &AgentSchedulerScopeDto,
        workflow_id: &str,
    ) -> Result<StaffWorkflowOrchestrationResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_workflow_orchestration_path(scope, workflow_id);
        self.get_json(&path, &[]).await
    }

    /// 保存工作流 staff 编排状态。
    pub async fn save_staff_workflow_orchestration(
        &self,
        scope: &AgentSchedulerScopeDto,
        workflow_id: &str,
        request: &StaffWorkflowOrchestrationRequest,
    ) -> Result<StaffWorkflowOrchestrationResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_workflow_orchestration_path(scope, workflow_id);
        self.put_json(&path, &[], request).await
    }

    /// 保存工作流自定义能力目录。
    pub async fn save_staff_workflow_capabilities(
        &self,
        scope: &AgentSchedulerScopeDto,
        workflow_id: &str,
        request: &StaffProjectCapabilityCatalogRequest,
    ) -> Result<StaffProjectCapabilityCatalogResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_workflow_capabilities_path(scope, workflow_id);
        self.put_json(&path, &[], request).await
    }

    /// 校验并执行 staff command。
    pub async fn staff_command(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &StaffCommandRequest,
    ) -> Result<StaffCommandResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_command_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 生成项目 staff 自动调度计划。
    pub async fn staff_auto_schedule_plan(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &StaffAutoSchedulePlanRequest,
    ) -> Result<StaffAutoSchedulePlanResponse, String> {
        self.ensure_staff_tenant_scope(scope)?;
        let path = staff_auto_schedule_plan_path(scope);
        self.post_json(&path, &[], request).await
    }

    fn ensure_staff_tenant_scope(&self, scope: &AgentSchedulerScopeDto) -> Result<(), String> {
        if scope.project_scope().tenant_scope() != self.endpoint().tenant_scope() {
            return Err("tenant scope does not match gateway endpoint tenant scope".to_string());
        }
        Ok(())
    }
}

fn staff_project_path(scope: &AgentSchedulerScopeDto) -> String {
    format!(
        "/v1/staff/projects/{}",
        encode_path_segment(scope.project_scope().project_id().as_ref())
    )
}

fn staff_projection_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/projection", staff_project_path(scope))
}

fn staff_project_capabilities_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/capabilities", staff_project_path(scope))
}

fn staff_project_orchestration_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/orchestration", staff_project_path(scope))
}

fn staff_workflow_path(scope: &AgentSchedulerScopeDto, workflow_id: &str) -> String {
    format!("{}/workflows/{}", staff_project_path(scope), encode_path_segment(workflow_id))
}

fn staff_workflow_capabilities_path(scope: &AgentSchedulerScopeDto, workflow_id: &str) -> String {
    format!("{}/capabilities", staff_workflow_path(scope, workflow_id))
}

fn staff_workflow_orchestration_path(scope: &AgentSchedulerScopeDto, workflow_id: &str) -> String {
    format!("{}/orchestration", staff_workflow_path(scope, workflow_id))
}

fn staff_command_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/commands", staff_project_path(scope))
}

fn staff_auto_schedule_plan_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/auto-schedule-plan", staff_project_path(scope))
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "staff_api_tests.rs"]
mod staff_api_tests;
