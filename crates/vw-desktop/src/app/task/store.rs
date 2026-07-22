//! 任务存储模块的公共出口。
//!
//! 该文件只负责组织子模块并重新导出上层需要的存储 API，避免调用方依赖内部文件布局。

mod artifacts;
mod operations;
mod paths;
mod persistence;

pub use artifacts::{
    save_task_code_review_result_artifact, save_task_execution_result_artifact,
    write_task_code_review_result_log, write_task_execution_result_log,
};
pub use operations::{
    create_task, load_tasks_by_status, rebuild_failed_task_from_current_branch,
    reorder_tasks_in_status, update_task,
};
pub use persistence::{
    delete_task_file, load_all_tasks, load_index, load_task, rebuild_index_from_task_files,
    save_index, save_task,
};

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
