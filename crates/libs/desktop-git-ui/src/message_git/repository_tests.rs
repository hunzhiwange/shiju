use super::{update as domain_update, RepositoryMessage as GitMessage, Effect};
use crate::git_panel::repository_ops::{RepositoryState, RepositorySnapshot, HistoryDetail};
fn update(state: &mut RepositoryState, message: GitMessage) {
    let path = state.repo_path.clone();
    let _ = domain_update(state, &mut None, path, false, Err("unused gateway".into()), &mut Vec::<Effect>::new(), message);
}
#[test]
fn history_results_ignore_old_generation_and_old_commit_selection() {
    let mut state = RepositoryState::default();
    let project = tempfile::tempdir().expect("temporary repository");
    let _repo = git2::Repository::init(project.path()).expect("init repository");
    let path = project.path().to_string_lossy().into_owned();
    state.repo_path = Some(path.clone());
    state.generation = 2;
    state.loading = true;
    let _task = update(
        &mut state,
        GitMessage::RepositoryReady {
            path: path.clone(),
            generation: 1,
            result: Ok(RepositorySnapshot { branch: Some("stale".into()), ..Default::default() }),
        },
    );
    assert!(state.loading);
    assert!(state.snapshot.branch.is_none());
    state.selected = Some("newer".into());
    state.detail_loading = true;
    let _task = update(
        &mut state,
        GitMessage::HistoryDetailReady {
            path,
            generation: 2,
            sha: "older".into(),
            result: Ok(HistoryDetail { message: "stale".into(), files: vec![] }),
        },
    );
    assert!(state.detail_loading);
    assert!(state.detail.is_none());
}
#[test]
fn history_file_selection_expands_one_file_and_closes_others() {
    let mut state = RepositoryState::default();
    let _ = update(&mut state, GitMessage::SelectHistoryFile(0));
    assert_eq!(state.selected_file, Some(0));
    let _ = update(&mut state, GitMessage::SelectHistoryFile(1));
    assert_eq!(state.selected_file, Some(1));
    let _ = update(&mut state, GitMessage::SelectHistoryFile(1));
    assert_eq!(state.selected_file, None);
}
#[test]
fn remote_animation_advances_only_while_syncing() {
    let mut state = RepositoryState::default();
    let _ = update(&mut state, GitMessage::RemoteAnimationTick);
    assert_eq!(state.remote_animation_frame, 0);
    state.remote_busy = true;
    let _ = update(&mut state, GitMessage::RemoteAnimationTick);
    assert_eq!(state.remote_animation_frame, 1);
    state.remote_busy = false;
    let _ = update(&mut state, GitMessage::RemoteAnimationTick);
    assert_eq!(state.remote_animation_frame, 1);
}
