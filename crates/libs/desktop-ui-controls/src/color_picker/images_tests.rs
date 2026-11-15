//! 验证饱和度明度、色相和透明度选择器生成的像素渐变。
//! 抽取角落与端点像素进行断言，既约束渐变方向也约束固定颜色和透明通道。

use iced::Color;
use iced::widget::image;

use super::{alpha_image_handle, hue_image_handle, sv_image_handle};

fn rgba_pixels(handle: image::Handle) -> (u32, u32, Vec<u8>) {
    match handle {
        image::Handle::Rgba { width, height, pixels, .. } => (width, height, pixels.to_vec()),
        other => panic!("expected rgba handle, got {other:?}"),
    }
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * width + x) * 4) as usize;
    [pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3]]
}

#[test]
fn sv_image_handle_builds_square_gradient_with_fixed_alpha() {
    let (width, height, pixels) = rgba_pixels(sv_image_handle(0.0));

    assert_eq!((width, height), (256, 256));
    assert_eq!(pixels.len(), 256 * 256 * 4);
    assert_eq!(pixel(&pixels, width, 0, 0), [255, 255, 255, 255]);
    assert_eq!(pixel(&pixels, width, 255, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 0, 255), [0, 0, 0, 255]);
}

#[test]
fn hue_image_handle_builds_horizontal_hue_spectrum() {
    let (width, height, pixels) = rgba_pixels(hue_image_handle());

    assert_eq!((width, height), (256, 16));
    assert_eq!(pixels.len(), 256 * 16 * 4);
    assert_eq!(pixel(&pixels, width, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 85, 0), [0, 255, 0, 255]);
    assert_eq!(pixel(&pixels, width, 255, 15), [255, 0, 0, 255]);
}

#[test]
fn alpha_image_handle_keeps_rgb_and_gradually_increases_alpha() {
    let (width, height, pixels) = rgba_pixels(alpha_image_handle(Color::from_rgb(0.25, 0.5, 0.75)));

    assert_eq!((width, height), (256, 16));
    assert_eq!(pixels.len(), 256 * 16 * 4);
    assert_eq!(pixel(&pixels, width, 0, 0), [64, 128, 191, 0]);
    assert_eq!(pixel(&pixels, width, 255, 0), [64, 128, 191, 255]);
    assert_eq!(pixel(&pixels, width, 255, 15), [64, 128, 191, 255]);
}
