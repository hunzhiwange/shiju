//! 验证任务集合能够等待已生成任务并正确报告空队列。

use super::task_set::TaskSet;

#[tokio::test]
async fn joins_spawned_tasks() {
    let mut tasks = TaskSet::new();
    tasks.spawn(async { 7 });

    assert_eq!(tasks.join_next().await.unwrap().unwrap(), 7);
    assert!(tasks.try_join_next().is_none());
}
