//! Lifecycle transitions for a single application routing listener.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionInProgress;

/// A running resource belongs to exactly one lifecycle. Callers serialize
/// transitions and finish them after acquiring or releasing the resource.
#[derive(Default)]
pub enum Lifecycle<T> {
    #[default]
    Stopped,
    Starting,
    Running(T),
    Stopping,
    Failed,
}

impl<T> Lifecycle<T> {
    pub fn state(&self) -> LifecycleState {
        match self {
            Self::Stopped => LifecycleState::Stopped,
            Self::Starting => LifecycleState::Starting,
            Self::Running(_) => LifecycleState::Running,
            Self::Stopping => LifecycleState::Stopping,
            Self::Failed => LifecycleState::Failed,
        }
    }

    /// Returns the current resource if already running, or reserves a start.
    pub fn begin_start(&mut self) -> Result<Option<&T>, TransitionInProgress> {
        match self {
            Self::Running(resource) => Ok(Some(resource)),
            Self::Starting | Self::Stopping => Err(TransitionInProgress),
            Self::Stopped | Self::Failed => {
                *self = Self::Starting;
                Ok(None)
            }
        }
    }

    pub fn finish_start(&mut self, resource: Option<T>) {
        assert!(matches!(self, Self::Starting));
        *self = resource.map_or(Self::Failed, Self::Running);
    }

    /// Transfers the running resource to the caller responsible for shutdown.
    pub fn begin_stop(&mut self) -> Result<Option<T>, TransitionInProgress> {
        match self {
            Self::Starting | Self::Stopping => Err(TransitionInProgress),
            Self::Running(_) => {
                let Self::Running(resource) = std::mem::replace(self, Self::Stopping) else {
                    unreachable!();
                };
                Ok(Some(resource))
            }
            Self::Stopped | Self::Failed => {
                *self = Self::Stopped;
                Ok(None)
            }
        }
    }

    /// Publish the exit only after the listener has released its socket.
    pub fn listener_exited(&mut self) {
        *self = match self {
            Self::Running(_) => Self::Failed,
            Self::Stopping => Self::Stopped,
            _ => unreachable!("no listener can exit in this state"),
        };
    }
}
