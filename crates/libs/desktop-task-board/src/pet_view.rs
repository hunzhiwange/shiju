use crate::pet_state::TaskPetAvatarKind;
use iced::Size;
use iced::animation::{Animation, Easing};
use iced::time::{Duration, Instant};

pub(crate) const WINDOW_PADDING: f32 = 8.0;
pub(crate) const DOCK_HEIGHT: f32 = 44.0;
pub(crate) const ACTIONS_WIDTH: f32 = 144.0;
pub(crate) const REMINDER_WIDTH: f32 = 344.0;
pub const FEEDBACK_HEIGHT: f32 = 72.0;
const DOCK_GAP: f32 = 4.0;
const PROJECT_MENU_SPACE: f32 = 224.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DockMode {
    #[default]
    Idle,
    Actions,
    Composer,
    VoiceConnecting,
    Voice,
}

impl DockMode {
    pub(crate) fn size(self) -> Size {
        match self {
            Self::Idle => Size::new(26.0, 8.0),
            Self::Actions | Self::VoiceConnecting => Size::new(ACTIONS_WIDTH, DOCK_HEIGHT),
            Self::Composer => Size::new(344.0, DOCK_HEIGHT),
            Self::Voice => Size::new(244.0, DOCK_HEIGHT),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PetVoice {
    pub muted: bool,
    pub microphone_enabled: bool,
    pub orb_action: Option<VoiceOrbAction>,
    pub can_interrupt: bool,
    pub connecting: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DockView<'a> {
    pub mode: DockMode,
    pub width: f32,
    pub height: f32,
    pub reminder_height: f32,
    pub reminders_open: bool,
    pub draft: &'a str,
    pub project_path: Option<&'a str>,
    pub project_query: Option<&'a str>,
    pub voice: Option<PetVoice>,
}

#[derive(Debug)]
pub struct PetControls {
    pub hovered: bool,
    pub composer: bool,
    pub draft: String,
    pub project_path: Option<String>,
    pub project_query: Option<String>,
    pub reminders_open: bool,
    pub voice_owned: bool,
    pub feedback_id: Option<usize>,
    pub native_size: Option<Size>,
    mode: DockMode,
    width: Animation<f32>,
    height: Animation<f32>,
    reminders: Animation<f32>,
}

impl Default for PetControls {
    fn default() -> Self {
        let animate = |value| {
            Animation::new(value).duration(Duration::from_millis(220)).easing(Easing::EaseOutCubic)
        };
        let idle = DockMode::Idle.size();
        Self {
            hovered: false,
            composer: false,
            draft: String::new(),
            project_path: None,
            project_query: None,
            reminders_open: false,
            voice_owned: false,
            feedback_id: None,
            native_size: None,
            mode: DockMode::Idle,
            width: animate(idle.width),
            height: animate(idle.height),
            reminders: animate(0.0),
        }
    }
}

impl PetControls {
    pub fn close_composer(&mut self) {
        self.composer = false;
        self.project_query = None;
    }

    pub fn update(&mut self, voice: Option<PetVoice>, reminder_height: f32, now: Instant) {
        self.mode = if self.composer {
            DockMode::Composer
        } else if let Some(voice) = voice {
            if voice.connecting { DockMode::VoiceConnecting } else { DockMode::Voice }
        } else if self.hovered || self.reminders_open {
            DockMode::Actions
        } else {
            DockMode::Idle
        };
        let size = self.mode.size();
        for (animation, target) in [
            (&mut self.width, size.width),
            (&mut self.height, size.height),
            (&mut self.reminders, reminder_height),
        ] {
            if animation.value() != target {
                animation.go_mut(target, now);
            }
        }
    }

    pub fn animating(&self, now: Instant) -> bool {
        self.width.is_animating(now)
            || self.height.is_animating(now)
            || self.reminders.is_animating(now)
    }

    pub fn view(&self, voice: Option<PetVoice>, now: Instant) -> DockView<'_> {
        DockView {
            mode: self.mode,
            width: self.width.interpolate_with(|width| width, now),
            height: self.height.interpolate_with(|height| height, now),
            reminder_height: self.reminders.interpolate_with(|height| height, now),
            reminders_open: self.reminders_open,
            draft: &self.draft,
            project_path: self.project_path.as_deref(),
            project_query: self.project_query.as_deref(),
            voice,
        }
    }

    pub fn window_size(
        &self,
        avatar: f32,
        reminder_height: f32,
        feedback: bool,
        now: Instant,
    ) -> Size {
        // Keep room for the three hover controls even at rest. During a transition
        // retain the previous native frame until the painted contents fit.
        let width = self.mode.size().width.max(ACTIONS_WIDTH).max(avatar);
        let width =
            if reminder_height > 0.0 || feedback { width.max(REMINDER_WIDTH) } else { width };
        let height = dock_window_height(avatar)
            + reminder_height
            + if self.composer && self.project_query.is_some() { PROJECT_MENU_SPACE } else { 0.0 }
            + if feedback { FEEDBACK_HEIGHT } else { 0.0 };
        let target = Size::new(width + WINDOW_PADDING * 2.0, height);
        match self.native_size {
            Some(previous) if self.animating(now) => {
                Size::new(target.width.max(previous.width), target.height.max(previous.height))
            }
            _ => target,
        }
    }
}

pub(crate) fn dock_window_height(avatar: f32) -> f32 {
    WINDOW_PADDING * 2.0 + avatar + DOCK_HEIGHT + if avatar > 0.0 { DOCK_GAP } else { 0.0 }
}

#[derive(Debug, Clone, Copy)]
pub enum VoiceOrbAction {
    Begin,
    Finish,
    Interrupt,
    End,
}
/// 宠物窗口的借用展示输入；窗口生命周期与运动时钟由宿主维护。
#[derive(Clone, Copy)]
pub struct PetView<'a> {
    pub dock: DockView<'a>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub task_pet_items: &'a [super::pet_state::TaskPetItem],
    pub project_paths: &'a [String],
    pub project_labels: &'a [String],
    pub can_restore_reminder: bool,
    pub task_pet_avatar_kind: TaskPetAvatarKind,
    pub task_pet_context_menu_pos: Option<(f32, f32)>,
    pub task_pet_robot_hovered: bool,
    pub status_animation_frame: usize,
    pub task_pet_reply_request_id: Option<u64>,
    pub task_pet_hovered_request_id: Option<u64>,
    pub task_pet_reply_input: &'a str,
    pub task_pet_scroll_id: &'a iced::widget::Id,
    pub render_collapsed: bool,
    pub preview_scale: f32,
    pub avatar_bounds: f32,
    pub active_count: usize,
    pub is_walking: bool,
    pub facing_left: bool,
}
/// 窗口交互意图，由桌面映射为全局视图消息。
#[derive(Debug, Clone)]
pub enum PetMessage {
    TaskPetWriteMessage,
    TaskPetDockHover(bool),
    TaskPetComposerChanged(String),
    TaskPetComposerSubmit,
    TaskPetComposerClose,
    TaskPetProjectMenuToggle,
    TaskPetProjectMenuClose,
    TaskPetProjectQueryChanged(String),
    TaskPetProjectSelected(String),
    TaskPetStartVoice,
    TaskPetVoiceMic,
    TaskPetVoiceOrb(VoiceOrbAction),
    TaskPetToggleReminders,
    TaskPetExpandReminders,
    TaskPetOpenReminder(u64),
    TaskPetDragStarted,
    TaskPetRobotHover(bool),
    TaskPetContextMenuOpened(f32, f32),
    TaskPetContextMenuClosed,
    TaskPetOpenSettings,
    TaskPetSizeOpen,
    TaskPetHideWindow,
    TaskPetRemove(u64),
    TaskPetRestoreReminder,
    TaskPetReplyPressed(u64),
    TaskPetReplyInputChanged(String),
    TaskPetReplySubmit,
    TaskPetReplyClose,
    TaskPetHover(Option<u64>),
}
