use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩包直接安装或卸载的阶段状态。
pub enum ShijuDirectInstallState {
    #[default]
    Idle,
    Fetching {
        generation: u64,
        package_id: super::ShijuPackageId,
    },
    Installing {
        generation: u64,
        package_id: super::ShijuPackageId,
    },
    Uninstalling {
        generation: u64,
    },
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 一次手动下载使用的代次和目标平台包。
pub struct ShijuManualDownloadState {
    pub generation: u64,
    pub package_id: super::ShijuPackageId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 在操作开始时冻结的安装目标选择。
pub struct ShijuSelectionOperation {
    pub generation: u64,
    pub selections: Vec<super::SkillInstallSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 安装向导的目标选择、在途操作与本地安装探测状态。
pub struct ShijuInstallGuideState {
    pub open: bool,
    pub current_package: Option<super::ShijuPackageId>,
    pub agent_package: LoadState<super::ShijuPackageRelease>,
    pub agent_generation: u64,
    pub manual_download: Option<ShijuManualDownloadState>,
    pub manual_generation: u64,
    pub direct_install: ShijuDirectInstallState,
    pub direct_generation: u64,
    pub direct_selections: Vec<super::SkillInstallSelection>,
    pub installation_generation: u64,
    pub install_scope: super::SkillInstallScope,
    pub selected_project_path: Option<String>,
    pub selected_targets: Vec<super::SkillInstallTarget>,
    pub installations: Vec<super::ShijuTargetInstallation>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub login_sync: Option<ShijuSelectionOperation>,
    pub login_sync_generation: u64,
    pub login_sync_in_progress: bool,
    pub uninstall_confirmation_open: bool,
}

impl ShijuInstallGuideState {
    /// 使全部向导请求代次失效，并清除在途操作但保留用户选择。
    pub fn reset_requests(&mut self) {
        self.agent_generation = self.agent_generation.wrapping_add(1);
        self.manual_generation = self.manual_generation.wrapping_add(1);
        self.direct_generation = self.direct_generation.wrapping_add(1);
        self.installation_generation = self.installation_generation.wrapping_add(1);
        self.login_sync_generation = self.login_sync_generation.wrapping_add(1);
        self.agent_package = LoadState::Idle;
        self.manual_download = None;
        self.direct_install = ShijuDirectInstallState::Idle;
        self.direct_selections.clear();
        self.login_sync = None;
        self.login_sync_in_progress = false;
        self.uninstall_confirmation_open = false;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.reset_requests();
    }

    pub fn start_agent_request(&mut self) -> u64 {
        self.agent_generation = self.agent_generation.wrapping_add(1);
        let generation = self.agent_generation;
        self.agent_package = LoadState::Loading { generation };
        generation
    }

    pub fn start_manual_download(
        &mut self,
        package_id: super::ShijuPackageId,
    ) -> Option<u64> {
        if self.manual_download.is_some() {
            return None;
        }
        self.manual_generation = self.manual_generation.wrapping_add(1);
        let generation = self.manual_generation;
        self.manual_download = Some(ShijuManualDownloadState { generation, package_id });
        Some(generation)
    }

    /// 冻结当前非空目标选择并进入包获取阶段；忙碌时拒绝重复启动。
    pub fn start_direct_install(
        &mut self,
        package_id: super::ShijuPackageId,
    ) -> Option<u64> {
        if matches!(
            &self.direct_install,
            ShijuDirectInstallState::Fetching { .. }
                | ShijuDirectInstallState::Installing { .. }
                | ShijuDirectInstallState::Uninstalling { .. }
        ) {
            return None;
        }
        let selections = self.selected_install_selections();
        if selections.is_empty() {
            return None;
        }
        self.direct_generation = self.direct_generation.wrapping_add(1);
        let generation = self.direct_generation;
        self.direct_selections = selections;
        self.direct_install = ShijuDirectInstallState::Fetching { generation, package_id };
        Some(generation)
    }

    pub fn start_uninstall(&mut self) -> Option<u64> {
        if self.installed_selection_count() == 0
            || matches!(
                &self.direct_install,
                ShijuDirectInstallState::Fetching { .. }
                    | ShijuDirectInstallState::Installing { .. }
                    | ShijuDirectInstallState::Uninstalling { .. }
            )
        {
            return None;
        }
        let selections = self.selected_install_selections();
        self.direct_generation = self.direct_generation.wrapping_add(1);
        let generation = self.direct_generation;
        self.direct_selections = selections;
        self.direct_install = ShijuDirectInstallState::Uninstalling { generation };
        Some(generation)
    }

    pub fn start_installation_load(&mut self) -> u64 {
        self.installation_generation = self.installation_generation.wrapping_add(1);
        self.installation_generation
    }

    pub fn start_login_sync(&mut self) -> Option<ShijuSelectionOperation> {
        if self.login_sync.is_some() || self.installed_selection_count() == 0 {
            return None;
        }
        self.login_sync_generation = self.login_sync_generation.wrapping_add(1);
        let operation = ShijuSelectionOperation {
            generation: self.login_sync_generation,
            selections: self.selected_install_selections(),
        };
        self.login_sync = Some(operation.clone());
        self.login_sync_in_progress = true;
        Some(operation)
    }

