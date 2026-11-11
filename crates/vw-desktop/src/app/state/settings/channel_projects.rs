//! 通道实例默认项目候选及请求生命周期。
use super::ChannelsSettingsState;

impl ChannelsSettingsState {
    pub(crate) fn begin_project_choices(&mut self, tenant: &str, key: &str) -> u64 {
        self.project_choices_generation = self.project_choices_generation.wrapping_add(1);
        self.project_choices_request =
            Some((tenant.into(), key.into(), self.project_choices_generation));
        self.project_choices.clear();
        self.project_choices_error = None;
        self.project_choices_loading = true;
        self.project_choices_generation
    }

    pub(crate) fn finish_project_choices(
        &mut self,
        tenant: &str,
        key: &str,
        generation: u64,
        result: Result<Vec<(String, String)>, String>,
    ) -> bool {
        if self.project_choices_request.as_ref() != Some(&(tenant.into(), key.into(), generation)) {
            return false;
        }
        self.project_choices_loading = false;
        match result {
            Ok(projects) => {
                self.project_choices = projects;
                self.project_choices_error = None;
            }
            Err(error) => {
                self.project_choices.clear();
                self.project_choices_error = Some(error);
            }
        }
        true
    }

    pub(crate) fn select_default_project(
        &mut self,
        tenant: &str,
        key: &str,
        project: Option<&str>,
    ) -> Result<(), &'static str> {
        if !self
            .project_choices_request
            .as_ref()
            .is_some_and(|(scope, instance, _)| scope == tenant && instance == key)
        {
            return Err("项目选择已过期，请重新加载当前实例的项目");
        }
        if let Some(project) = project {
            if self.project_choices_loading
                || self.project_choices_error.is_some()
                || !self.project_choices.iter().any(|(id, _)| id == project)
            {
                return Err("项目不存在或候选尚未加载，请重新加载项目");
            }
            self.instance_default_projects.insert(key.into(), project.into());
        } else {
            self.instance_default_projects.remove(key);
        }
        Ok(())
    }

    pub(crate) fn begin_execution_choices(&mut self, tenant: &str, instance: &str) -> u64 {
        self.execution_generation = self.execution_generation.wrapping_add(1);
        self.execution_request.insert(instance.into(), (tenant.into(), self.execution_generation));
        self.execution_provider.remove(instance);
        self.execution_choices.remove(instance);
        self.execution_error.remove(instance);
        self.execution_generation
    }

    pub(crate) fn finish_execution_choices(
        &mut self,
        tenant: &str,
        instance: &str,
        generation: u64,
        result: Result<super::ChannelExecutionChoices, String>,
    ) -> bool {
        if self.execution_request.get(instance) != Some(&(tenant.into(), generation)) {
            return false;
        }
        match result {
            Ok(choices) => {
                self.execution_choices.insert(instance.into(), choices);
            }
            Err(error) => {
                self.execution_error.insert(instance.into(), error);
            }
        }
        true
    }

    pub(crate) fn invalidate_execution_choices(&mut self) {
        self.execution_provider.clear();
        self.execution_request.clear();
        self.execution_choices.clear();
        self.execution_error.clear();
    }

    pub(crate) fn invalidate_project_choices(&mut self) {
        self.invalidate_execution_choices();
        self.project_choices_request = None;
        self.project_choices.clear();
        self.project_choices_error = None;
        self.project_choices_loading = false;
    }
}
