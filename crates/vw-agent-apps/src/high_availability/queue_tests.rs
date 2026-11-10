use super::queue::{Queue, QueueError};
use crate::providers::failover::{ProviderAuth, ProviderSnapshot};
use vw_api_types::{agent_apps::AppId, app_providers::AppProviderSummary};

fn provider(app_id: AppId, id: &str, auth: ProviderAuth) -> ProviderSnapshot {
    ProviderSnapshot {
        provider: AppProviderSummary { id: id.into(), app_id, name: id.into(), configured: true },
        auth,
    }
}

#[test]
fn ha_queue_rejects_ineligible_members() {
    use AppId::{Claude, Codex, Pi};
    use ProviderAuth::{OfficialAccount, ThirdParty, Unknown};
    use QueueError::{IneligibleProvider, ProviderNotFound, UnsupportedApp};

    let providers = [
        provider(Codex, "official", OfficialAccount),
        provider(Codex, "unknown", Unknown),
        provider(Claude, "other-app", ThirdParty),
        provider(Pi, "unsupported", ThirdParty),
        provider(Codex, "first", ThirdParty),
        provider(Codex, "second", ThirdParty),
    ];
    let mut unsupported = Queue::new(Pi, vec![]);
    assert_eq!(unsupported.add("unsupported", &providers), Err(UnsupportedApp));
    assert_eq!(unsupported.candidates(&providers), Err(UnsupportedApp));
    assert!(unsupported.members().is_empty());

    let mut queue = Queue::new(Codex, vec!["stale".into()]);
    for (id, error) in [
        ("official", IneligibleProvider),
        ("unknown", IneligibleProvider),
        ("missing", ProviderNotFound),
        ("other-app", ProviderNotFound),
    ] {
        assert_eq!(queue.add(id, &providers), Err(error), "{id}");
        assert_eq!(queue.members(), ["stale"]);
    }
    assert_eq!(queue.add("first", &providers), Ok(true));
    let available = queue.candidates(&providers).unwrap();
    assert_eq!(available.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(), ["second"]);
    assert_eq!(queue.add("second", &providers), Ok(true));
    assert_eq!(queue.add("first", &providers), Ok(false));
    assert_eq!(queue.remove("stale"), Ok(Some("stale".into())));
    assert_eq!(queue.remove("stale"), Ok(None));
    assert_eq!(queue.members(), ["first", "second"]);
    assert!(queue.candidates(&providers).unwrap().is_empty());
}

#[test]
fn ha_reorder_rejects_partial_or_duplicate_members() {
    let original = ["zeta", "alpha", "stale"].map(str::to_owned).to_vec();
    let mut queue = Queue::new(AppId::Codex, original.clone());
    for ids in [
        vec!["alpha", "zeta"],
        vec!["alpha", "alpha", "stale"],
        vec!["alpha", "zeta", "foreign"],
        vec!["alpha", "zeta", "stale", "foreign"],
    ] {
        let result = queue.reorder(ids.iter().copied().map(str::to_owned).collect());
        assert!(result.is_err(), "reorder accepted invalid membership {ids:?}");
        assert_eq!(queue.members(), original);
    }

    let reordered = ["stale", "alpha", "zeta"].map(str::to_owned).to_vec();
    queue.reorder(reordered.clone()).unwrap();
    assert_eq!(queue.members(), reordered);
    queue.reorder(reordered.clone()).unwrap();
    assert_eq!(queue.members(), reordered);

    let mut empty = Queue::new(AppId::Codex, Vec::new());
    assert_eq!(empty.reorder(Vec::new()), Ok(()));
    let mut unsupported = Queue::new(AppId::Pi, Vec::new());
    assert_eq!(unsupported.reorder(Vec::new()), Err(QueueError::UnsupportedApp));
}
