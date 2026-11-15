//! 验证新版技能来源标识的解析、规范化和显示契约。
//! 解析仅接受精确 skills.sh HTTPS 主机，并将目录名收敛到安全字符集合。

use super::*;

#[test]
fn parse_skills_sh_source_accepts_query_and_fragment() {
    let source =
        parse_skills_sh_source("https://skills.sh/acme/tools/review?version=1#readme").unwrap();
    assert_eq!(source.owner, "acme");
    assert_eq!(source.repo, "tools");
    assert_eq!(source.skill, "review");
    assert_eq!(source.github_repo_url(), "https://github.com/acme/tools.git");
}

#[test]
fn parse_skills_sh_source_rejects_unsafe_or_wrong_hosts() {
    assert!(parse_skills_sh_source("https://example.com/acme/tools/review").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/acme/../review").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/acme/tools/re\\view").is_none());
    assert!(parse_skills_sh_source("http://skills.sh/acme/tools/review").is_none());
    assert!(parse_skills_sh_source("https://Skills.sh/acme/tools/review").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/acme/tools").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/acme").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/acme/to\\ols/review").is_none());
    assert!(parse_skills_sh_source("https://skills.sh/ac\\me/tools/review").is_none());
}

#[test]
fn normalize_skills_sh_dir_name_keeps_only_safe_chars() {
    assert_eq!(normalize_skills_sh_dir_name("My Skill_01!"), "myskill_01");
    assert_eq!(normalize_skills_sh_dir_name("A-B.C/🙂"), "a-bc");
    assert!(is_skills_sh_source("https://skills.sh/acme/tools/review"));
    assert!(!is_skills_sh_source("not a source"));
}