    pub fn is_busy(&self) -> bool {
        self.login_sync.is_some()
            || matches!(
                &self.direct_install,
                ShijuDirectInstallState::Fetching { .. }
                    | ShijuDirectInstallState::Installing { .. }
                    | ShijuDirectInstallState::Uninstalling { .. }
            )
    }

    pub fn select_scope(&mut self, scope: super::SkillInstallScope) -> bool {
        if self.is_busy() || self.install_scope == scope {
            return false;
        }
        self.install_scope = scope;
        self.refresh_selected_installation_status();
        true
    }

    /// 切换目标但至少保留一个选择，任何安装或同步在途时禁止修改。
    pub fn toggle_target(&mut self, target: super::SkillInstallTarget) -> bool {
        if self.is_busy() {
            return false;
        }
        if let Some(index) = self.selected_targets.iter().position(|selected| *selected == target) {
            if self.selected_targets.len() == 1 {
                return false;
            }
            self.selected_targets.remove(index);
        } else {
            self.selected_targets.push(target);
        }
        self.refresh_selected_installation_status();
        true
    }

    pub fn selected_install_selections(&self) -> Vec<super::SkillInstallSelection> {
        let mut selections = Vec::with_capacity(self.selected_targets.len());
        for target in &self.selected_targets {
            let selection =
                super::SkillInstallSelection { target: *target, scope: self.install_scope };
            if !selections.contains(&selection) {
                selections.push(selection);
            }
        }
        selections
    }

    pub fn installation(
        &self,
        scope: super::SkillInstallScope,
        target: super::SkillInstallTarget,
    ) -> Option<&super::ShijuInstallation> {
        self.installations
            .iter()
            .find(|item| item.destination.scope == scope && item.destination.target == target)
            .and_then(|item| item.installation.as_ref())
    }

    pub fn installed_selection_count(&self) -> usize {
        self.selected_install_selections()
            .iter()
            .filter(|selection| self.installation(selection.scope, selection.target).is_some())
            .count()
    }

    pub fn set_installations(&mut self, report: super::ShijuInstallationsReport) {
        self.installations = report.targets;
        self.refresh_selected_installation_status();
    }

    pub fn apply_install_report(&mut self, report: super::ShijuMultiInstallReport) {
        for install in report.installs {
            let installation = super::ShijuInstallation {
                installed_dir: install.destination.installed_dir.clone(),
                version: install.version,
            };
            if let Some(current) = self.installations.iter_mut().find(|current| {
                current.destination.scope == install.destination.scope
                    && current.destination.target == install.destination.target
            }) {
                current.destination = install.destination;
                current.installation = Some(installation);
            } else {
                self.installations.push(super::ShijuTargetInstallation {
                    destination: install.destination,
                    installation: Some(installation),
                });
            }
        }
        self.refresh_selected_installation_status();
    }

    pub fn apply_uninstall_report(&mut self, report: super::ShijuUninstallReport) {
        for removed in report.targets {
            if !removed.removed {
                continue;
            }
            if let Some(current) = self.installations.iter_mut().find(|current| {
                current.destination.scope == removed.destination.scope
                    && current.destination.target == removed.destination.target
            }) {
                current.installation = None;
            }
        }
        self.refresh_selected_installation_status();
    }

    fn refresh_selected_installation_status(&mut self) {
        let installed = self
            .selected_install_selections()
            .into_iter()
            .filter_map(|selection| {
                self.installation(selection.scope, selection.target)
                    .map(|installation| installation.version.clone())
            })
            .collect::<Vec<_>>();
        self.is_installed = !installed.is_empty();
        self.installed_version = installed.into_iter().flatten().next();
        if !self.is_busy() {
            self.direct_install = if self.is_installed {
                ShijuDirectInstallState::Installed
            } else {
                ShijuDirectInstallState::Idle
            };
        }
    }

    pub fn set_installation(&mut self, installation: Option<super::ShijuInstallation>) {
        self.is_installed = installation.is_some();
        self.installed_version = installation.and_then(|installation| installation.version);
        self.direct_install = if self.is_installed {
            ShijuDirectInstallState::Installed
        } else {
            ShijuDirectInstallState::Idle
        };
    }
}

impl Default for ShijuInstallGuideState {
    fn default() -> Self {
        Self {
            open: false,
            current_package: super::ShijuPackageId::current(),
            agent_package: LoadState::Idle,
            agent_generation: 0,
            manual_download: None,
            manual_generation: 0,
            direct_install: ShijuDirectInstallState::Idle,
            direct_generation: 0,
            direct_selections: Vec::new(),
            installation_generation: 0,
            install_scope: super::SkillInstallScope::Global,
            selected_project_path: None,
            selected_targets: vec![super::SkillInstallTarget::VibeWindow],
            installations: Vec::new(),
            is_installed: false,
            installed_version: None,
            login_sync: None,
            login_sync_generation: 0,
            login_sync_in_progress: false,
            uninstall_confirmation_open: false,
        }
    }
}
