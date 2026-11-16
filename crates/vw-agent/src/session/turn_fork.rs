//! Turn fork composition using the installed lifecycle host.
pub(crate) use session_engine::turn_fork::{forked_session, load_fork_history};
pub(crate) type TurnFork = session_engine::turn_fork::TurnFork<super::super::session_io::NativeSessionIo>;
