// 验证图像压缩的尺寸与总量预算，并避免静默破坏动画格式。

// 小图保持原字节，大图按共享预算缩放，GIF 不会被悄然压成静态帧。
use super::image_compression::{
    MAX_OUTPUT_DIMENSION, TOTAL_NORMALIZED_IMAGE_BYTES, compress_image_bytes,
    per_image_output_budget,
};
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, ImageBuffer, ImageFormat, Rgb, Rgba, RgbaImage};
use std::io::Cursor;

fn solid_png(width: u32, height: u32) -> Vec<u8> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([((x.wrapping_mul(3)) % 256) as u8, ((y.wrapping_mul(5)) % 256) as u8, 90])
    });
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encode solid png fixture");
    bytes
}

fn animated_gif_bytes(width: u32, height: u32, frames: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder.set_repeat(Repeat::Infinite).expect("gif repeat");
        for index in 0..frames {
            let color = ((u32::from(index) * 40) % 256) as u8;
            let frame: RgbaImage =
                ImageBuffer::from_pixel(width, height, Rgba([color, 255 - color, 128, 255]));
            encoder.encode_frame(Frame::new(frame)).expect("encode gif frame");
        }
    }
    bytes
}

fn data_uri_payload(data_uri: &str) -> (String, Vec<u8>) {
    let (header, payload) = data_uri.split_once(',').expect("data uri comma");
    let mime = header.trim_start_matches("data:").split(';').next().expect("mime").to_string();
    let bytes = STANDARD.decode(payload).expect("decode data uri payload");
    (mime, bytes)
}

#[tokio::test]
async fn large_static_image_is_resized_and_bounded_before_base64() {
    let temp = tempfile::tempdir().unwrap();
    let image_path = temp.path().join("large.png");
    let png = solid_png(3200, 1800);
    assert!(png.len() > 64 * 1024, "fixture should be large enough to exercise compression");
    std::fs::write(&image_path, &png).unwrap();

    let messages = vec![ChatMessage::user(format!("inspect [IMAGE:{}]", image_path.display()))];
    let prepared =
        prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

    let (_, refs) = parse_image_markers(&prepared.messages[0].content);
    assert_eq!(refs.len(), 1);
    let (mime, bytes) = data_uri_payload(&refs[0]);
    assert_eq!(mime, "image/jpeg");
    assert!(bytes.len() <= per_image_output_budget(1));

    let decoded = image::load_from_memory(&bytes).expect("compressed jpeg should decode");
    assert!(decoded.width() <= MAX_OUTPUT_DIMENSION);
    assert!(decoded.height() <= MAX_OUTPUT_DIMENSION);
    assert!(decoded.width() < 3200 || decoded.height() < 1800);
}

#[tokio::test]
async fn small_image_preserves_original_bytes_and_mime() {
    let temp = tempfile::tempdir().unwrap();
    let image_path = temp.path().join("small.png");
    let png = solid_png(64, 48);
    std::fs::write(&image_path, &png).unwrap();

    let messages = vec![ChatMessage::user(format!("tiny [IMAGE:{}]", image_path.display()))];
    let prepared =
        prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

    let (_, refs) = parse_image_markers(&prepared.messages[0].content);
    let (mime, bytes) = data_uri_payload(&refs[0]);
    assert_eq!(mime, "image/png");
    assert_eq!(bytes, png);
}

#[tokio::test]
async fn multiple_images_share_normalized_byte_budget() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("a.png");
    let second = temp.path().join("b.png");
    std::fs::write(&first, solid_png(2800, 2800)).unwrap();
    std::fs::write(&second, solid_png(2600, 2600)).unwrap();

    let messages = vec![ChatMessage::user(format!(
        "[IMAGE:{}]\n[IMAGE:{}]",
        first.display(),
        second.display()
    ))];
    let prepared =
        prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

    let (_, refs) = parse_image_markers(&prepared.messages[0].content);
    assert_eq!(refs.len(), 2);

    let shared_budget = per_image_output_budget(2);
    assert_eq!(shared_budget, TOTAL_NORMALIZED_IMAGE_BYTES / 2);

    let mut total = 0usize;
    for data_uri in &refs {
        let (mime, bytes) = data_uri_payload(data_uri);
        assert_eq!(mime, "image/jpeg");
        assert!(
            bytes.len() <= shared_budget,
            "each image must fit the shared per-image budget ({shared_budget})"
        );
        total += bytes.len();
    }
    assert!(total <= TOTAL_NORMALIZED_IMAGE_BYTES);
}

#[tokio::test]
async fn gif_is_not_silently_flattened() {
    let temp = tempfile::tempdir().unwrap();
    let gif_path = temp.path().join("anim.gif");
    let gif = animated_gif_bytes(32, 32, 3);
    std::fs::write(&gif_path, &gif).unwrap();

    let messages = vec![ChatMessage::user(format!("gif [IMAGE:{}]", gif_path.display()))];
    let prepared =
        prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

    let (_, refs) = parse_image_markers(&prepared.messages[0].content);
    let (mime, bytes) = data_uri_payload(&refs[0]);
    assert_eq!(mime, "image/gif");
    assert_eq!(bytes, gif);

    let oversized_budget = gif.len().saturating_sub(1).max(1);
    let error = compress_image_bytes(&gif, "image/gif", oversized_budget)
        .expect_err("GIF over output budget must fail without flattening");
    assert!(
        error.contains("GIF exceeds output budget") && error.contains("cannot be compressed"),
        "unexpected gif budget error: {error}"
    );
}
