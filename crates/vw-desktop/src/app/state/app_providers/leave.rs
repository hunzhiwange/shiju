/// Navigation is retained only in memory; it never contains editor credentials.
#[derive(Clone, Debug, Default)]
pub(crate) struct LeaveState {
    pub dirty: bool,
    pub pending: Option<Box<crate::app::Message>>,
    pub saving: bool,
}
