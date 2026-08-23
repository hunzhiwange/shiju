//! Ambient terminal rendering for the Codex companion.
//!
//! Ambient pets reuse the same extracted image frames as the full-screen viewer
//! but are rendered through a different ownership split: ratatui still owns the
//! transcript/composer layout, while the sprite itself is emitted through the
//! terminal image protocol after the frame draw completes.
//!
//! This module therefore owns two separate contracts:
//! choosing which animation frame should be visible for the current semantic
//! pet state, and translating that frame into a precise on-screen image request
//! that does not overlap reserved bottom-pane space. It does not persist pet
//! selection or decide when modal/popover UI should suppress the sprite.
//!
//! 环境宠物复用全屏查看器切出的图片帧，但所有权分离：ratatui 管理记录与输入区布局，
//! 精灵则在帧绘制完成后通过终端图片协议发送。本模块一方面按语义状态选择动画帧，
//! 另一方面把该帧转换为不覆盖底部保留区的精确绘制请求；它不持久化选择，也不决定弹窗遮蔽策略。

#[cfg(test)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use ratatui::layout::Rect;

use crate::tui::FrameRequester;

use super::DEFAULT_PET_ID;
use super::frames;
use super::image_protocol::ImageProtocol;
use super::image_protocol::PetImageSupport;
#[cfg(not(test))]
use super::image_protocol::ProtocolSelection;
use super::model::Animation;
#[cfg(test)]
use super::model::AnimationFrame;
use super::model::Pet;

const PET_TARGET_HEIGHT_PX: u16 = 75;
const PET_COMPOSER_GAP_PX: u16 = 10;
const TERMINAL_ROW_HEIGHT_PX: u16 = 15;

const RUNNING_LIFETIME: Duration = Duration::from_secs(3 * 60);
const FAILED_LIFETIME: Duration = Duration::from_secs(60 * 60);
const WAITING_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
const REVIEW_LIFETIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// 宠物通知的语义状态，同时决定动画、文案与可见期限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PetNotificationKind {
    Running,
    Waiting,
    Review,
    Failed,
}

impl PetNotificationKind {
    fn animation_name(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Review => "review",
            Self::Failed => "failed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Waiting => "Needs input",
            Self::Review => "Ready",
            Self::Failed => "Blocked",
        }
    }

    fn fallback_body(self) -> &'static str {
        match self {
            Self::Running => "Thinking",
            Self::Waiting => "Needs input",
            Self::Review => "Ready",
            Self::Failed => "Blocked",
        }
    }

    fn lifetime(self) -> Duration {
        match self {
            Self::Running => RUNNING_LIFETIME,
            Self::Waiting => WAITING_LIFETIME,
            Self::Review => REVIEW_LIFETIME,
            Self::Failed => FAILED_LIFETIME,
        }
    }
}

/// 一条带更新时间的可过期宠物通知。
#[derive(Debug, Clone)]
struct PetNotification {
    kind: PetNotificationKind,
    body: String,
    updated_at: Instant,
}

impl PetNotification {
    fn new(kind: PetNotificationKind, body: Option<String>) -> Self {
        Self {
            kind,
            body: body.unwrap_or_else(|| kind.fallback_body().to_string()),
            updated_at: Instant::now(),
        }
    }

    fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.updated_at) >= self.kind.lifetime()
    }
}

/// 帧绘制结束后交给终端图片协议执行的完整定位请求。
#[derive(Debug, Clone)]
pub(crate) struct AmbientPetDraw {
    pub(crate) frame: PathBuf,
    pub(crate) protocol: ImageProtocol,
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) clear_top_y: u16,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) height_px: u16,
    pub(crate) sixel_dir: PathBuf,
}

/// 持有宠物模型、帧缓存与连续动画时钟的环境渲染状态。
#[derive(Debug)]
pub(crate) struct AmbientPet {
    pet: Pet,
    support: PetImageSupport,
    frames: Vec<PathBuf>,
    sixel_dir: PathBuf,
    frame_requester: FrameRequester,
    notification: Option<PetNotification>,
    animation_started_at: Instant,
    animations_enabled: bool,
}

