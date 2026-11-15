//! 多模态图片在 Base64 前的进程内压缩。
//!
//! 仅使用 Rust `image` crate 完成解码、白底合成、等比缩放与 JPEG 重编码，不启动子进程或外部服务。
//! 源图片数量、源字节上限与 MIME 白名单仍由调用方在进入本模块前校验。

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView, ImageError, Rgb, RgbImage, Rgba};

/// 归一化后单张图片最长边上限（像素）。
pub(super) const MAX_OUTPUT_DIMENSION: u32 = 2048;

/// 单次请求所有归一化图片的总字节预算。
pub(super) const TOTAL_NORMALIZED_IMAGE_BYTES: usize = 4 * 1024 * 1024;

const JPEG_QUALITY_STEPS: &[u8] = &[85, 70, 55, 40, 25];
const MIN_DIMENSION: u32 = 32;

/// 按请求图片数量分摊总输出预算。
pub(super) fn per_image_output_budget(image_count: usize) -> usize {
    TOTAL_NORMALIZED_IMAGE_BYTES / image_count.max(1)
}

/// 在输出预算内压缩静态图片；GIF 保留原字节。
///
/// # 返回值
///
/// `(mime, bytes)`：可能保留原 MIME/字节，或重编码为 `image/jpeg`。
///
/// # 错误
///
/// - GIF 超过输出预算（不支持重编码）
/// - 解码失败且需要压缩
/// - 无法在预算内完成 JPEG 编码
pub(super) fn compress_image_bytes(
    bytes: &[u8],
    mime: &str,
    output_budget: usize,
) -> Result<(String, Vec<u8>), String> {
    if mime.eq_ignore_ascii_case("image/gif") {
        if bytes.len() > output_budget {
            return Err(format!(
                "GIF exceeds output budget and cannot be compressed: {} bytes > {} bytes",
                bytes.len(),
                output_budget
            ));
        }
        return Ok((mime.to_ascii_lowercase(), bytes.to_vec()));
    }

    let needs_size_bound = bytes.len() > output_budget;
    let decoded = match image::load_from_memory(bytes) {
        Ok(image) => image,
        Err(error) => {
            if needs_size_bound {
                return Err(format!("failed to decode image for compression: {error}"));
            }
            // 已通过源大小与 MIME 校验且仍在输出预算内：保留原字节，避免对边界 fixture 强制解码。
            return Ok((mime.to_ascii_lowercase(), bytes.to_vec()));
        }
    };

    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err("image has zero width or height".to_string());
    }

    let needs_resize = width > MAX_OUTPUT_DIMENSION || height > MAX_OUTPUT_DIMENSION;
    if !needs_size_bound && !needs_resize {
        return Ok((mime.to_ascii_lowercase(), bytes.to_vec()));
    }

    let jpeg_bytes = encode_jpeg_within_budget(&decoded, output_budget)?;
    Ok(("image/jpeg".to_string(), jpeg_bytes))
}

fn encode_jpeg_within_budget(
    image: &DynamicImage,
    output_budget: usize,
) -> Result<Vec<u8>, String> {
    let mut rgb = flatten_to_rgb(image);
    let (mut width, mut height) = (rgb.width(), rgb.height());

    if width > MAX_OUTPUT_DIMENSION || height > MAX_OUTPUT_DIMENSION {
        let resized = resize_to_max_edge(&rgb, MAX_OUTPUT_DIMENSION);
        width = resized.width();
        height = resized.height();
        rgb = resized;
    }

    loop {
        for &quality in JPEG_QUALITY_STEPS {
            let encoded = encode_jpeg_rgb(&rgb, quality)
                .map_err(|error| format!("JPEG encode failed at quality {quality}: {error}"))?;
            if encoded.len() <= output_budget {
                return Ok(encoded);
            }
        }

        if width <= MIN_DIMENSION || height <= MIN_DIMENSION {
            break;
        }

        let next_max = (width.max(height) * 3 / 4).max(MIN_DIMENSION);
        let resized = resize_to_max_edge(&rgb, next_max);
        if resized.width() == width && resized.height() == height {
            break;
        }
        width = resized.width();
        height = resized.height();
        rgb = resized;
    }

    Err(format!("unable to compress image within output budget of {output_budget} bytes"))
}

fn flatten_to_rgb(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    let mut rgb = RgbImage::new(rgba.width(), rgba.height());
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        let alpha = f32::from(a) / 255.0;
        let blend = |channel: u8| ((f32::from(channel) * alpha) + (255.0 * (1.0 - alpha))) as u8;
        rgb.put_pixel(x, y, Rgb([blend(r), blend(g), blend(b)]));
    }
    rgb
}

fn resize_to_max_edge(image: &RgbImage, max_edge: u32) -> RgbImage {
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 || max_edge == 0 {
        return image.clone();
    }
    if width <= max_edge && height <= max_edge {
        return image.clone();
    }

    let longest = width.max(height);
    let scaled_width =
        ((u64::from(width) * u64::from(max_edge)) / u64::from(longest)).max(1) as u32;
    let scaled_height =
        ((u64::from(height) * u64::from(max_edge)) / u64::from(longest)).max(1) as u32;
    image::imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle)
}

fn encode_jpeg_rgb(image: &RgbImage, quality: u8) -> Result<Vec<u8>, ImageError> {
    let mut buffer = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
    encoder.encode(image.as_raw(), image.width(), image.height(), ColorType::Rgb8.into())?;
    Ok(buffer)
}
