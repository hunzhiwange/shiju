//! Product error templates retain opaque causes until the current view renders them.
use desktop_i18n::{I18n, format::Message};
use std::{error::Error, fmt};

pub(super) trait IntoCopy {
    fn into_copy(self) -> Message;
}
impl IntoCopy for &'static str {
    fn into_copy(self) -> Message { Message::new(self, vec![]) }
}
impl IntoCopy for Message {
    fn into_copy(self) -> Message { self }
}
impl IntoCopy for String {
    fn into_copy(self) -> Message { Message::literal(self) }
}

#[derive(Debug)]
struct ProductError {
    copy: Message,
    cause: Option<anyhow::Error>,
}
impl fmt::Display for ProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = self.copy.render(I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
            .map_err(|_| fmt::Error)?;
        f.write_str(&source)
    }
}
impl Error for ProductError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|cause| {
            let source: &(dyn Error + Send + Sync + 'static) = cause.as_ref();
            source as &(dyn Error + 'static)
        })
    }
}
pub(super) fn error(copy: impl IntoCopy) -> anyhow::Error {
    anyhow::Error::new(ProductError { copy: copy.into_copy(), cause: None })
}

pub(super) trait ProductContext<T> {
    fn product_context(self, copy: impl IntoCopy) -> anyhow::Result<T>;
    fn product_with_context<C: IntoCopy>(self, copy: impl FnOnce() -> C) -> anyhow::Result<T>;
}
impl<T, E: Into<anyhow::Error>> ProductContext<T> for Result<T, E> {
    fn product_context(self, copy: impl IntoCopy) -> anyhow::Result<T> {
        self.product_with_context(|| copy)
    }
    fn product_with_context<C: IntoCopy>(self, copy: impl FnOnce() -> C) -> anyhow::Result<T> {
        self.map_err(|cause| anyhow::Error::new(ProductError {
            copy: copy().into_copy(), cause: Some(cause.into()),
        }))
    }
}
impl<T> ProductContext<T> for Option<T> {
    fn product_context(self, copy: impl IntoCopy) -> anyhow::Result<T> {
        self.product_with_context(|| copy)
    }
    fn product_with_context<C: IntoCopy>(self, copy: impl FnOnce() -> C) -> anyhow::Result<T> {
        self.ok_or_else(|| error(copy()))
    }
}

pub(super) fn message(error: &anyhow::Error) -> Message {
    fn from_cause(error: &(dyn Error + 'static)) -> Message {
        let copy = error.downcast_ref::<ProductError>()
            .map(|error| error.copy.clone())
            .unwrap_or_else(|| Message::literal(error.to_string()));
        match error.source() {
            Some(cause) => Message::nested("%s: %s".into(), vec![copy, from_cause(cause)]),
            None => copy,
        }
    }
    from_cause(error.as_ref())
}

#[cfg(test)]
#[path = "copy_tests.rs"]
mod tests;
