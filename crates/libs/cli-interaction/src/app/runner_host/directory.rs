//! 目录切换完成后的会话提交与旧请求失效。

use crate::app::{ActiveRunner, V3DisplayEvent, V3GatewayApp, push_runner_error};

impl ActiveRunner {
    pub(super) fn finish_directory_switch(
        &mut self,
        app: &mut V3GatewayApp,
        generation: u64,
        source_directory: std::path::PathBuf,
        recovery_draft: String,
        result: Result<Box<crate::app::DirectorySwitchBootstrap>, String>,
    ) {
        if generation != self.directory_switch_generation {
            return;
        }
        self.directory_switch_source = None;
        if app.runtime().directory() != source_directory {
            return;
        }
        match result {
            Ok(bootstrap) => {
                self.file_search_generation =
                    self.file_search_generation.wrapping_add(1);
                self.file_search_query = None;
                self.session_diff_generation =
                    self.session_diff_generation.wrapping_add(1);
                self.doctor_generation = self.doctor_generation.wrapping_add(1);
                self.cron_generation = self.cron_generation.wrapping_add(1);
                self.task_detail_generation =
                    self.task_detail_generation.wrapping_add(1);
                self.workspace_files_generation =
                    self.workspace_files_generation.wrapping_add(1);
                self.hooks_generation = self.hooks_generation.wrapping_add(1);
                self.memory_generation = self.memory_generation.wrapping_add(1);
                self.memory_mutation_generation = None;
                self.memory_snapshot_directory = None;
                self.memory_snapshot_query = None;
                self.hook_catalog_directory = None;
                self.hook_catalog = None;
                self.workflow_run = None;
                let directory = bootstrap.runtime.directory().display().to_string();
                app.commit_directory_switch(*bootstrap);
                app.widget_mut().push_history(V3DisplayEvent::System {
                    text: format!("工作目录已切换到 {directory}"),
                });
            }
            Err(error) => {
                if app.widget().draft().is_empty() {
                    app.widget_mut().replace_draft(recovery_draft.as_str());
                }
                push_runner_error(
                    app,
                    format!("failed to switch working directory: {error}"),
                );
            }
        }
    }
}