impl AmbientPet {
    /// Load the active ambient pet and prepare its frame cache.
    ///
    /// This resolves the selected pet id, extracts per-frame PNGs into the
    /// CODEX_HOME cache, and records the terminal protocol support snapshot used
    /// for later draw requests. A caller that repeatedly recreates `AmbientPet`
    /// instead of mutating one instance would lose animation timing continuity
    /// and pay the frame-cache preparation cost more often than necessary.
    /// 加载当前宠物并准备帧缓存：解析选择、把 PNG 帧写入 CODEX_HOME 缓存并记录协议支持快照。
    /// 应复用并更新同一实例，否则会丢失动画计时连续性并重复承担缓存准备成本。
    pub(crate) fn load(
        selected_pet: Option<&str>,
        codex_home: &std::path::Path,
        frame_requester: FrameRequester,
        animations_enabled: bool,
    ) -> Result<Self> {
        let pet = Pet::load_with_codex_home(
            selected_pet.unwrap_or(DEFAULT_PET_ID),
            /*codex_home*/ Some(codex_home),
        )
        .with_context(|| "load ambient pet")?;
        let cache_dir = codex_home
            .join("cache")
            .join("tui-pets")
            .join("frame-cache")
            .join(&pet.id)
            .join(pet.frame_cache_key()?);
        let frame_dir = cache_dir.join("frames");
        let sixel_dir = cache_dir.join("sixel");
        let frames = frames::prepare_png_frames(&pet, &frame_dir)?;
        Ok(Self {
            pet,
            support: default_image_support(),
            frames,
            sixel_dir,
            frame_requester,
            notification: None,
            animation_started_at: Instant::now(),
            animations_enabled,
        })
    }

    /// 替换当前通知并从第一帧重新开始对应动画。
    pub(crate) fn set_notification(&mut self, kind: PetNotificationKind, body: Option<String>) {
        self.notification = Some(PetNotification::new(kind, body));
        self.animation_started_at = Instant::now();
    }

    pub(crate) fn image_enabled(&self) -> bool {
        self.support.protocol().is_some()
    }

    pub(crate) fn image_columns(&self) -> u16 {
        self.image_size().columns
    }

    #[cfg(test)]
    pub(crate) fn set_image_support_for_tests(&mut self, support: PetImageSupport) {
        self.support = support;
    }

    /// 若动画仍需推进，则向共享帧调度器提交下一次绘制期限。
    pub(crate) fn schedule_next_frame(&self) {
        if let Some(delay) = self.next_frame_delay() {
            self.frame_requester.schedule_frame_in(delay);
        }
    }

    fn next_frame_delay(&self) -> Option<Duration> {
        if self.support.protocol().is_none() || !self.animations_enabled {
            return None;
        }

        current_animation_frame(
            self.current_animation()?,
            self.animation_started_at.elapsed(),
        )?
        .delay
    }

    /// Build an image draw request for the ambient pet anchored above the composer.
    ///
    /// Returning `None` means "do not render the sprite this frame", typically
    /// because the terminal protocol is unavailable or the current layout cannot
    /// fit the image without overlapping reserved UI. Callers should not try to
    /// partially clip the image themselves; that would desynchronize the image
    /// protocol output from the TUI's notion of cleared rows.
    /// 构造锚定在输入区上方的环境宠物绘制请求。协议不可用或空间不足时返回 `None`；
    /// 调用方不得自行裁剪，否则图片协议输出会与 TUI 记录的清除行失去同步。
    pub(crate) fn draw_request(
        &self,
        area: Rect,
        composer_bottom_y: u16,
    ) -> Option<AmbientPetDraw> {
        let protocol = self.support.protocol()?;
        let size = self.image_size();
        let notification = self.visible_notification(Instant::now());
        let notification_height = notification.map_or(0, notification_height);
        let required_height = size.rows.saturating_add(notification_height);
        let sprite_bottom_y = composer_bottom_y.saturating_sub(composer_gap_rows());
        if sprite_bottom_y < area.y.saturating_add(required_height) || area.width < size.columns {
            return None;
        }

        let x = area.x + area.width.saturating_sub(size.columns);
        let y = sprite_bottom_y.saturating_sub(size.rows);
        Some(AmbientPetDraw {
            frame: self.current_frame_path()?,
            protocol,
            x,
            y,
            clear_top_y: area.y,
            columns: size.columns,
            rows: size.rows,
            height_px: size.height_px,
            sixel_dir: self.sixel_dir.clone(),
        })
    }

    /// Build a centered preview draw request for the `/pets` picker side pane.
    ///
    /// The picker preview intentionally uses the first idle frame rather than
    /// the live animation state so selection browsing stays stable and does not
    /// require the full ambient animation lifecycle.
    /// 为 `/pets` 侧栏构造居中的预览请求。预览固定使用 idle 首帧，避免浏览选择时引入实时动画生命周期。
    pub(crate) fn preview_draw_request(&self, area: Rect) -> Option<AmbientPetDraw> {
        let protocol = self.support.protocol()?;
        let size = self.image_size();
        if area.width < size.columns || area.height < size.rows {
            return None;
        }

        let y = area.y + area.height.saturating_sub(size.rows) / 2;
        Some(AmbientPetDraw {
            frame: self.first_idle_frame_path()?,
            protocol,
            x: area.x + area.width.saturating_sub(size.columns) / 2,
            y,
            clear_top_y: y,
            columns: size.columns,
            rows: size.rows,
            height_px: size.height_px,
            sixel_dir: self.sixel_dir.clone(),
        })
    }

