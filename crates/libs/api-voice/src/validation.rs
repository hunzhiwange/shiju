use serde::Deserialize;

fn bounded_text<'de, D: serde::Deserializer<'de>>(d: D, limit: usize) -> Result<String, D::Error> {
    let text = String::deserialize(d)?;
    if text.len() > limit {
        return Err(serde::de::Error::custom("voice text exceeds size limit"));
    }
    Ok(text)
}
pub(crate) fn text_64k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    bounded_text(d, 64 * 1024)
}
pub(crate) fn text_16k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    bounded_text(d, 16 * 1024)
}
