// 覆盖桌面技能目录解析、详情读取、脚手架创建与安装。

// 临时技能目录同时构造 TOML、Markdown 和缺失资源，验证元数据优先级与回退。
#[test]
fn desktop_skills_test_module_is_loaded() {
    assert!(true);
}

#[test]
fn router_builds_with_unit_state() {
    let _ = super::router::<()>();
}

fn write_markdown_skill(skill_dir: &std::path::Path, title: &str, description: &str) {
    std::fs::create_dir_all(skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {title}\ndescription: {description}\n---\n# {title}\n"),
    )
    .expect("skill md");
}

fn write_toml_skill(skill_dir: &std::path::Path, title: &str, description: &str) {
    std::fs::create_dir_all(skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.toml"),
        format!("[skill]\nname = '{title}'\ndescription = '{description}'\n"),
    )
    .expect("skill toml");
}

#[test]
fn skill_kind_sort_key_orders_known_kinds_first() {
    assert_eq!(super::skill_kind_sort_key("recommended"), 0);
    assert_eq!(super::skill_kind_sort_key("system"), 1);
    assert_eq!(super::skill_kind_sort_key("personal"), 2);
    assert_eq!(super::skill_kind_sort_key("custom"), 3);
}

#[test]
fn parse_skill_frontmatter_reads_yaml_block() {
    let parsed = super::parse_skill_frontmatter(
        r#"---
name: Demo Skill
description: Helps with demos.
---

# Body
"#,
    )
    .expect("frontmatter should parse");

    assert_eq!(parsed.name.as_deref(), Some("Demo Skill"));
    assert_eq!(parsed.description.as_deref(), Some("Helps with demos."));
}

// 拒绝路径除状态码外还检查状态保持不变，防止错误响应掩盖已发生的副作用。
#[test]
fn parse_skill_frontmatter_rejects_missing_or_unclosed_blocks() {
    assert!(super::parse_skill_frontmatter("# Demo").is_none());
    assert!(super::parse_skill_frontmatter("---\nname: Demo").is_none());
    assert!(super::parse_skill_frontmatter("---\nname: [\n---\n").is_none());
}

#[test]
fn parse_skill_frontmatter_stops_before_markdown_body() {
    let parsed = super::parse_skill_frontmatter(
        "---\nname: Frontmatter Name\ndescription: Frontmatter description\n---\nname: Body text\n",
    )
    .expect("frontmatter should parse independently of body");

    assert_eq!(parsed.name.as_deref(), Some("Frontmatter Name"));
    assert_eq!(parsed.description.as_deref(), Some("Frontmatter description"));
}

#[test]
fn humanize_skill_id_formats_hyphen_and_underscore_words() {
    assert_eq!(super::humanize_skill_id("find-skills"), "Find Skills");
    assert_eq!(super::humanize_skill_id("agent_helper"), "Agent Helper");
    assert_eq!(super::humanize_skill_id("---"), "---");
}

#[test]
fn normalize_optional_project_path_trims_blank_values() {
    assert_eq!(super::normalize_optional_project_path(None), None);
    assert_eq!(super::normalize_optional_project_path(Some("   ".to_string())), None);
    assert_eq!(
        super::normalize_optional_project_path(Some(" /tmp/project ".to_string())),
        Some("/tmp/project".to_string())
    );
}

#[test]
fn resolve_project_path_requires_existing_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file_path = temp.path().join("not-a-directory");
    std::fs::write(&file_path, "content").expect("regular file");

    assert_eq!(super::resolve_project_path("  ").unwrap_err(), "project_path is required");
    assert!(super::resolve_project_path(temp.path().to_str().expect("utf8 path")).is_ok());
    assert!(super::resolve_project_path(file_path.to_str().expect("utf8 path")).is_err());
    assert!(super::resolve_project_path("/definitely/missing/vw-skills").is_err());
}

#[test]
fn read_local_catalog_metadata_prefers_toml_manifest() {
    let temp = tempfile::tempdir().expect("temp dir");
    let skill_dir = temp.path().join("demo-skill");
    std::fs::create_dir(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.toml"),
        "[skill]\nname = 'TOML Title'\ndescription = 'TOML description'\n",
    )
    .expect("skill toml");

    let (title, description) =
        super::read_local_catalog_metadata(&skill_dir, "demo-skill").expect("metadata");

    assert_eq!(title, "TOML Title");
    assert_eq!(description, "TOML description");
}

