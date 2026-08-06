use super::assets::{
    ImageAssetId, ImageAssetMetadata, ImageAssetScope, ImageAssetStore, ImageAssetVariant,
    IMAGE_ASSET_METADATA_VERSION, asset_dir_for_tests,
};
use super::image_compression::{compress_image_bytes, per_image_output_budget};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::path::Path;

fn solid_png(width: u32, height: u32) -> Vec<u8> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([
            ((x.wrapping_mul(3)) % 256) as u8,
            ((y.wrapping_mul(5)) % 256) as u8,
            90,
        ])
    });
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encode solid png fixture");
    bytes
}

fn store_in(temp: &Path) -> ImageAssetStore {
    ImageAssetStore::new(temp)
}

fn put_fixture(
    store: &ImageAssetStore,
    scope: &ImageAssetScope,
    original: &[u8],
) -> (ImageAssetId, Vec<u8>, String) {
    let (mime, model_input) =
        compress_image_bytes(original, "image/png", per_image_output_budget(1))
            .expect("compress fixture for model input");
    let image_id = store
        .put(scope, original, "image/png", &model_input, &mime)
        .expect("put image asset");
    (image_id, model_input, mime)
}

#[test]
fn same_content_in_scope_reuses_image_id() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("project-a").unwrap();
    let original = solid_png(96, 64);

    let (first_id, model_input, mime) = put_fixture(&store, &scope, &original);
    let second_id = store
        .put(&scope, &original, "image/png", &model_input, &mime)
        .expect("reuse existing asset");

    assert_eq!(first_id, second_id);
    assert_eq!(first_id.as_str().len(), 64);

    let asset_dir = asset_dir_for_tests(&store, &scope, &first_id);
    let path = asset_dir.to_string_lossy();
    assert!(
        !path.contains("project-a"),
        "disk path must use encoded scope, not raw scope key: {path}"
    );
    assert!(
        path.contains(scope.path_segment().as_str()),
        "disk path must include controlled scope encoding"
    );
    assert!(
        path.contains(first_id.as_str()),
        "disk path must include full content hash"
    );
    let entries = std::fs::read_dir(asset_dir.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
        .filter(|entry| {
            !entry
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp-")
        })
        .count();
    assert_eq!(entries, 1, "same content must not publish a second asset directory");
}

#[test]
fn asset_cannot_be_resolved_across_scopes() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope_a = ImageAssetScope::new("project-a").unwrap();
    let scope_b = ImageAssetScope::new("project-b").unwrap();
    let original = solid_png(80, 60);

    let (image_id, _, _) = put_fixture(&store, &scope_a, &original);

    assert!(store.is_published(&scope_a, &image_id));
    assert!(!store.is_published(&scope_b, &image_id));
    let err = store
        .get_metadata(&scope_b, &image_id)
        .expect_err("cross-scope resolve must fail");
    assert!(
        err.to_string().contains("not published"),
        "unexpected error: {err}"
    );
    let err = store
        .read_variant(&scope_b, &image_id, ImageAssetVariant::Original)
        .expect_err("cross-scope original read must fail");
    assert!(err.to_string().contains("not published"));
}

#[test]
fn asset_keeps_original_and_normalized_variants() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::from_request_context(Some("proj-42"), "session-ignored").unwrap();
    assert_eq!(scope.key(), "proj-42");

    let original = solid_png(2400, 1800);
    let (image_id, model_input, mime) = put_fixture(&store, &scope, &original);
    assert_ne!(model_input, original, "fixture should exercise compressed model input");
    assert_eq!(mime, "image/jpeg");

    let stored_original = store
        .read_variant(&scope, &image_id, ImageAssetVariant::Original)
        .expect("read original");
    let stored_model = store
        .read_variant(&scope, &image_id, ImageAssetVariant::ModelInput)
        .expect("read model input");

    assert_eq!(stored_original, original);
    assert_eq!(stored_model, model_input);

    let metadata = store.get_metadata(&scope, &image_id).expect("metadata");
    assert_eq!(metadata.version, IMAGE_ASSET_METADATA_VERSION);
    assert_eq!(metadata.original_byte_len, original.len());
    assert_eq!(metadata.model_input_byte_len, model_input.len());
    assert_eq!(metadata.original_mime, "image/png");
    assert_eq!(metadata.model_input_mime, "image/jpeg");
    assert_eq!(metadata.width, Some(2400));
    assert_eq!(metadata.height, Some(1800));
}