    fn visible_notification(&self, now: Instant) -> Option<&PetNotification> {
        self.notification
            .as_ref()
            .filter(|notification| !notification.is_expired(now))
    }

    // 一次性动画结束后切换到声明的回退轨道；循环动画则继续使用自身。
    fn current_animation(&self) -> Option<&Animation> {
        let animation_name = self
            .visible_notification(Instant::now())
            .map_or("idle", |notification| notification.kind.animation_name());
        let animation = self
            .pet
            .animations
            .get(animation_name)
            .or_else(|| self.pet.animations.get("idle"))?;
        if animation.loop_start.is_none() {
            let elapsed = self.animation_started_at.elapsed();
            if elapsed >= animation.total_duration()
                && let Some(fallback) = self.pet.animations.get(&animation.fallback)
            {
                return Some(fallback);
            }
        }
        Some(animation)
    }

    // 禁用动画时稳定返回当前轨道第一帧，以满足减少动态效果设置。
    fn current_frame_path(&self) -> Option<PathBuf> {
        let sprite_index = self
            .current_animation()
            .and_then(|animation| {
                if self.animations_enabled {
                    current_animation_frame(animation, self.animation_started_at.elapsed())
                        .map(|frame| frame.sprite_index)
                } else {
                    animation.frames.first().map(|frame| frame.sprite_index)
                }
            })
            .unwrap_or(0);
        self.frame_path_for_sprite_index(sprite_index)
    }

    fn first_idle_frame_path(&self) -> Option<PathBuf> {
        let sprite_index = self
            .pet
            .animations
            .get("idle")
            .and_then(|animation| animation.frames.first())
            .map_or(0, |frame| frame.sprite_index);
        self.frame_path_for_sprite_index(sprite_index)
    }

    fn frame_path_for_sprite_index(&self, sprite_index: usize) -> Option<PathBuf> {
        self.frames
            .get(sprite_index.min(self.frames.len().saturating_sub(1)))
            .cloned()
    }

    fn image_size(&self) -> ImageSize {
        let rows = (f64::from(PET_TARGET_HEIGHT_PX) / f64::from(TERMINAL_ROW_HEIGHT_PX))
            .round()
            .max(/*other*/ 1.0) as u16;
        let aspect = f64::from(self.pet.frame_height) / f64::from(self.pet.frame_width) * 0.52;
        let columns = (f64::from(rows) / aspect).round() as u16;
        ImageSize {
            columns: columns.max(1),
            rows,
            height_px: PET_TARGET_HEIGHT_PX,
        }
    }
}

fn composer_gap_rows() -> u16 {
    ((f64::from(PET_COMPOSER_GAP_PX) / f64::from(TERMINAL_ROW_HEIGHT_PX)).round() as u16)
        .max(/*other*/ 1)
}

#[cfg(not(test))]
fn default_image_support() -> PetImageSupport {
    ProtocolSelection::Auto.resolve()
}

#[cfg(test)]
fn default_image_support() -> PetImageSupport {
    PetImageSupport::Unsupported(super::image_protocol::PetImageUnsupportedReason::Terminal)
}

