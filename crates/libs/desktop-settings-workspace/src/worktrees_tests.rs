use crate::{messages::WorktreeSettingsMessage as Event, worktrees::apply};
use crate::state::{WorktreeSettingsIdentity as Identity, WorktreeSettingsState as State};
use config_worktree::WorktreeSettings;

fn identity() -> Identity {
    Identity {
        gateway: "http://fixture".into(),
        tenant: Default::default(),
        auth_fingerprint: [0; 32],
        project: Some("/project".into()),
    }
}
fn load(state: &mut State, current: &Identity, policy: WorktreeSettings) {
    apply(state, Event::Refresh, current);
    let generation = state.generation;
    apply(
        state,
        Event::Loaded {
            identity: current.clone(),
            generation,
            result: Ok((policy, "/server/default".into())),
        },
        current,
    );
}
#[test]
fn worktree_settings_ui_round_trips_policy() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    assert_eq!(state.default_root, "/server/default");
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::RootChanged("/工作树".into()), &id);
    apply(&mut state, Event::CountChanged("7".into()), &id);
    apply(&mut state, Event::FetchChanged(true), &id);
    apply(&mut state, Event::CleanupChanged(false), &id);
    apply(&mut state, Event::AskChanged(false), &id);
    let policy = apply(&mut state, Event::Save, &id).expect("validated save request");
    assert_eq!(
        policy,
        WorktreeSettings {
            root: "/工作树".into(),
            keep_count: 7,
            fetch_before_create: true,
            auto_cleanup_enabled: false,
            ask_before_delete: false
        }
    );
    let generation = state.generation;
    apply(
        &mut state,
        Event::Saved { identity: id.clone(), generation, result: Err("offline".into()) },
        &id,
    );
    assert_eq!(state.draft.as_ref().unwrap().policy.root, "/工作树");
    assert_eq!(state.saved, Some(WorktreeSettings::default()));
    apply(&mut state, Event::Save, &id);
    let generation = state.generation;
    apply(
        &mut state,
        Event::Saved { identity: id.clone(), generation, result: Ok(policy.clone()) },
        &id,
    );
    assert!(state.draft.is_none());
    load(&mut state, &id, policy.clone());
    assert_eq!(state.saved, Some(policy));
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::CountChanged("99".into()), &id);
    apply(&mut state, Event::Cancel, &id);
    apply(&mut state, Event::Edit, &id);
    assert_eq!(state.draft.unwrap().keep_count, "7");
}
#[test]
fn worktree_settings_ui_rejects_invalid_count() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Edit, &id);
    for count in ["0", "-1", "bad", "4294967296", ""] {
        apply(&mut state, Event::CountChanged(count.into()), &id);
        let generation = state.generation;
        assert!(apply(&mut state, Event::Save, &id).is_none());
        assert!(!state.saving);
        assert_eq!(state.generation, generation);
        assert!(state.error.is_some());
    }
}
#[test]
fn worktree_settings_ui_ignores_stale_load() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Refresh, &id);
    let old = state.generation;
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::RootChanged("/draft".into()), &id);
    apply(
        &mut state,
        Event::Loaded {
            identity: id.clone(),
            generation: old,
            result: Ok((WorktreeSettings::default(), "/server/default".into())),
        },
        &id,
    );
    assert_eq!(state.draft.as_ref().unwrap().policy.root, "/draft");
    apply(&mut state, Event::Refresh, &id);
    let generation = state.generation;
    apply(
        &mut state,
        Event::Loaded { identity: id.clone(), generation: old, result: Err("old".into()) },
        &id,
    );
    assert!(state.loading);
    let mut other = id.clone();
    other.project = Some("/other".into());
    apply(
        &mut state,
        Event::Loaded {
            identity: id.clone(),
            generation,
            result: Ok((WorktreeSettings::default(), "wrong".into())),
        },
        &other,
    );
    assert_ne!(state.default_root, "wrong");
    assert_eq!(state.draft.as_ref().unwrap().policy.root, "/draft");
}

#[test]
fn worktree_settings_ui_save_blocks_edits_and_stale_responses() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::CountChanged("4".into()), &id);
    let policy = apply(&mut state, Event::Save, &id).unwrap();
    let generation = state.generation;
    apply(&mut state, Event::Cancel, &id);
    apply(&mut state, Event::CountChanged("9".into()), &id);
    apply(&mut state, Event::Refresh, &id);
    assert_eq!(state.generation, generation);
    assert_eq!(state.draft.as_ref().unwrap().keep_count, "4");
    let mut other = id.clone();
    other.auth_fingerprint = [1; 32];
    apply(
        &mut state,
        Event::Saved { identity: id.clone(), generation, result: Ok(policy.clone()) },
        &other,
    );
    assert!(state.saving);
    apply(
        &mut state,
        Event::Saved {
            identity: id.clone(),
            generation: generation - 1,
            result: Ok(policy.clone()),
        },
        &id,
    );
    assert!(state.saving);
    apply(&mut state, Event::Saved { identity: id.clone(), generation, result: Ok(policy) }, &id);
    assert!(!state.saving);
    assert!(state.draft.is_none());
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::RootChanged("relative".into()), &id);
    assert!(apply(&mut state, Event::Save, &id).is_none());
    assert!(state.error.is_some());
    apply(&mut state, Event::Refresh, &other);
    assert!(state.draft.is_none());
    assert!(state.saved.is_none());
}


#[test]
fn worktree_settings_ui_project_round_trip_discards_prior_generation() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::RootChanged("/draft".into()), &id);
    apply(&mut state, Event::Refresh, &id);
    let old = state.generation;
    let mut other = id.clone();
    other.project = Some("/other".into());
    apply(&mut state, Event::ContextChanged, &other);
    apply(&mut state, Event::ContextChanged, &id);
    apply(&mut state, Event::Refresh, &id);
    let generation = state.generation;
    apply(
        &mut state,
        Event::Loaded {
            identity: id.clone(),
            generation: old,
            result: Ok((WorktreeSettings::default(), "stale".into())),
        },
        &id,
    );
    assert!(state.loading);
    apply(
        &mut state,
        Event::Loaded { identity: id.clone(), generation, result: Err("offline".into()) },
        &id,
    );
    assert!(!state.loading);
    assert_eq!(state.draft.as_ref().unwrap().policy.root, "/draft");
    assert_eq!(state.default_root, "/server/default");
}

#[test]
fn worktree_settings_ui_project_switch_keeps_save_locked_until_acknowledged() {
    let (mut state, id) = (State::default(), identity());
    load(&mut state, &id, WorktreeSettings::default());
    apply(&mut state, Event::Edit, &id);
    apply(&mut state, Event::CountChanged("5".into()), &id);
    let policy = apply(&mut state, Event::Save, &id).unwrap();
    let generation = state.generation;
    let mut other = id.clone();
    other.project = Some("/other".into());
    apply(&mut state, Event::ContextChanged, &other);
    assert!(state.saving);
    assert!(apply(&mut state, Event::Save, &other).is_none());
    apply(&mut state, Event::Refresh, &other);
    assert!(!state.loading);
    apply(&mut state, Event::Saved { identity: id, generation, result: Ok(policy) }, &other);
    assert!(!state.saving);
    assert_eq!(state.saved.unwrap().keep_count, 15);
    assert_eq!(state.draft.unwrap().keep_count, "5");
}
