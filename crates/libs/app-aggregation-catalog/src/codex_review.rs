use std::collections::HashSet;

use serde_json::Value;

const REVIEW_OVERRIDE: &str = "auto_review_model_override";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexReviewError {
    InvalidCatalog,
}

/// Projects the effective reviewer into every row of a managed aggregate catalog.
///
/// Supply IDs from the non-default members of the same publication. The requested
/// preference remains with the caller, so republishing a removed member restores
/// the override. With no effective choice, Codex selects its own default reviewer.
/// The caller publishes the result through the existing config transaction.
pub fn apply_codex_auto_review_model<'a>(
    catalog: &mut Value,
    published_non_default: &[&str],
    requested: Option<&'a str>,
) -> Result<Option<&'a str>, CodexReviewError> {
    let rows = catalog
        .get_mut("models")
        .and_then(Value::as_array_mut)
        .ok_or(CodexReviewError::InvalidCatalog)?;

    // Validate the complete catalog before mutating any row.
    let mut slugs = HashSet::with_capacity(rows.len());
    for row in rows.iter() {
        let slug = row
            .get("slug")
            .and_then(Value::as_str)
            .filter(|slug| !slug.trim().is_empty())
            .ok_or(CodexReviewError::InvalidCatalog)?;
        if !slugs.insert(slug) {
            return Err(CodexReviewError::InvalidCatalog);
        }
    }
    let effective =
        requested.filter(|slug| published_non_default.contains(slug) && slugs.contains(slug));

    for row in rows {
        let row = row.as_object_mut().expect("validated catalog row");
        if let Some(slug) = effective {
            row.insert(REVIEW_OVERRIDE.to_owned(), Value::String(slug.to_owned()));
        } else {
            row.remove(REVIEW_OVERRIDE);
        }
    }
    Ok(effective)
}