#[derive(Debug, Clone, Copy)]
struct ImageSize {
    columns: u16,
    rows: u16,
    height_px: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AnimationFrameTick {
    sprite_index: usize,
    delay: Option<Duration>,
}

/// 计算当前动画帧与下一次需要重绘的剩余时间，循环段会对经过时间取模。
fn current_animation_frame(animation: &Animation, elapsed: Duration) -> Option<AnimationFrameTick> {
    if animation.frames.len() <= 1 {
        return Some(AnimationFrameTick {
            sprite_index: animation.frames.first()?.sprite_index,
            delay: None,
        });
    }

    let elapsed_nanos = elapsed.as_nanos();
    if let Some(loop_start) = animation
        .loop_start
        .filter(|idx| *idx < animation.frames.len())
    {
        let total_nanos = animation.total_duration().as_nanos();
        let prefix_nanos = animation.frames[..loop_start]
            .iter()
            .map(|frame| frame.duration.as_nanos())
            .sum::<u128>();
        let loop_nanos = animation.frames[loop_start..]
            .iter()
            .map(|frame| frame.duration.as_nanos())
            .sum::<u128>();
        let effective_elapsed = if elapsed_nanos >= total_nanos && loop_nanos > 0 {
            prefix_nanos + elapsed_nanos.saturating_sub(prefix_nanos) % loop_nanos
        } else {
            elapsed_nanos
        };
        frame_at_elapsed(animation, effective_elapsed)
    } else if elapsed_nanos >= animation.total_duration().as_nanos() {
        Some(AnimationFrameTick {
            sprite_index: animation.frames.last()?.sprite_index,
            delay: None,
        })
    } else {
        frame_at_elapsed(animation, elapsed_nanos)
    }
}

/// 在线性帧序列中定位给定经过时间，并返回本帧剩余时长。
fn frame_at_elapsed(animation: &Animation, elapsed_nanos: u128) -> Option<AnimationFrameTick> {
    let mut remaining_elapsed = elapsed_nanos;
    for frame in &animation.frames {
        let frame_nanos = frame.duration.as_nanos().max(/*other*/ 1);
        if remaining_elapsed < frame_nanos {
            return Some(AnimationFrameTick {
                sprite_index: frame.sprite_index,
                delay: Some(nanos_to_duration(frame_nanos - remaining_elapsed)),
            });
        }
        remaining_elapsed = remaining_elapsed.saturating_sub(frame_nanos);
    }

    Some(AnimationFrameTick {
        sprite_index: animation.frames.last()?.sprite_index,
        delay: None,
    })
}

fn nanos_to_duration(nanos: u128) -> Duration {
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

fn notification_height(notification: &PetNotification) -> u16 {
    if notification.body == notification.kind.label() {
        1
    } else {
        2
    }
}

#[cfg(test)]
pub(crate) fn test_ambient_pet(
    frame_requester: FrameRequester,
    animations_enabled: bool,
) -> AmbientPet {
    AmbientPet {
        pet: Pet {
            id: "test".to_string(),
            display_name: "Test".to_string(),
            description: String::new(),
            spritesheet_path: PathBuf::from("spritesheet.webp"),
            frame_width: 192,
            frame_height: 208,
            columns: 8,
            rows: 9,
            frame_count: 72,
            animations: HashMap::from([("idle".to_string(), test_animation())]),
        },
        support: PetImageSupport::Supported(ImageProtocol::Kitty),
        frames: vec![PathBuf::from("frame-0.png"), PathBuf::from("frame-1.png")],
        sixel_dir: PathBuf::new(),
        frame_requester,
        notification: None,
        animation_started_at: Instant::now()
            .checked_sub(Duration::from_millis(/*millis*/ 15))
            .unwrap(),
        animations_enabled,
    }
}

#[cfg(test)]
fn test_animation() -> Animation {
    Animation {
        frames: vec![
            AnimationFrame {
                sprite_index: 0,
                duration: Duration::from_millis(/*millis*/ 10),
            },
            AnimationFrame {
                sprite_index: 1,
                duration: Duration::from_millis(/*millis*/ 10),
            },
        ],
        loop_start: Some(/*loop_start*/ 0),
        fallback: "idle".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // 验证通知标签与 Codex App 使用的产品词汇一致。
    fn notification_labels_match_codex_app_vocabulary() {
        assert_eq!(PetNotificationKind::Running.label(), "Running");
        assert_eq!(PetNotificationKind::Waiting.label(), "Needs input");
        assert_eq!(PetNotificationKind::Review.label(), "Ready");
        assert_eq!(PetNotificationKind::Failed.label(), "Blocked");
    }

    #[test]
    // 验证调度延迟取自当前帧自身的持续时间，而非固定帧率。
    fn animation_frame_uses_per_frame_duration() {
        let animation = test_animation();

        assert_eq!(
            current_animation_frame(&animation, Duration::from_millis(/*millis*/ 15)),
            Some(AnimationFrameTick {
                sprite_index: 1,
                delay: Some(Duration::from_millis(/*millis*/ 5)),
            })
        );
    }

    #[test]
    // 验证减少动态效果时固定首帧且不再安排后续重绘。
    fn reduced_motion_uses_stable_first_frame_and_schedules_no_follow_up() {
        let pet = test_ambient_pet(
            FrameRequester::test_dummy(),
            /*animations_enabled*/ false,
        );

        assert_eq!(pet.current_frame_path(), Some(PathBuf::from("frame-0.png")));
        assert_eq!(pet.next_frame_delay(), None);
    }
}
