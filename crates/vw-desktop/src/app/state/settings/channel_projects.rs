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
    ) -> Result<(), String> {
        if !self
            .project_choices_request
            .as_ref()
            .is_some_and(|(scope, instance, _)| scope == tenant && instance == key)
        {
            return Err("项目选择已过期，请重新加载当前实例的项目".into());
        }
        if let Some(project) = project {
            if self.project_choices_loading
                || self.project_choices_error.is_some()
                || !self.project_choices.iter().any(|(id, _)| id == project)
            {
                return Err("项目不存在或候选尚未加载，请重新加载项目".into());
            }
            self.instance_default_projects.insert(key.into(), project.into());
        } else {
            self.instance_default_projects.remove(key);
        }
        Ok(())
    }

    pub(crate) fn invalidate_project_choices(&mut self) {
        self.project_choices_request = None;
        self.project_choices.clear();
        self.project_choices_error = None;
        self.project_choices_loading = false;
    }
}
