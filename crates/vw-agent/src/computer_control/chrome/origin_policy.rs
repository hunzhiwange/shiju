//! Exact HTTP origins and connection-local document leases. No cached grant survives
//! a document replacement, even if Chrome later returns to the same URL.
use super::bridge::ChromeTarget;
use vw_api_types::computer_control::ControlErrorCode;

pub fn canonical_origin(value: &str) -> Result<String, ControlErrorCode> {
    let url = url::Url::parse(value).map_err(|_| ControlErrorCode::InvalidRequest)?;
    if value.len() > 8192
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ControlErrorCode::InvalidRequest);
    }
    Ok(url.origin().ascii_serialization())
}

#[derive(Default)]
pub struct DocumentBinding {
    target: Option<ChromeTarget>,
    invalidated: bool,
}
impl DocumentBinding {
    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }

    pub fn check(
        &mut self,
        target: &ChromeTarget,
        expected_origin: &str,
    ) -> Result<(), ControlErrorCode> {
        if self.invalidated
            || target.document_id.is_empty()
            || canonical_origin(&target.url).as_deref() != Ok(expected_origin)
            || self.target.as_ref().is_some_and(|old| {
                old.tab_id != target.tab_id
                    || old.document_id != target.document_id
                    || old.navigation_version != target.navigation_version
                    || old.url != target.url
            })
        {
            self.invalidate();
            return Err(ControlErrorCode::TargetChanged);
        }
        self.target = Some(target.clone());
        Ok(())
    }
}
