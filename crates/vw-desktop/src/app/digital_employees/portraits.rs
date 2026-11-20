//! Built-in profile portraits from the digital employee web console.

use iced::widget::image::Handle;
use std::sync::LazyLock;

macro_rules! portrait {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/digital-employees/employee-portraits/",
            $name,
            ".webp"
        ))
        .as_slice()
    };
}

static MANAGERS: LazyLock<[Option<Handle>; 3]> = LazyLock::new(|| {
    [portrait!("lead-navy"), portrait!("lead-charcoal"), portrait!("lead-indigo")]
        .map(round_portrait)
});

static WORKERS: LazyLock<[Option<Handle>; 3]> = LazyLock::new(|| {
    [portrait!("staff-teal"), portrait!("staff-cream"), portrait!("staff-slate")]
        .map(round_portrait)
});

pub(super) fn avatar(name: &str, manager: bool) -> Option<Handle> {
    let name = name.trim();
    if name.is_empty() || name == "—" {
        return None;
    }
    // The web roster hashes Unicode code points with unsigned 32-bit wraparound.
    let seed = portrait_seed(name);
    let portraits = if manager { &*MANAGERS } else { &*WORKERS };
    portraits[seed as usize % portraits.len()].clone()
}

static SUPPORT: LazyLock<Option<Handle>> = LazyLock::new(|| round_portrait(portrait!("support")));
static ANALYSTS: LazyLock<[Option<Handle>; 2]> =
    LazyLock::new(|| [portrait!("analyst-indigo"), portrait!("analyst-sage")].map(round_portrait));

pub(super) fn shared_avatar(name: &str, manager: bool, role: &str) -> Option<Handle> {
    if !manager {
        if ["客服", "问答", "咨询", "服务"].iter().any(|tag| role.contains(*tag)) {
            return (*SUPPORT).clone();
        }
        let analyst = [
            "数据",
            "分析",
            "报表",
            "可视化",
            "指标",
            "财务",
            "核算",
            "合规",
            "发票",
            "审查",
            "合同",
            "对账",
        ];
        if analyst.iter().any(|tag| role.contains(*tag)) {
            return ANALYSTS[portrait_seed(name) as usize % ANALYSTS.len()].clone();
        }
    }
    avatar(name, manager)
}

fn portrait_seed(name: &str) -> u32 {
    name.chars().fold(0, |seed, ch| seed.wrapping_mul(31).wrapping_add(u32::from(ch)))
}

// Apply the circular mask once so the software renderer gets the same shape as GPU rendering.
fn round_portrait(bytes: &[u8]) -> Option<Handle> {
    let mut portrait =
        image::load_from_memory_with_format(bytes, image::ImageFormat::WebP).ok()?.into_rgba8();
    let (width, height) = portrait.dimensions();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = center_x.min(center_y);

    for (x, y, pixel) in portrait.enumerate_pixels_mut() {
        let distance = (x as f32 + 0.5 - center_x).hypot(y as f32 + 0.5 - center_y);
        let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
        pixel[3] = (f32::from(pixel[3]) * coverage).round() as u8;
    }

    Some(Handle::from_rgba(width, height, portrait.into_raw()))
}

macro_rules! seated {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/digital-employees/office-portraits/",
            $name,
            ".webp"
        ))
        .as_slice()
    };
}

static SEATED_MANAGERS: LazyLock<[Handle; 3]> = LazyLock::new(|| {
    [seated!("lead-navy"), seated!("lead-charcoal"), seated!("lead-indigo")].map(Handle::from_bytes)
});
static SEATED_WORKERS: LazyLock<[Handle; 3]> = LazyLock::new(|| {
    [seated!("staff-teal"), seated!("staff-cream"), seated!("staff-slate")].map(Handle::from_bytes)
});

pub(super) fn office_avatar(name: &str, manager: bool) -> Handle {
    let seed = name
        .trim()
        .chars()
        .fold(0_u32, |seed, ch| seed.wrapping_mul(31).wrapping_add(u32::from(ch)));
    let portraits = if manager { &*SEATED_MANAGERS } else { &*SEATED_WORKERS };
    portraits[seed as usize % portraits.len()].clone()
}
