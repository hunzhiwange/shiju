//! 技能子系统测试入口，集中声明下载策略、加载和脚本相关测试模块。

mod download_policy;
mod git_source;
mod init;
mod load_skills;
mod md_skill;
mod prompt;
mod skills_sh;
mod toml_skill;

use crate::policy::{host_matches_trusted_domain, resolve_skill_source_alias};
use crate::source::{parse_skills_sh_source, normalize_skills_sh_dir_name};
use crate::types::SkillDownloadPolicy;
use crate::install::is_git_source;
use crate::*;
