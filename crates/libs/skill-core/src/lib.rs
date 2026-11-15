//! 技能审计、内置来源常量与远程发现。

pub mod audit;
pub mod constants;
pub mod discovery;
pub mod policy;

pub use policy::host_matches_trusted_domain;

pub mod install;
pub mod loader;
pub mod prompt;
pub mod source;
pub mod types;

pub use install::{handle_command, init_skills_dir};
pub use loader::{load_skills, load_skills_with_config};
pub use prompt::{skills_dir, skills_to_prompt, skills_to_prompt_with_mode};
pub use types::{Skill, SkillCommands, SkillRuntimeConfig, SkillTool, SkillsConfig, SkillsPromptInjectionMode};

pub mod project;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod symlink_tests;