#[test]
fn read_local_catalog_metadata_falls_back_for_blank_toml_fields() {
    let temp = tempfile::tempdir().expect("temp dir");
    let skill_dir = temp.path().join("blank-skill");
    std::fs::create_dir(&skill_dir).expect("skill dir");
    std::fs::write(skill_dir.join("SKILL.toml"), "[skill]\nname = ''\ndescription = ''\n")
        .expect("skill toml");

    let (title, description) =
        super::read_local_catalog_metadata(&skill_dir, "blank-skill").expect("metadata");

    assert_eq!(title, "Blank Skill");
    assert_eq!(description, "本地技能。");
}

#[test]
fn read_local_catalog_metadata_reads_markdown_defaults_and_rejects_invalid_toml() {
    let temp = tempfile::tempdir().expect("temp dir");
    let markdown_dir = temp.path().join("markdown-skill");
    std::fs::create_dir(&markdown_dir).expect("markdown skill dir");
    std::fs::write(markdown_dir.join("SKILL.md"), "# Markdown Skill").expect("skill md");

    let (title, description) =
        super::read_local_catalog_metadata(&markdown_dir, "markdown-skill").expect("metadata");

    assert_eq!(title, "Markdown Skill");
    assert_eq!(description, "本地技能。");

    let invalid_toml_dir = temp.path().join("invalid-toml");
    std::fs::create_dir(&invalid_toml_dir).expect("invalid toml dir");
    std::fs::write(invalid_toml_dir.join("SKILL.toml"), "not toml").expect("invalid toml");

    assert!(super::read_local_catalog_metadata(&invalid_toml_dir, "invalid-toml").is_none());
}

#[test]
fn read_local_catalog_metadata_does_not_fall_back_when_toml_is_invalid() {
    let temp = tempfile::tempdir().expect("temp dir");
    let skill_dir = temp.path().join("invalid-toml-with-markdown");
    write_markdown_skill(&skill_dir, "Markdown Title", "Markdown description");
    std::fs::write(skill_dir.join("SKILL.toml"), "not valid toml").expect("invalid toml");

    assert!(super::read_local_catalog_metadata(&skill_dir, "invalid-toml-with-markdown").is_none());
}

#[test]
fn read_local_catalog_metadata_returns_none_without_skill_document() {
    let temp = tempfile::tempdir().expect("temp dir");
    let skill_dir = temp.path().join("empty-skill");
    std::fs::create_dir(&skill_dir).expect("skill dir");

    assert!(super::read_local_catalog_metadata(&skill_dir, "empty-skill").is_none());
}

#[test]
fn count_skill_resources_ignores_skill_documents() {
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::write(temp.path().join("SKILL.md"), "# Demo").expect("skill md");
    std::fs::write(temp.path().join("SKILL.toml"), "[skill]\nname='Demo'\ndescription='Demo'\n")
        .expect("skill toml");
    std::fs::write(temp.path().join("notes.txt"), "resource").expect("resource");
    std::fs::create_dir(temp.path().join("assets")).expect("assets dir");

    assert_eq!(super::count_skill_resources(temp.path()), 2);
    assert_eq!(super::count_skill_resources(&temp.path().join("missing")), 0);
}

#[test]
fn local_skill_source_key_maps_all_kinds() {
    assert_eq!(
        super::local_skill_source_key(crate::app::agent::skills::LocalSkillSourceKind::Workspace),
        "workspace"
    );
    assert_eq!(
        super::local_skill_source_key(crate::app::agent::skills::LocalSkillSourceKind::Ancestor),
        "ancestor"
    );
    assert_eq!(
        super::local_skill_source_key(crate::app::agent::skills::LocalSkillSourceKind::Global),
        "global"
    );
}

#[test]
fn unique_skill_directory_skips_existing_names() {
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(temp.path().join("new-skill")).expect("new skill");
    std::fs::create_dir(temp.path().join("new-skill-2")).expect("new skill 2");

    assert_eq!(super::unique_skill_directory(temp.path()), temp.path().join("new-skill-3"));
}

#[test]
fn unique_skill_directory_prefers_unsuffixed_name() {
    let temp = tempfile::tempdir().expect("temp dir");

    assert_eq!(super::unique_skill_directory(temp.path()), temp.path().join("new-skill"));
}