#[test]
fn metadata_omits_sensitive_payloads() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::from_request_context(None, "session-xyz").unwrap();
    let original = solid_png(32, 24);
    let (image_id, _, _) = put_fixture(&store, &scope, &original);

    let metadata = store.get_metadata(&scope, &image_id).unwrap();
    let json = serde_json::to_value(&metadata).expect("serialize metadata");
    let object = json.as_object().expect("metadata object");

    let forbidden = [
        "source_path",
        "path",
        "base64",
        "data_uri",
        "ocr",
        "visible_text",
        "visual_summary",
        "summary",
        "payload",
        "bytes",
    ];
    for key in forbidden {
        assert!(
            !object.contains_key(key),
            "metadata must not contain sensitive field `{key}`: {object:?}"
        );
    }

    let encoded = serde_json::to_string(&metadata).unwrap();
    assert!(!encoded.contains("data:image"));
    assert!(!encoded.contains("/Users/"));
    assert!(!encoded.contains("base64"));
}

#[test]
fn partial_write_is_not_published() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("project-partial").unwrap();
    let original = solid_png(48, 48);
    let image_id = ImageAssetId::from_hex({
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(&original))
    })
    .unwrap();

    let asset_dir = asset_dir_for_tests(&store, &scope, &image_id);
    std::fs::create_dir_all(&asset_dir).unwrap();
    // 仅写入原图、缺少 metadata.json：对解析方不可见。
    std::fs::write(asset_dir.join("original.bin"), &original).unwrap();
    assert!(!store.is_published(&scope, &image_id));
    assert!(store.get_metadata(&scope, &image_id).is_err());
    assert!(
        store
            .read_variant(&scope, &image_id, ImageAssetVariant::Original)
            .is_err()
    );

    // 带 `.tmp-` 前缀的半成品目录同样不可解析。
    let staging = asset_dir
        .parent()
        .unwrap()
        .join(format!(".tmp-{}-orphan", image_id.as_str()));
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("original.bin"), &original).unwrap();
    std::fs::write(staging.join("model_input.bin"), &original).unwrap();
    std::fs::write(
        staging.join("metadata.json"),
        serde_json::to_vec(&ImageAssetMetadata {
            version: IMAGE_ASSET_METADATA_VERSION,
            image_id: image_id.clone(),
            original_mime: "image/png".into(),
            model_input_mime: "image/png".into(),
            original_byte_len: original.len(),
            model_input_byte_len: original.len(),
            width: Some(48),
            height: Some(48),
            created_at_ms: 1,
            last_accessed_at_ms: 1,
            session_refs: Default::default(),
        })
        .unwrap(),
    )
    .unwrap();
    assert!(!store.is_published(&scope, &image_id));

    // 完整 put 应能原子发布，并覆盖半成品目录。
    let (published_id, _, _) = put_fixture(&store, &scope, &original);
    assert_eq!(published_id, image_id);
    assert!(store.is_published(&scope, &image_id));
    assert_eq!(
        store
            .read_variant(&scope, &image_id, ImageAssetVariant::Original)
            .unwrap(),
        original
    );
}

#[test]
fn deleting_one_of_two_references_keeps_asset() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("shared-scope").unwrap();
    let original = solid_png(40, 30);
    let (image_id, _, _) = put_fixture(&store, &scope, &original);

    store.register_session_ref(&scope, &image_id, "session-a").unwrap();
    store.register_session_ref(&scope, &image_id, "session-b").unwrap();
    store.release_session_refs(&scope, "session-a").unwrap();

    assert!(store.is_published(&scope, &image_id));
    let metadata = store.get_metadata(&scope, &image_id).unwrap();
    assert!(metadata.session_refs.contains("session-b"));
    assert!(!metadata.session_refs.contains("session-a"));
}

