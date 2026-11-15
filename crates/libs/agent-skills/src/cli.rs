//! Filesystem actions used by the host skill command dispatcher.

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use config_skills::SkillSecurityConfig;

#[cfg(not(target_arch = "wasm32"))]
pub fn audit_skill(source: String, skills_path: &Path, security: &SkillSecurityConfig) -> Result<()> {
    // 解析源路径：可能是绝对路径或技能名称
    let source_path = PathBuf::from(&source);

    // 确定审计目标：优先使用实际路径，否则在技能目录中查找
    let target = if source_path.exists() {
        source_path
    } else {
        skills_path.join(&source)
    };

    // 验证目标是否存在
    if !target.exists() {
        anyhow::bail!("Skill source or installed skill not found: {source}");
    }

    // 执行技能安全审计
    let report = crate::audit::audit_skill_directory_with_security(&target, security)?;

    // 如果审计通过（无安全问题），显示成功信息
    if report.is_clean() {
        println!(
            "  {} Skill audit passed for {} ({} files scanned).",
            console::style("✓").green().bold(),
            target.display(),
            report.files_scanned
        );
        return Ok(());
    }

    // 审计未通过，显示所有发现的问题
    println!(
        "  {} Skill audit failed for {}",
        console::style("✗").red().bold(),
        target.display()
    );
    for finding in report.findings {
        println!("    - {finding}");
    }
    anyhow::bail!("Skill audit failed.");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove_skill(name: String, skills_path: &Path) -> Result<()> {
    // 安全校验：拒绝路径遍历攻击尝试
    // 禁止在技能名称中包含父目录引用或路径分隔符
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        anyhow::bail!("Invalid skill name: {name}");
    }

    // 构建技能的完整路径
    let skill_path = skills_path.join(&name);

    // 安全校验：确保解析后的路径实际位于技能目录内部
    // 通过规范化路径来检测潜在的路径逃逸攻击
    let canonical_skills =
        skills_path.canonicalize().unwrap_or_else(|_| skills_path.to_path_buf());
    if let Ok(canonical_skill) = skill_path.canonicalize() {
        // 验证技能路径是否以技能目录为前缀
        if !canonical_skill.starts_with(&canonical_skills) {
            anyhow::bail!("Skill path escapes skills directory: {name}");
        }
    }

    // 验证技能是否存在
    if !skill_path.exists() {
        anyhow::bail!("Skill not found: {name}");
    }

    // 递归删除技能目录及其所有内容
    std::fs::remove_dir_all(&skill_path)?;
    println!("  {} Skill '{}' removed.", console::style("✓").green().bold(), name);
    Ok(())
}

/// A borrowed projection of the fields printed by the skills CLI.
pub struct SkillListEntry<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub description: &'a str,
    pub tools: Vec<&'a str>,
    pub tags: &'a [String],
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print_skill_list(workspace_dir: &Path, skills: &[SkillListEntry<'_>]) -> Result<()> {
    // 如果没有安装任何技能，显示帮助信息
    if skills.is_empty() {
        println!("No skills installed.");
        println!();
        println!(
            "  Create one: mkdir -p {}",
            workspace_dir.join("skills").join("my-skill").display()
        );
        println!(
            "              echo '# My Skill' > {}",
            workspace_dir.join("skills").join("my-skill").join("SKILL.md").display()
        );
        println!();
        println!("  Or install: shiju skills install <source>");
    } else {
        // 显示已安装技能的统计信息
        println!("Installed skills ({}):", skills.len());
        println!();

        // 遍历并显示每个技能的详细信息
        for skill in skills {
            // 显示技能名称、版本和描述
            println!(
                "  {} {} — {}",
                console::style(&skill.name).white().bold(),
                console::style(format!("v{}", skill.version)).dim(),
                skill.description
            );

            // 如果技能包含工具，显示工具列表
            if !skill.tools.is_empty() {
                println!(
                    "    Tools: {}",
                    skill.tools.join(", ")
                );
            }

            // 如果技能有标签，显示标签列表
            if !skill.tags.is_empty() {
                println!("    Tags:  {}", skill.tags.join(", "));
            }
        }
    }
    println!();
    Ok(())
}

/// Keep the host's initializer and installer while owning command ordering and errors.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_skill(
    source: &str,
    skills_path: &Path,
    initialize: impl FnOnce() -> Result<()>,
    install: impl FnOnce(&str, &Path) -> Result<()>,
) -> Result<()> {
    println!("Installing skill from: {source}");
    initialize()?;
    std::fs::create_dir_all(skills_path).map_err(|err| {
        anyhow::anyhow!("failed to create {}: {err}", skills_path.display())
    })?;
    install(source, skills_path)?;
    println!("  Security audit completed successfully.");
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print_install_success(message: &str, installed_dir: &Path, files_scanned: usize) {
    println!(
        "  {} {message}: {} ({} files scanned)",
        console::style("✓").green().bold(),
        installed_dir.display(),
        files_scanned
    );
}
