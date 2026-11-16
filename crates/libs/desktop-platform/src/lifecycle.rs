/// Machine-local preferences; native handles remain owned by the UI thread.
#[derive(Debug)]
pub struct State {
    pub launch_at_login: bool,
    pub launch_at_login_busy: bool,
    pub close_to_tray: bool,
}
