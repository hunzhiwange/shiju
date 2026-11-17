//! Window sharing is owned by the voice domain.
pub(crate) use desktop_voice::screen_context::State;

#[cfg(test)]
pub(crate) use desktop_voice::screen_context::Target;
#[cfg(test)]
pub(crate) use desktop_voice::screen_context::Ticket;
#[cfg(test)]
pub(crate) use desktop_voice::screen_context::capture;
#[cfg(test)]
pub(crate) use desktop_voice::screen_context::foreground;
#[cfg(test)]
pub(crate) use desktop_voice::screen_context::permitted;
