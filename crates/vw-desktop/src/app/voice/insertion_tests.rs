use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct TextApplication(Rc<RefCell<Snapshot>>);
impl TextApplication {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(Snapshot {
            pid: 12,
            element: 1,
            window: 2,
            selection: (1, 2),
            value: "甲乙丙丁".into(),
            editable: true,
            secure: false,
        })))
    }
}
impl Backend for TextApplication {
    fn snapshot(&self) -> Result<Snapshot, String> {
        Ok(self.0.borrow().clone())
    }
    fn replace_selection(&self, text: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        let mut units: Vec<u16> = state.value.encode_utf16().collect();
        let (start, len) = state.selection;
        units.splice(start as usize..(start + len) as usize, text.encode_utf16());
        state.value = String::from_utf16(&units).unwrap();
        Ok(())
    }
}
#[test]
fn voice_task_16_insertion_focus_change_preserves_target() {
    let app = TextApplication::new();
    let target = Target::capture(app.clone()).unwrap();
    app.0.borrow_mut().element = 7;
    assert!(target.insert("误写").is_err());
    assert_eq!(app.0.borrow().value, "甲乙丙丁");
}
#[test]
fn voice_task_16_insertion_secure_field_is_rejected() {
    let app = TextApplication::new();
    app.0.borrow_mut().secure = true;
    assert!(Target::capture(app.clone()).is_err());
    assert_eq!(app.0.borrow().value, "甲乙丙丁");
}

#[test]
fn voice_task_16_insertion_replaces_utf16_selection_with_chinese_multiline() {
    let app = TextApplication::new();
    app.0.borrow_mut().value = "甲😀乙丁".into();
    let target = Target::capture(app.clone()).unwrap();
    target.insert("你好，\n世界！").unwrap();
    assert_eq!(app.0.borrow().value, "甲你好，\n世界！乙丁");
}
#[test]
fn voice_task_16_insertion_rejects_changed_pid_window_selection_value_and_readonly() {
    for change in 0..6 {
        let app = TextApplication::new();
        let target = Target::capture(app.clone()).unwrap();
        {
            let mut snapshot = app.0.borrow_mut();
            match change {
                0 => snapshot.pid += 1,
                1 => snapshot.window += 1,
                2 => snapshot.selection = (0, 0),
                3 => snapshot.value = "编辑后的目标".into(),
                4 => snapshot.editable = false,
                _ => snapshot.secure = true,
            }
        }
        let before = app.0.borrow().value.clone();
        assert!(target.insert("不得写入").is_err());
        assert_eq!(app.0.borrow().value, before);
    }
}
#[test]
fn voice_task_16_insertion_rejects_invalid_or_split_surrogate_selection() {
    for selection in [(-1, 1), (1, -1), (99, 1), (1, i64::MAX), (2, 0)] {
        let app = TextApplication::new();
        app.0.borrow_mut().value = "甲😀乙".into();
        app.0.borrow_mut().selection = selection;
        assert!(Target::capture(app).is_err());
    }
}
#[test]
fn voice_task_16_insertion_drop_and_empty_text_make_no_write() {
    let app = TextApplication::new();
    drop(Target::capture(app.clone()).unwrap());
    assert!(Target::capture(app.clone()).unwrap().insert(" \n").is_err());
    assert_eq!(app.0.borrow().value, "甲乙丙丁");
}

#[test]
fn voice_task_16_insertion_requires_readback_and_never_retries() {
    use std::cell::Cell;
    struct Transforming {
        app: TextApplication,
        writes: Rc<Cell<usize>>,
    }
    impl Backend for Transforming {
        fn snapshot(&self) -> Result<Snapshot, String> {
            self.app.snapshot()
        }
        fn replace_selection(&self, text: &str) -> Result<(), String> {
            self.writes.set(self.writes.get() + 1);
            self.app.replace_selection(&text.replace('\n', ""))
        }
    }
    let writes = Rc::new(Cell::new(0));
    let app = TextApplication::new();
    let target =
        Target::capture(Transforming { app: app.clone(), writes: writes.clone() }).unwrap();
    assert!(target.insert("你\n好").is_err());
    assert_eq!(writes.get(), 1);
    assert_eq!(app.0.borrow().value, "甲你好丁");
}
