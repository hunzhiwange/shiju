//! Private client credentials are committed by the existing project transaction.
use super::mode_apply::{ClaudeMode, ModeApplyError};
use super::{
    live::{
        commit::*,
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    },
    mode_state::{self, ClientConfiguration},
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    ffi::OsStr,
};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

impl ClaudeMode<'_> {
    /// Recover only a plain mode intent belonging to this authorized project.
    /// Account and aggregation intents stay with their respective owners.
    pub fn recover_route(&mut self, project: &ProjectScope) -> Result<(), ModeApplyError> {
        if self.scope.app != AppId::Claude
            || self.scope.tenant != project.tenant_scope()
            || self.transactions.execution_host() != &ExecutionHostId::new(self.scope.host)?
        {
            return Err(ModeApplyError::Ownership);
        }
        let pending = self.transactions.pending(AppId::Claude)?.ok_or(WriteError::Conflict)?;
        if pending.tenant != *self.scope.tenant
            || pending.target.project_scope.as_ref() != Some(project)
        {
            return Err(ModeApplyError::Ownership);
        }
        if pending.version != 7
            || pending.target.mode.is_none()
            || pending.target.pointer.is_some()
            || pending.target.stack.is_some()
            || pending.target.claude_managed_login.is_some()
            || pending.target.kimi_binding.is_some()
            || pending
                .target
                .mode
                .as_ref()
                .and_then(|mode| mode.contract.as_ref())
                .is_some_and(|contract| contract.claude_login_operation.is_some())
        {
            return Err(WriteError::Conflict.into());
        }
        let _ = self.transactions.recover_project(project, &mut self.file.guard)?;
        Ok(())
    }

    /// The route owner holds its publication lock until this commit settles.
    /// The target comes from an authorized saved row and contains a private local
    /// credential only when attaching. Caller validation runs before publication.
    pub fn commit_route(
        &mut self,
        project: &ProjectScope,
        expected: &AppState,
        action: &vw_api_types::app_providers::mode_apply::ModeApplyAction,
        target: &Value,
        validate: impl Fn() -> bool,
    ) -> Result<bool, ModeApplyError> {
        if self.scope.app != AppId::Claude
            || self.scope.tenant != project.tenant_scope()
            || self.transactions.execution_host() != &ExecutionHostId::new(self.scope.host)?
        {
            return Err(ModeApplyError::Ownership);
        }
        mode_state::observe(expected, self.scope.tenant).map_err(|_| ModeApplyError::Ownership)?;
        if expected.project_scope.as_ref().is_some_and(|owner| owner != project)
            || expected.pointer.as_ref().is_none_or(|p| p.tenant != *self.scope.tenant)
            || expected.claude_managed_login.is_some()
        {
            return Err(ModeApplyError::Ownership);
        }
        if expected.stack.enabled
            || super::claude_login::configuration_has_pending_login(&self.file.guard)?
            || self.transactions.pending(AppId::Claude)?.is_some()
        {
            return Err(WriteError::Conflict.into());
        }
        use vw_api_types::app_providers::mode_apply::ModeApplyAction;
        let provider = match action {
            ModeApplyAction::Proxy { provider_id } => Some(provider_id.as_str()),
            _ => None,
        };
        let contract = ClientConfiguration::saved(AppId::Claude, target, false)
            .map_err(|_| ModeApplyError::InvalidInput)?
            .contract();
        let next = ModeState {
            mode: if provider.is_some()
                || matches!(action, ModeApplyAction::Detach {})
                    && expected.mode.as_ref().is_some_and(|m| m.mode == Mode::Proxy)
            {
                Mode::Proxy
            } else {
                Mode::Direct
            },
            attached: provider.is_some(),
            proxy_route: provider
                .map(str::to_owned)
                .or_else(|| expected.mode.as_ref().and_then(|m| m.proxy_route.clone())),
            contract: Some(contract),
        };
        let previous = expected
            .mode
            .as_ref()
            .and_then(|m| m.contract.as_ref())
            .map(|c| json!({"env":c.exclusive}));
        let changed = Cell::new(false);
        let planned = RefCell::new(None);
        let patch = |pre: Option<&[u8]>| {
            let after = super::claude::patch_with_residue(pre, previous.as_ref(), target, false)?;
            changed.set(Some(after.as_slice()) != pre);
            *planned.borrow_mut() = Some(after.clone());
            Ok(after)
        };
        self.transactions.commit_project_checked(
            project,
            &mut self.file.guard,
            expected,
            expected.project_scope.is_none(),
            &[FileChange {
                name: OsStr::new(self.file.name),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            PendingTarget { mode: Some(next), ..Default::default() },
            || {
                if validate() { Ok(()) } else { Err(WriteError::Conflict) }
            },
        )?;
        let published = self.file.guard.read(OsStr::new(self.file.name))?;
        // External writers may ignore the guard. Never activate a credential
        // against bytes different from the transaction's last staged projection.
        if published.as_ref() != planned.borrow().as_ref() {
            return Err(WriteError::Conflict.into());
        }
        self.file.bytes = published;
        Ok(changed.get())
    }
}
