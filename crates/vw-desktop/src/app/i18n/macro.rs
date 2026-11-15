#[macro_export]
macro_rules! __ {
    ($i18n:expr, $source:expr $(, $argument:expr)* $(,)?) => {{
        ($i18n).format($source, &[$($crate::app::i18n::format::Argument::from($argument)),*])
    }};
}
