use super::*;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicU64, Ordering},
};
use vw_api_transport::id::UserId;

const SESSION: &str = "session-1";

fn scope(tenant: &TenantScope, principal: u8, session: &str) -> MediaJobScope {
    MediaJobScope::new(tenant, [principal; 32], MediaSessionQuery { session_id: session.into() })
        .unwrap()
}

fn owner() -> MediaJobScope {
    scope(&TenantScope::default(), 1, SESSION)
}

fn request() -> MediaSubmit {
    MediaSubmit {
        request_id: MediaJobId::new_v4(),
        session_id: SESSION.into(),
        operation: MediaOperation::VideoGenerate,
        deployment_id: "direct".into(),
        model_id: "video-model".into(),
        text: Some("A moon above a lake".into()),
        inputs: Vec::new(),
    }
}

fn video(bytes: u64) -> MediaAsset {
    MediaAsset {
        id: "video-1".into(),
        mime: "video/mp4".into(),
        byte_len: bytes,
        width: Some(640),
        height: Some(480),
        duration_ms: Some(5000),
        source: MediaAssetSource::Stream,
    }
}

#[test]
fn media_job_contract_scopes_deduplication_and_conflicts() {
    let temp = tempfile::tempdir().unwrap();
    let store = MediaJobStore::open_with_clock(&temp.path().join("media.db"), || 100).unwrap();
    let owner = owner();
    let mut request = request();
    let id = request.request_id;
    let MediaReservation::Start { job, .. } = store.reserve(&owner, &request).unwrap() else {
        panic!("first reservation must start");
    };
    assert_eq!(job.id, id);
    assert_eq!(job.created_at_ms, 100);
    assert_eq!(job.state, MediaJobState::Submitted);
    assert!(matches!(
        store.reserve(&owner, &request).unwrap(),
        MediaReservation::Existing(existing) if existing == job
    ));

    let tenant = TenantScope::default();
    let other_tenant = TenantScope::new("platform-2", "company-2").unwrap();
    let other_user = tenant.clone().with_user_id(UserId::new("user-2").unwrap());
    for unrelated in [
        scope(&tenant, 2, SESSION),
        scope(&other_tenant, 1, SESSION),
        scope(&other_user, 1, SESSION),
        scope(&tenant, 1, "session-2"),
    ] {
        assert_eq!(store.get(&unrelated, id), Err(Code::NotFound));
        assert_eq!(store.cancel(&unrelated, id), Err(Code::NotFound));
        assert_eq!(store.asset(&unrelated, id, "video-1"), Err(Code::NotFound));
    }

    let other_session = scope(&tenant, 1, "session-2");
    assert!(matches!(store.reserve(&other_session, &request), Err(Code::NotFound)));
    request.text = Some("Different input".into());
    assert!(matches!(store.reserve(&owner, &request), Err(Code::Conflict)));
    request.session_id = "session-2".into();
    assert!(matches!(
        store.reserve(&other_session, &request).unwrap(),
        MediaReservation::Start { .. }
    ));
}

#[test]
fn media_job_contract_concurrent_reservation_only_starts_once() {
    let store = Arc::new(MediaJobStore::open_with_clock(Path::new(":memory:"), || 10).unwrap());
    let request = Arc::new(request());
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let (store, request, barrier) = (store.clone(), request.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                matches!(store.reserve(&owner(), &request).unwrap(), MediaReservation::Start { .. })
            })
        })
        .collect();
    assert_eq!(
        workers.into_iter().map(|worker| worker.join().unwrap()).filter(|started| *started).count(),
        1
    );
}

#[test]
fn media_job_contract_cancellation_is_terminal_and_prevents_asset_publication() {
    static NOW: AtomicU64 = AtomicU64::new(100);
    let store =
        MediaJobStore::open_with_clock(Path::new(":memory:"), || NOW.load(Ordering::Relaxed))
            .unwrap();
    let owner = owner();
    let request = request();
    let id = request.request_id;
    let MediaReservation::Start { cancellation, .. } = store.reserve(&owner, &request).unwrap()
    else {
        panic!("first reservation must start");
    };
    NOW.store(120, Ordering::Relaxed);
    let running =
        store.update(&owner, id, MediaJobState::Running { progress_percent: Some(25) }).unwrap();
    assert_eq!(running.updated_at_ms, 120);
    assert_eq!(
        store.update(&owner, id, MediaJobState::Running { progress_percent: Some(10) }),
        Err(Code::InvalidRequest)
    );
    assert_eq!(store.asset(&owner, id, "video-1"), Err(Code::NotFound));
    NOW.store(130, Ordering::Relaxed);
    let canceled = store.cancel(&owner, id).unwrap();
    assert_eq!(canceled.state, MediaJobState::Canceled);
    assert_eq!(canceled.updated_at_ms, 130);
    assert!(cancellation.is_cancelled());
    assert_eq!(store.cancel(&owner, id), Ok(canceled.clone()));
    assert!(matches!(
        store.reserve(&owner, &request).unwrap(),
        MediaReservation::Existing(job) if job == canceled
    ));
    assert_eq!(
        store.update(
            &owner,
            id,
            MediaJobState::Succeeded {
                result: MediaResult { text: None, assets: vec![video(10)] }
            }
        ),
        Err(Code::Conflict)
    );
    assert_eq!(store.asset(&owner, id, "video-1"), Err(Code::NotFound));
}

