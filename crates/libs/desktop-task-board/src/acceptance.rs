//! Native companion rendering data.
pub fn items(count: usize) -> Vec<crate::pet_state::TaskPetItem> {
    (0..count)
        .map(|index| crate::pet_state::TaskPetItem {
            request_id: index as u64 + 1,
            session_id: format!("companion-acceptance-{index}"),
            title: format!("中文原生渲染验收任务 {}：检查动态长标题与叠层卡片边界", index + 1),
            detail: "正在读取上下文并整理结果；完成后可以展开详情、关闭卡片或打开回复。".into(),
            detail_feedback: None,
            project: Some("隔离验收项目".into()),
            status: if index % 2 == 0 {
                crate::pet_state::TaskPetStatus::Running
            } else {
                crate::pet_state::TaskPetStatus::Completed
            },
            last_click_at: None,
        })
        .collect()
}
