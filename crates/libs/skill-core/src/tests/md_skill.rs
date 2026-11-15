//! Markdown 技能加载测试，覆盖只有标题或正文较少的 SKILL.md 文件。

use super::*;
use std::fs;

#[test]
fn md_skill_heading_only() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("heading-only");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(skill_dir.join("SKILL.md"), "# Just a Heading\n").unwrap();

    let skills = load_skills(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].description, "No description");
}
