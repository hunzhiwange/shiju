//! One user-initiated insertion. Only the OS boundary is replaceable.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub pid: i32,
    pub element: usize,
    pub window: usize,
    pub selection: (i64, i64),
    pub value: String,
    pub editable: bool,
    pub secure: bool,
}

pub(crate) trait Backend {
    fn snapshot(&self) -> Result<Snapshot, String>;
    fn replace_selection(&self, text: &str) -> Result<(), String>;
}

pub(crate) struct Target<B> {
    backend: B,
    original: Snapshot,
}
impl<B: Backend> Target<B> {
    pub(crate) fn capture(backend: B) -> Result<Self, String> {
        let original = backend.snapshot()?;
        validate(&original)?;
        Ok(Self { backend, original })
    }
    pub(crate) fn verify(&self) -> Result<(), String> {
        let current = self.backend.snapshot()?;
        validate(&current)?;
        if current != self.original {
            return Err("目标、焦点、文本或选择区已变化；未插入。".into());
        }
        Ok(())
    }
    pub(crate) fn insert(self, text: &str) -> Result<(), String> {
        self.verify()?;
        if text.trim().is_empty() {
            return Err("没有可插入的听写结果。".into());
        }
        self.backend.replace_selection(text)?;
        let mut expected: Vec<u16> = self.original.value.encode_utf16().collect();
        let (start, length) = self.original.selection;
        expected.splice(start as usize..(start + length) as usize, text.encode_utf16());
        let current = self
            .backend
            .snapshot()
            .map_err(|_| "目标写入后的状态无法核验；请检查目标，勿重复插入。".to_owned())?;
        if current.pid != self.original.pid
            || current.element != self.original.element
            || current.window != self.original.window
            || current.value.encode_utf16().collect::<Vec<_>>() != expected
        {
            return Err("目标未确认完整写入；请检查目标后再决定复制，勿重复插入。".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "insertion_tests.rs"]
mod tests;

fn validate(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.pid <= 0 || snapshot.secure || !snapshot.editable {
        return Err("密码、安全字段或只读目标不可听写插入。".into());
    }
    let (start, len) = snapshot.selection;
    let units: Vec<u16> = snapshot.value.encode_utf16().collect();
    let end = start.checked_add(len).ok_or("目标选择区无效。")?;
    if start < 0
        || len < 0
        || end > units.len() as i64
        || String::from_utf16(&units[..start as usize]).is_err()
        || String::from_utf16(&units[end as usize..]).is_err()
    {
        return Err("目标选择区无效。".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) type NativeTarget = Target<super::insertion_macos::NativeBackend>;
#[cfg(not(target_os = "macos"))]
pub(crate) type NativeTarget = Target<Unsupported>;
#[cfg(not(target_os = "macos"))]
pub(crate) struct Unsupported;
#[cfg(not(target_os = "macos"))]
impl Backend for Unsupported {
    fn snapshot(&self) -> Result<Snapshot, String> {
        Err("当前平台不支持跨应用听写。".into())
    }
    fn replace_selection(&self, _: &str) -> Result<(), String> {
        Err("当前平台不支持跨应用听写。".into())
    }
}

pub(crate) fn capture_external() -> Result<Option<NativeTarget>, String> {
    #[cfg(target_os = "macos")]
    {
        super::insertion_macos::capture()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("当前平台不支持跨应用听写。".into())
    }
}

pub(crate) struct State {
    // Err retains the external destination: an unsafe target must never fall back into a chat draft.
    pub target: Option<Result<NativeTarget, String>>,
    pub pending: Option<String>,
    pub onboarding: bool,
    pub permissions: [super::permissions::PermissionStatus; 3],
    checked: Option<std::time::Instant>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            target: None,
            pending: None,
            onboarding: false,
            permissions: [super::permissions::PermissionStatus::Unknown; 3],
            checked: None,
        }
    }
}
impl State {
    pub(crate) fn refresh(&mut self) {
        use super::permissions::{Permission, PermissionBackend, SystemPermissions};
        self.permissions =
            [Permission::Microphone, Permission::InputMonitoring, Permission::Accessibility]
                .map(|permission| SystemPermissions.query(permission));
    }
    pub(crate) fn poll(&mut self) {
        if self.checked.is_some_and(|at| at.elapsed() < std::time::Duration::from_millis(250)) {
            return;
        }
        self.checked = Some(std::time::Instant::now());
        if self.onboarding {
            self.refresh();
        }
        if let Some(Ok(target)) = &self.target {
            if let Err(error) = target.verify() {
                self.target = Some(Err(error));
            }
        }
    }
}
