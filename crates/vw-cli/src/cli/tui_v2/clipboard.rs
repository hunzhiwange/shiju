use std::io::{self, Write};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

pub(crate) fn osc52_sequence(text: &str) -> String {
    format!("\u{1b}]52;c;{}\u{7}", STANDARD.encode(text.as_bytes()))
}

pub(crate) fn write_osc52(writer: &mut impl Write, text: &str) -> io::Result<()> {
    writer.write_all(osc52_sequence(text).as_bytes())?;
    writer.flush()
}