#[test]
fn media_job_contract_restart_preserves_ids_and_reports_incomplete_work_as_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("media.db");
    let owner = owner();
    let pending = request();
    let complete = request();
    let descriptor = video(10);
    {
        let store = MediaJobStore::open_with_clock(&path, || 100).unwrap();
        store.reserve(&owner, &pending).unwrap();
        store
            .update(
                &owner,
                pending.request_id,
                MediaJobState::Running { progress_percent: Some(40) },
            )
            .unwrap();
        store.reserve(&owner, &complete).unwrap();
        store
            .update(
                &owner,
                complete.request_id,
                MediaJobState::Succeeded {
                    result: MediaResult { text: None, assets: vec![descriptor.clone()] },
                },
            )
            .unwrap();
    }

    let store = MediaJobStore::open_with_clock(&path, || 200).unwrap();
    let unknown = store.get(&owner, pending.request_id).unwrap();
    assert_eq!(unknown.state, MediaJobState::SubmissionUnknown);
    assert_eq!(unknown.updated_at_ms, 100);
    assert!(matches!(
        store.reserve(&owner, &pending).unwrap(),
        MediaReservation::Existing(job) if job == unknown
    ));
    assert_eq!(store.asset(&owner, pending.request_id, "video-1"), Err(Code::NotFound));
    assert_eq!(store.asset(&owner, complete.request_id, "video-1"), Ok(descriptor));
    assert!(matches!(
        store.cancel(&owner, complete.request_id).unwrap().state,
        MediaJobState::Succeeded { .. }
    ));
    let unrelated = scope(&TenantScope::default(), 2, SESSION);
    assert_eq!(store.asset(&unrelated, complete.request_id, "video-1"), Err(Code::NotFound));
    assert_eq!(
        store
            .update(
                &owner,
                pending.request_id,
                MediaJobState::Failed { code: Code::UpstreamFailed }
            )
            .unwrap()
            .state,
        MediaJobState::Failed { code: Code::UpstreamFailed }
    );
}

#[test]
fn media_job_contract_generated_image_reuses_a_session_registered_asset() {
    let temp = tempfile::tempdir().unwrap();
    let images = ImageAssetStore::new(temp.path());
    let image_scope = ImageAssetScope::new("authorized-project").unwrap();
    let owner = owner();
    let store = MediaJobStore::open_with_clock(Path::new(":memory:"), || 100).unwrap();
    let mut request = request();
    request.operation = MediaOperation::ImageGenerate;
    let id = request.request_id;
    store.reserve(&owner, &request).unwrap();

    let bytes = b"opaque image fixture";
    let image = images.put(&image_scope, bytes, "image/png", bytes, "image/png").unwrap();
    assert_eq!(
        MediaJobStore::image_descriptor(&owner, &images, &image_scope, &image),
        Err(Code::NotFound)
    );
    images.register_session_ref(&image_scope, &image, SESSION).unwrap();
    let asset = MediaJobStore::image_descriptor(&owner, &images, &image_scope, &image).unwrap();
    store
        .update(
            &owner,
            id,
            MediaJobState::Succeeded {
                result: MediaResult { text: None, assets: vec![asset.clone()] },
            },
        )
        .unwrap();
    assert_eq!(store.read_image(&owner, id, &asset.id, &images, &image_scope).unwrap(), bytes);
    let other_scope = ImageAssetScope::new("unrelated-project").unwrap();
    assert_eq!(store.read_image(&owner, id, &asset.id, &images, &other_scope), Err(Code::NotFound));
}

#[test]
fn media_job_contract_upload_rejects_paths_invalid_mime_and_oversize() {
    let store = MediaJobStore::open_with_clock(Path::new(":memory:"), || 100).unwrap();
    let mut request = request();
    request.inputs = vec![MediaInput::Asset { asset_id: "../secret".into() }];
    assert!(matches!(store.reserve(&owner(), &request), Err(Code::InvalidRequest)));
    request.inputs =
        vec![MediaInput::Upload { mime: "text/html".into(), data_base64: "YQ==".into() }];
    assert!(matches!(store.reserve(&owner(), &request), Err(Code::UnsupportedMedia)));
    request.inputs =
        vec![
            MediaInput::upload("audio/wav".into(), &vec![0; MAX_MEDIA_UPLOAD_BYTES / 2 + 1])
                .unwrap();
            2
        ];
    assert!(matches!(store.reserve(&owner(), &request), Err(Code::PayloadTooLarge)));
    assert_eq!(store.get(&owner(), request.request_id), Err(Code::NotFound));
}

#[tokio::test]
async fn media_job_contract_checks_stream_length_and_media_type() {
    let source =
        || stream::iter([Ok::<_, ()>(Bytes::from_static(b"one")), Ok(Bytes::from_static(b"two"))]);
    let mut valid = Box::pin(bounded_media_asset_stream(&video(6), source()).unwrap());
    assert_eq!(valid.next().await.unwrap().unwrap().as_ref(), b"one");
    assert_eq!(valid.next().await.unwrap().unwrap().as_ref(), b"two");
    assert!(valid.next().await.is_none());

    for length in [3, 7] {
        let mut stream = Box::pin(bounded_media_asset_stream(&video(length), source()).unwrap());
        assert_eq!(stream.next().await.unwrap().unwrap().as_ref(), b"one");
        if length > 6 {
            assert_eq!(stream.next().await.unwrap().unwrap().as_ref(), b"two");
        }
        assert_eq!(stream.next().await, Some(Err(Code::Unavailable)));
        assert!(stream.next().await.is_none());
    }
    assert!(matches!(
        bounded_media_asset_stream(&video(MAX_MEDIA_ASSET_BYTES + 1), source()),
        Err(Code::PayloadTooLarge)
    ));
}