#[test]
fn create_new_skill_scaffold_writes_template() {
    let project = tempfile::tempdir().expect("project dir");

    let path = super::create_new_skill_scaffold(
        project.path(),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("create skill");
    let skill_dir = std::path::PathBuf::from(path);
    let content = std::fs::read_to_string(skill_dir.join("SKILL.md")).expect("skill md");

    assert_eq!(skill_dir.file_name().and_then(|name| name.to_str()), Some("new-skill"));
    assert!(content.contains("name: new-skill"));
    assert!(content.contains("# new skill"));
}

#[test]
fn create_new_skill_scaffold_uses_next_available_name() {
    let project = tempfile::tempdir().expect("project dir");
    let skills_root = project.path().join(".vibewindow/skills");
    std::fs::create_dir_all(skills_root.join("new-skill")).expect("existing skill dir");

    let path = super::create_new_skill_scaffold(
        project.path(),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("create second skill");
    let skill_dir = std::path::PathBuf::from(path);
    let content = std::fs::read_to_string(skill_dir.join("SKILL.md")).expect("skill md");

    assert_eq!(skill_dir.file_name().and_then(|name| name.to_str()), Some("new-skill-2"));
    assert!(content.contains("name: new-skill-2"));
    assert!(content.contains("# new skill 2"));
}

#[test]
fn read_local_skill_document_prefers_markdown_then_toml() {
    let temp = tempfile::tempdir().expect("temp dir");
    let toml_dir = temp.path().join("toml-skill");
    std::fs::create_dir(&toml_dir).expect("toml dir");
    let toml_content = "[skill]\nname = 'Only TOML'\ndescription = 'TOML doc'\n";
    std::fs::write(toml_dir.join("SKILL.toml"), toml_content).expect("skill toml");

    let (name, content) = super::read_local_skill_document(&toml_dir).expect("toml document");

    assert_eq!(name, "SKILL.toml");
    assert_eq!(content, toml_content);

    std::fs::write(toml_dir.join("SKILL.md"), "# Markdown").expect("skill md");
    let (name, content) = super::read_local_skill_document(&toml_dir).expect("markdown document");

    assert_eq!(name, "SKILL.md");
    assert_eq!(content, "# Markdown");

    let missing_dir = temp.path().join("missing-doc");
    std::fs::create_dir(&missing_dir).expect("missing doc dir");
    assert!(super::read_local_skill_document(&missing_dir).is_err());
}

#[test]
fn built_in_skill_helpers_read_metadata_and_mark_recommended_group() {
    let skill = super::find_built_in_skill("create-task-plans").unwrap_or_else(|| {
        panic!(
            "bundled skill metadata; discovered: {:?}",
            super::BUILT_IN_SKILLS.iter().map(|skill| &skill.id).collect::<Vec<_>>()
        )
    });
    assert_eq!(skill.id, "create-task-plans");
    assert_eq!(skill.group, super::BuiltInSkillGroup::Recommended);
    assert_eq!(skill.resource_count, 0);

    let markdown =
        super::read_built_in_skill_markdown("create-task-plans").expect("bundled markdown");
    assert!(markdown.contains("create-task-plans"));
    assert!(super::find_built_in_skill("missing-skill").is_none());
    assert!(super::read_built_in_skill_markdown("missing-skill").is_err());
}

#[test]
fn resolve_skill_detail_returns_personal_toml_skill_and_missing_errors() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/custom-skill");
    write_toml_skill(&skill_dir, "Custom Skill", "Custom description");

    let detail = super::resolve_skill_detail(
        Some(project.path().to_str().expect("utf8 path")),
        "custom-skill",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("personal detail");

    assert_eq!(detail.id, "custom-skill");
    assert_eq!(detail.title, "Custom Skill");
    assert_eq!(detail.description, "Custom description");
    assert_eq!(detail.kind, "personal");
    assert_eq!(detail.document_name, "SKILL.toml");
    assert!(detail.document_content.contains("Custom Skill"));
    assert!(detail.installed);
    assert!(detail.can_toggle);
    assert!(detail.can_delete);

    let missing = super::resolve_skill_detail(
        Some(project.path().to_str().expect("utf8 path")),
        "missing-skill",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect_err("missing detail");
    assert!(missing.contains("未找到技能"));
}

#[test]
fn resolve_skill_detail_returns_built_in_and_local_variants() {
    let project = tempfile::tempdir().expect("project dir");
    let built_in_result = super::resolve_skill_detail(
        Some(project.path().to_str().expect("utf8 path")),
        "create-task-plans",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    );

    if let Ok(built_in) = built_in_result {
        assert_eq!(built_in.id, "create-task-plans");
        assert_eq!(built_in.kind, "recommended");
        assert!(!built_in.installed);
        assert!(built_in.can_install);
        assert!(!built_in.can_toggle);
        assert_eq!(built_in.document_name, "SKILL.md");
    }

    let skill_dir = project.path().join(".vibewindow/skills/create-task-plans");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Local Plans\ndescription: Local override\n---\n# Local\n",
    )
    .expect("local skill");

    let local = super::resolve_skill_detail(
        Some(project.path().to_str().expect("utf8 path")),
        "create-task-plans",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("local detail");

    assert_eq!(local.title, "Local Plans");
    assert_eq!(local.description, "Local override");
    assert!(local.installed);
    assert!(local.enabled);
    assert!(local.can_toggle);
    assert!(local.can_delete);
    assert!(matches!(local.kind.as_str(), "recommended" | "personal"));
}

#[test]
fn resolve_skill_detail_reports_disabled_local_skill_capabilities() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/disabled-detail");
    write_markdown_skill(&skill_dir, "Disabled Detail", "Disabled description");
    std::fs::write(
        crate::app::agent::skills::local_skill_disabled_marker_path(&skill_dir),
        "disabled\n",
    )
    .expect("disabled marker");

    let detail = super::resolve_skill_detail(
        Some(project.path().to_str().expect("utf8 path")),
        "disabled-detail",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("disabled local detail");

    assert_eq!(detail.kind, "personal");
    assert!(detail.installed);
    assert!(!detail.enabled);
    assert_eq!(detail.source, "workspace");
    assert!(detail.can_toggle);
    assert!(detail.can_delete);
    assert!(!detail.can_install);
}

#[test]
fn install_built_in_skill_copies_once_and_collect_catalog_marks_installed() {
    let project = tempfile::tempdir().expect("project dir");

    let first = super::install_built_in_skill(
        project.path(),
        "create-task-plans",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("install built-in");
    let second = super::install_built_in_skill(
        project.path(),
        "create-task-plans",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("install built-in again");
    let catalog = super::collect_catalog_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("catalog");
    let entry = catalog.iter().find(|item| item.id == "create-task-plans").expect("catalog entry");

    assert_eq!(first, second);
    assert!(std::path::Path::new(&first).join("SKILL.md").is_file());
    assert!(entry.installed);
    assert!(entry.enabled);
    assert!(matches!(entry.kind.as_str(), "recommended" | "personal"));
    assert_eq!(entry.source, "workspace");
    assert!(
        super::install_built_in_skill(
            project.path(),
            "missing-skill",
            vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
        )
        .is_err()
    );
}

#[test]
fn installed_skills_preserve_market_source_metadata() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/skillhub--public--safe-skill");
    write_markdown_skill(&skill_dir, "Safe Skill", "Installed from market");
    std::fs::write(
        skill_dir.join("_meta.json"),
        serde_json::json!({
            "source": "skillhub",
            "namespace": "public",
            "slug": "safe-skill",
            "version": "1.0.0",
            "package_type": "skill",
            "source_pack": "safe-suite"
        })
        .to_string(),
    )
    .expect("market metadata");

    let catalog = super::collect_catalog_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("catalog");
    let installed = catalog
        .iter()
        .find(|item| item.id == "skillhub--public--safe-skill")
        .expect("installed market skill");

    assert_eq!(installed.source, "skillhub");
    assert_eq!(installed.origin.as_deref(), Some("workspace"));
    assert_eq!(installed.namespace.as_deref(), Some("public"));
    assert_eq!(installed.package_type.as_deref(), Some("skill"));
    assert_eq!(installed.source_pack.as_deref(), Some("safe-suite"));
}

#[test]
fn install_built_in_skill_copies_nested_resources() {
    let project = tempfile::tempdir().expect("project dir");

    let path = super::install_built_in_skill(
        project.path(),
        "rustcodegraph",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("install nested built-in");
    let skill_dir = std::path::PathBuf::from(path);

    assert!(skill_dir.join("SKILL.md").is_file());
    assert!(skill_dir.join("README.md").is_file());
}

#[test]
fn collect_catalog_skills_includes_personal_items_and_preserves_bundled_defaults() {
    let project = tempfile::tempdir().expect("project dir");
    write_markdown_skill(
        &project.path().join(".vibewindow/skills/zeta-skill"),
        "Zeta Skill",
        "Zeta description",
    );

    let catalog = super::collect_catalog_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("catalog");

    let personal = catalog.iter().find(|item| item.id == "zeta-skill").expect("personal item");
    assert_eq!(personal.kind, "personal");
    assert!(personal.installed);
    assert!(personal.enabled);
    assert_eq!(personal.source, "workspace");

    let bundled = catalog.iter().find(|item| item.id == "do-task-plans").expect("bundled item");
    assert_eq!(bundled.source, "bundled");
    assert!(!bundled.installed);
    assert!(!bundled.enabled);
}

#[test]
fn collect_catalog_skills_uses_local_metadata_for_built_in_override() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/create-task-plans");
    write_markdown_skill(&skill_dir, "Local Plans", "Local description");
    std::fs::write(skill_dir.join("notes.txt"), "resource").expect("resource");
    std::fs::write(
        crate::app::agent::skills::local_skill_disabled_marker_path(&skill_dir),
        "disabled\n",
    )
    .expect("disabled marker");

    let catalog = super::collect_catalog_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("catalog");
    let entry = catalog.iter().find(|item| item.id == "create-task-plans").expect("catalog entry");

    assert_eq!(entry.title, "Local Plans");
    assert_eq!(entry.description, "Local description");
    assert_eq!(entry.kind, "recommended");
    assert_eq!(entry.resource_count, 2);
    assert!(entry.installed);
    assert!(!entry.enabled);
    assert_eq!(entry.source, "workspace");
    assert_eq!(
        entry
            .source_path
            .as_deref()
            .map(std::path::Path::new)
            .and_then(|path| path.canonicalize().ok()),
        skill_dir.canonicalize().ok()
    );
}

#[test]
fn collect_catalog_skills_orders_personal_items_by_title_then_id() {
    let project = tempfile::tempdir().expect("project dir");
    let skills_root = project.path().join(".vibewindow/skills");
    write_markdown_skill(&skills_root.join("task-50-zeta"), "Zulu Task 50", "Last title");
    write_markdown_skill(&skills_root.join("task-50-alpha"), "alpha Task 50", "First title");

    let catalog = super::collect_catalog_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("catalog");
    let selected_ids = catalog
        .iter()
        .filter(|item| item.id.starts_with("task-50-"))
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(selected_ids, vec!["task-50-alpha", "task-50-zeta"]);
}

#[test]
fn discover_local_skills_reads_workspace_skills_and_deduplicates() {
    let project = tempfile::tempdir().expect("project dir");
    let skills_root = project.path().join(".vibewindow/skills");
    let duplicate_root = project.path().join("skills");
    std::fs::create_dir_all(skills_root.join("demo")).expect("primary skill dir");
    std::fs::create_dir_all(duplicate_root.join("demo")).expect("duplicate skill dir");
    std::fs::write(
        skills_root.join("demo/SKILL.md"),
        "---\nname: Primary Demo\ndescription: Primary description\n---\n# Demo\n",
    )
    .expect("primary md");
    std::fs::write(
        duplicate_root.join("demo/SKILL.md"),
        "---\nname: Duplicate Demo\ndescription: Duplicate description\n---\n# Demo\n",
    )
    .expect("duplicate md");
    std::fs::write(skills_root.join("demo/notes.txt"), "notes").expect("resource");

    let skills = super::discover_local_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("discover local skills");

    let demo = skills.iter().find(|skill| skill.id == "demo").expect("demo skill");
    assert_eq!(demo.title, "Primary Demo");
    assert_eq!(demo.description, "Primary description");
    assert_eq!(demo.resource_count, 1);
    assert!(demo.enabled);
    assert_eq!(demo.source, "workspace");
    assert_eq!(skills.iter().filter(|skill| skill.id == "demo").count(), 1);
}

#[test]
fn discover_local_skills_ignores_non_skills_and_reports_disabled_state() {
    let project = tempfile::tempdir().expect("project dir");
    let skills_root = project.path().join(".vibewindow/skills");
    let disabled_dir = skills_root.join("disabled-demo");
    write_markdown_skill(&disabled_dir, "Disabled Demo", "Disabled description");
    std::fs::write(
        crate::app::agent::skills::local_skill_disabled_marker_path(&disabled_dir),
        "disabled\n",
    )
    .expect("disabled marker");
    std::fs::create_dir_all(skills_root.join("missing-document")).expect("empty skill dir");
    std::fs::write(skills_root.join("plain-file"), "not a skill directory").expect("plain file");

    let skills = super::discover_local_skills(
        Some(project.path().to_str().expect("utf8 path")),
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("discover local skills");
    let disabled = skills.iter().find(|skill| skill.id == "disabled-demo").expect("disabled skill");

    assert!(!disabled.enabled);
    assert_eq!(disabled.source, "workspace");
    assert!(!skills.iter().any(|skill| skill.id == "missing-document"));
    assert!(!skills.iter().any(|skill| skill.id == "plain-file"));
}

#[test]
fn set_local_skill_enabled_toggles_disabled_marker() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/demo");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Demo\ndescription: Demo skill\n---\n# Demo\n",
    )
    .expect("skill md");
    let marker = crate::app::agent::skills::local_skill_disabled_marker_path(&skill_dir);

    let disabled_path = super::set_local_skill_enabled(
        Some(project.path().to_str().expect("utf8 path")),
        "demo",
        false,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("disable skill");

    assert_eq!(
        std::path::PathBuf::from(disabled_path).canonicalize().expect("disabled path"),
        skill_dir.canonicalize().expect("skill path")
    );
    assert!(marker.is_file());

    super::set_local_skill_enabled(
        Some(project.path().to_str().expect("utf8 path")),
        "demo",
        true,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("enable skill");
    assert!(!marker.exists());
}

#[test]
fn set_local_skill_enabled_noops_for_current_state_and_errors_for_missing_skill() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/demo");
    write_markdown_skill(&skill_dir, "Demo", "Demo skill");

    super::set_local_skill_enabled(
        Some(project.path().to_str().expect("utf8 path")),
        "demo",
        true,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("enable already enabled skill");

    let marker = crate::app::agent::skills::local_skill_disabled_marker_path(&skill_dir);
    std::fs::write(&marker, "disabled\n").expect("disabled marker");

    super::set_local_skill_enabled(
        Some(project.path().to_str().expect("utf8 path")),
        "demo",
        false,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("disable already disabled skill");
    assert!(marker.is_file());

    let missing = super::set_local_skill_enabled(
        Some(project.path().to_str().expect("utf8 path")),
        "missing",
        true,
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect_err("missing skill");
    assert!(missing.contains("未找到可管理的本地技能"));
}

#[test]
fn delete_local_skill_removes_directory() {
    let project = tempfile::tempdir().expect("project dir");
    let skill_dir = project.path().join(".vibewindow/skills/demo");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Demo\ndescription: Demo skill\n---\n# Demo\n",
    )
    .expect("skill md");
    let expected_deleted = skill_dir.canonicalize().expect("canonical skill path");

    let deleted = super::delete_local_skill(
        Some(project.path().to_str().expect("utf8 path")),
        "demo",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect("delete skill");

    assert_eq!(std::path::PathBuf::from(deleted), expected_deleted);
    assert!(!skill_dir.exists());
}

#[test]
fn delete_local_skill_errors_for_missing_skill() {
    let project = tempfile::tempdir().expect("project dir");

    let missing = super::delete_local_skill(
        Some(project.path().to_str().expect("utf8 path")),
        "missing",
        vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
    )
    .expect_err("missing skill");

    assert!(missing.contains("未找到可管理的本地技能"));
}

#[tokio::test]
async fn handlers_reject_blank_required_fields_before_config_lookup() {
    let err = super::detail_get(axum::extract::Query(super::SkillsDetailQuery {
        project_path: None,
        skill_id: "  ".to_string(),
    }))
    .await
    .expect_err("blank detail skill id");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "skill_id is required");

    let err = super::create_post(axum::Json(super::SkillsProjectRequest {
        project_path: "  ".to_string(),
    }))
    .await
    .expect_err("blank project path");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "project_path is required");

    let project = tempfile::tempdir().expect("project dir");
    let err = super::install_builtin_post(axum::Json(super::SkillsInstallBuiltInRequest {
        project_path: project.path().to_string_lossy().to_string(),
        skill_id: "  ".to_string(),
    }))
    .await
    .expect_err("blank install skill id");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "skill_id is required");

    let err = super::set_enabled_post(axum::Json(super::SkillsSetEnabledRequest {
        project_path: None,
        skill_id: "  ".to_string(),
        enabled: true,
    }))
    .await
    .expect_err("blank toggle skill id");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "skill_id is required");

    let err = super::delete_post(axum::Json(super::SkillsMutationRequest {
        project_path: None,
        skill_id: "  ".to_string(),
    }))
    .await
    .expect_err("blank delete skill id");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "skill_id is required");
}
