use super::*;
use std::{fs, os::unix::fs::symlink};

fn read_native_directory_with(
    home: &Path,
    configured_dir: Option<&Path>,
    config: Option<&str>,
    selected: Option<&str>,
    inherited: Option<&str>,
    agent: Option<&OsStr>,
) -> Result<OmpNativeDirectory, ProviderReadUnavailable> {
    let location = resolve_location(home, configured_dir, config, selected, inherited, agent)
        .map_err(|_| ProviderReadUnavailable::InvalidConfiguration)?;
    read_native_directory_at(home, location)
}

#[test]
fn aggregation_t37_omp_read_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let work = home.path().join(".custom/profiles/work/agent");
    let inherited = home.path().join(".custom/profiles/inherited/agent");
    let default = home.path().join(".custom/agent");
    let configured = home.path().join("configured");
    for directory in [&work, &inherited, &default, &configured] {
        fs::create_dir_all(directory).unwrap();
        fs::write(directory.join("models.yml"), "providers: {custom: {}}\n").unwrap();
    }
    let settings = b"modelRoles:\n  default: native/vendor/model-a\n";
    fs::write(work.join("config.yml"), settings).unwrap();
    fs::write(work.join("config.yaml"), "modelRoles: {default: ignored/ignored}\n").unwrap();
    fs::write(work.join("models.yaml"), "providers: {ignored: {}}\n").unwrap();
    // A dangling login link must be left alone. The read only needs models and settings.
    let auth = work.join("auth.json");
    let auth_target = home.path().join("missing-login.json");
    symlink(&auth_target, &auth).unwrap();
    let login_db = work.join("agent.db");
    fs::write(&login_db, b"opaque native login fixture").unwrap();

    let read = |configured_dir, selected, inherited_profile, agent: Option<&Path>| {
        read_native_directory_with(
            home.path(),
            configured_dir,
            Some(".custom"),
            selected,
            inherited_profile,
            agent.map(Path::as_os_str),
        )
        .unwrap()
    };
    let snapshot = read(None, Some("work"), Some("inherited"), Some(&inherited));
    assert_eq!(snapshot.members, ["custom"]);
    assert_eq!(snapshot.default_provider_id.as_deref(), Some("native"));
    assert_eq!(snapshot.default_model_id.as_deref(), Some("vendor/model-a"));
    assert_eq!(snapshot.profile.as_deref(), Some("work"));
    assert!(!snapshot.source_revision.is_empty());
    assert_eq!(read(None, None, Some("work"), None), snapshot);
    assert_eq!(fs::read(work.join("config.yml")).unwrap(), settings);
    assert_eq!(fs::read_link(auth).unwrap(), auth_target);
    assert!(!auth_target.exists());
    assert_eq!(fs::read(login_db).unwrap(), b"opaque native login fixture");

    fs::write(work.join("config.yml"), "modelRoles: {default: native/new}\n").unwrap();
    let updated = read(None, Some("work"), None, None);
    assert_ne!(updated.source_revision, snapshot.source_revision);
    assert_eq!(updated.default_model_id.as_deref(), Some("new"));

    fs::write(default.join("config.yaml"), "modelRoles: {default: fallback/model}\n").unwrap();
    let explicit_default = read(None, Some("default"), Some("inherited"), Some(&inherited));
    assert_eq!(explicit_default.profile, None);
    assert_eq!(explicit_default.default_provider_id.as_deref(), Some("fallback"));
    assert_eq!(read(None, Some(""), Some("inherited"), Some(&inherited)), explicit_default);
    assert_ne!(
        explicit_default.source_revision,
        read(None, None, Some("inherited"), None).source_revision
    );

    fs::write(configured.join("config.yml"), "modelRoles: {default: override/model}\n").unwrap();
    let override_snapshot = read(Some(&configured), Some("work"), None, None);
    assert_eq!(override_snapshot.default_provider_id.as_deref(), Some("override"));
    assert_eq!(override_snapshot.profile.as_deref(), Some("work"));
    assert_ne!(
        override_snapshot.source_revision,
        read(Some(&configured), Some("other"), None, None).source_revision
    );
    assert_eq!(
        read(None, None, None, Some(&configured)).default_provider_id.as_deref(),
        Some("override")
    );
}

#[test]
fn aggregation_t37_omp_read_rejects_unsafe_sources() {
    use ProviderReadUnavailable::{InvalidConfiguration, Unreadable, UnsupportedFormat};
    let home = tempfile::tempdir().unwrap();
    let read = |configured_dir, selected, config| {
        read_native_directory_with(home.path(), configured_dir, config, selected, None, None)
    };
    for profile in ["../escape", "work/other", "UPPER", "nul.txt", "trailing."] {
        assert_eq!(read(None, Some(profile), None), Err(InvalidConfiguration));
    }
    let configured = home.path().join("configured");
    assert_eq!(read(Some(&configured), Some("../escape"), None), Err(InvalidConfiguration));
    assert_eq!(read(None, None, Some("../escape")), Err(InvalidConfiguration));
    let outside = tempfile::tempdir().unwrap();
    assert_eq!(read(Some(outside.path()), None, None), Err(InvalidConfiguration));

    let directory = home.path().join(".omp/agent");
    let empty = read(None, None, None).unwrap();
    assert!(empty.members.is_empty());
    assert!(!directory.exists());

    fs::create_dir_all(directory.parent().unwrap()).unwrap();
    symlink(outside.path(), &directory).unwrap();
    assert_eq!(read(None, None, None), Err(Unreadable));
    fs::remove_file(&directory).unwrap();

    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("models.json"), "{}").unwrap();
    assert_eq!(read(None, None, None), Err(UnsupportedFormat));
    fs::write(directory.join("models.yml"), "providers: {}").unwrap();
    let settings = directory.join("config.yml");
    symlink(outside.path().join("outside.yml"), &settings).unwrap();
    assert_eq!(read(None, None, None), Err(Unreadable));
    fs::remove_file(&settings).unwrap();
    fs::write(settings, "modelRoles: {default: [malformed]}\n").unwrap();
    assert_eq!(read(None, None, None), Err(InvalidConfiguration));
}
