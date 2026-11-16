//! Prepare the skill directory without invoking host installers or policy storage.

use anyhow::Result;
use std::path::Path;

pub fn prepare_skills_dir(dir: &Path) -> Result<()> {
    // 创建技能目录及其所有父目录（如果不存在）
    std::fs::create_dir_all(dir)?;

    // 构建 README.md 文件路径
    let readme = dir.join("README.md");
    // 仅在 README.md 不存在时创建，避免覆盖用户已有的自定义内容
    if !readme.exists() {
        // 写入技能目录的说明文档，包含 SKILL.toml 和 SKILL.md 格式说明
        std::fs::write(
            &readme,
            "# VibeWindow Skills\n\n\
             Each subdirectory is a skill. Create a `SKILL.toml` or `SKILL.md` file inside.\n\n\
             ## SKILL.toml format\n\n\
             ```toml\n\
             [skill]\n\
             name = \"my-skill\"\n\
             description = \"What this skill does\"\n\
             version = \"0.1.0\"\n\
             author = \"your-name\"\n\
             tags = [\"productivity\", \"automation\"]\n\n\
             [[tools]]\n\
             name = \"my_tool\"\n\
             description = \"What this tool does\"\n\
             kind = \"shell\"\n\
             command = \"echo hello\"\n\
             ```\n\n\
             ## SKILL.md format (simpler)\n\n\
             Just write a markdown file with instructions for the agent.\n\
             The agent will read it and follow the instructions.\n\n\
             ## Installing community skills\n\n\
             ```bash\n\
             shiju skills install <source>\n\
             shiju skills list\n\
             ```\n",
        )?;
    }

    Ok(())
}

/// Prepare assets, preload builtins, then initialize policy; fail at the first error.
pub fn initialize_skills_dir(
    dir: &Path,
    preload: impl FnOnce(&Path) -> Result<()>,
    initialize_policy: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    prepare_skills_dir(dir)?;
    preload(dir)?;
    initialize_policy(dir)?;
    Ok(())
}

use crate::installer::ensure_builtin_preloaded_skills;
use crate::policy::load_or_init_skill_download_policy;
use crate::catalog::skills_dir;

/// Initialize the workspace skill assets, bundled skills and download policy.
pub fn init_skills_dir(workspace_dir: &Path) -> Result<()> {
    let dir = skills_dir(workspace_dir);
    initialize_skills_dir(
        &dir,
        ensure_builtin_preloaded_skills,
        |path| load_or_init_skill_download_policy(path).map(|_| ()),
    )
}