#[test]
fn deleting_last_reference_removes_asset_and_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("last-ref").unwrap();
    let original = solid_png(40, 30);
    let (image_id, _, _) = put_fixture(&store, &scope, &original);
    let analysis_dir = asset_dir_for_tests(&store, &scope, &image_id).join("analysis");
    std::fs::create_dir_all(&analysis_dir).unwrap();
    std::fs::write(analysis_dir.join("dummy.json"), b"{}").unwrap();

    store.register_session_ref(&scope, &image_id, "only").unwrap();
    store.release_session_refs(&scope, "only").unwrap();

    assert!(!store.is_published(&scope, &image_id));
    assert!(!analysis_dir.exists());
}

#[test]
fn session_cleanup_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("idempotent").unwrap();
    let original = solid_png(20, 20);
    let (image_id, _, _) = put_fixture(&store, &scope, &original);
    store.register_session_ref(&scope, &image_id, "s1").unwrap();
    store.release_session_refs(&scope, "s1").unwrap();
    store.release_session_refs(&scope, "s1").unwrap();
    store.release_session_refs(&scope, "missing").unwrap();
    assert!(!store.is_published(&scope, &image_id));
}

#[test]
fn many_historical_images_emit_only_bounded_text() {
    use super::context::{ImageTurnContext, TrackedImage};
    use super::analysis::ImageAnalysisResult;

    let mut images = Vec::new();
    for index in 0..6 {
        let id = format!("{:02}", index).repeat(32);
        let image_id = ImageAssetId::from_hex(&id[..64]).unwrap();
        let analysis = ImageAnalysisResult {
            version: 1,
            image_id: image_id.clone(),
            visible_text: "ocr".into(),
            visual_summary: "sum".into(),
            model_ref: "vision/reader".into(),
            prompt_version: "v1".into(),
        };
        let reference =
            super::analysis::format_image_reference_text(&image_id, &analysis);
        images.push(TrackedImage {
            image_id,
            is_current: false,
            source_ref: format!("/tmp/{index}.png"),
            model_input_data_uri: format!("data:image/jpeg;base64,AAAA{index}"),
            reference_text: reference,
            analysis,
        });
    }
    let ctx = ImageTurnContext::from_tracked(
        ImageAssetScope::new("hist").unwrap(),
        images.clone(),
        false,
    );
    let mut content = String::new();
    for image in &images {
        content.push_str(&format!("[IMAGE:{}]\n", image.source_ref));
    }
    let rewritten = ctx.rewrite_content(&content);
    assert!(!rewritten.contains("data:image"));
    assert!(!rewritten.contains("/tmp/"));
    assert_eq!(rewritten.matches("[VW_IMAGE:").count(), 6);
    assert!(ctx.assert_provider_messages_safe(&[serde_json::json!({
        "role": "user",
        "content": rewritten,
    })]).is_ok());
}

#[test]
fn observability_omits_image_and_analysis_payloads() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_in(temp.path());
    let scope = ImageAssetScope::new("obs").unwrap();
    let (image_id, _, _) = put_fixture(&store, &scope, &solid_png(16, 16));
    store.register_session_ref(&scope, &image_id, "sess").unwrap();
    let metadata = store.get_metadata(&scope, &image_id).unwrap();
    let encoded = serde_json::to_string(&metadata).unwrap();
    assert!(!encoded.contains("data:image"));
    assert!(!encoded.contains("base64"));
    assert!(!encoded.contains("visible_text"));
    assert!(!encoded.contains("visual_summary"));
    assert!(encoded.contains("session_refs"));
    assert!(encoded.contains(&image_id.as_str()[..16]));
}
