//! 验证运行目录辅助函数、构建配置常量及波浪号路径归一化结果。
//! 所有断言只检查纯路径拼接，不依赖宿主机目录是否存在。

use std::path::{Path, PathBuf};

#[test]
fn home_config_directory_names_match_agent_layout() {
    assert_eq!(super::PROD_HOME_CONFIG_DIR_NAME, ".shiju-agent");
    assert_eq!(super::DEV_HOME_CONFIG_DIR_NAME, ".shiju-agent-dev");
}

#[test]
fn platform_directory_names_match_shiju_layout() {
    assert_eq!(super::PROD_APP_DIR_NAME, "shiju");
    assert_eq!(super::DEV_APP_DIR_NAME, "shiju-dev");
    assert_eq!(super::APP_DIR_NAME, if cfg!(debug_assertions) { "shiju-dev" } else { "shiju" });
}

// 基础辅助函数必须共享同一构建隔离目录，防止数据落入不同根路径。
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

// 任务计划是用户级持久数据，其位置必须跟随当前构建的配置目录。
#[test]
fn task_plan_storage_dir_uses_build_specific_home_directory() {
    let home = Path::new("/tmp/vibewindow-home");

    assert_eq!(
        super::task_plan_storage_dir(home),
        home.join(super::HOME_CONFIG_DIR_NAME).join("storage/plan")
    );
}

// 接受带斜杠的调用方输入，但输出始终保持单一 `~/配置目录` 前缀。
#[test]
fn tilde_config_path_normalizes_empty_and_leading_slash_inputs() {
    assert_eq!(super::tilde_config_path(""), format!("~/{}", super::HOME_CONFIG_DIR_NAME));
    assert_eq!(
        super::tilde_config_path("///skills"),
        format!("~/{}/skills", super::HOME_CONFIG_DIR_NAME)
    );
}

// 编译期常量和运行时辅助函数必须指向同一个紧急停止状态文件。
#[test]
fn fixed_state_paths_match_build_profile_constants() {
    assert_eq!(super::estop_state_file_path(), super::ESTOP_STATE_FILE_PATH);
}
