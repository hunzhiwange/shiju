use std::path::{Path, PathBuf};

#[test]
fn path_helpers_build_expected_runtime_locations() {
    let home = Path::new("/tmp/vibewindow-home");
    let config_dir = home.join(super::HOME_CONFIG_DIR_NAME);

    assert_eq!(super::home_config_dir(home), config_dir);
    assert_eq!(super::global_database_path(home), config_dir.join("database/vibe-window.sqlite3"));
    assert_eq!(super::global_database_config_path(home), config_dir.join("database/config.json"));
    assert_eq!(super::task_worktrees_dir(home), config_dir.join(super::TASK_WORKTREES_DIR_NAME));
    assert_eq!(super::root_config_dir(), PathBuf::from("/").join(super::HOME_CONFIG_DIR_NAME));
}

#[test]
fn task_plan_storage_dir_uses_build_specific_home_directory() {
    let home = Path::new("/tmp/vibewindow-home");

    assert_eq!(
        super::task_plan_storage_dir(home),
        home.join(super::HOME_CONFIG_DIR_NAME).join("storage/plan")
    );
}

#[test]
fn tilde_config_path_normalizes_empty_and_leading_slash_inputs() {
    assert_eq!(super::tilde_config_path(""), format!("~/{}", super::HOME_CONFIG_DIR_NAME));
    assert_eq!(
        super::tilde_config_path("///skills"),
        format!("~/{}/skills", super::HOME_CONFIG_DIR_NAME)
    );
}

#[test]
fn fixed_state_paths_match_build_profile_constants() {
    assert_eq!(super::estop_state_file_path(), super::ESTOP_STATE_FILE_PATH);
}
