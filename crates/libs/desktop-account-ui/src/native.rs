use crate::message::AccountMessage;
use desktop_skill_hub::{download_native_shiju_skill, prepare_native_shiju_skill};
use directories::BaseDirs;
use iced::Task;
pub fn start(state: desktop_skill_hub::NativeShijuSkillState) -> Task<AccountMessage> {
    let Some((generation, revision)) = state.begin() else {
        return Task::none();
    };
    Task::perform(
        async move {
            let home = BaseDirs::new().ok_or_else(|| "无法确定用户主目录".to_string())?;
            prepare_native_shiju_skill(
                state,
                home.home_dir().to_path_buf(),
                download_native_shiju_skill,
            )
            .await
            .map_err(|_| "识矩原生技能准备失败".to_string())
        },
        move |result| AccountMessage::NativeShijuSkillPrepared { generation, revision, result },
    )
}

/// Ignore superseded preparations and retry when the account credential changed.
pub enum Completion {
    Ignore,
    Retry,
    Ready(Result<desktop_skill_hub::NativeShijuSkillPrepared, String>),
}
pub fn finished(
    state: &desktop_skill_hub::NativeShijuSkillState,
    generation: u64,
    revision: u64,
    result: Result<desktop_skill_hub::NativeShijuSkillPrepared, String>,
) -> Completion {
    if !state.finish(generation) {
        return Completion::Ignore;
    }
    let completed_revision = result.as_ref().map_or(revision, |prepared| prepared.revision);
    if state.revision() != completed_revision {
        return Completion::Retry;
    }
    Completion::Ready(result)
}
