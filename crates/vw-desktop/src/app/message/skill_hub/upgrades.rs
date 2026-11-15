//! 升级控制由领域库持有，桌面只映射任务及效果。
#[cfg(test)]
use super::{App, SkillInstallSelection};
pub use desktop_expert_hub::message::skill_hub::upgrades::FunctionUpgradeReport;
#[cfg(test)]
pub(super) fn main_selections(app: &App) -> Vec<SkillInstallSelection> { desktop_expert_hub::message::skill_hub::upgrades::main_selections(&app.expert_hub) }
