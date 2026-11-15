use super::codex_review::{CodexReviewError, apply_codex_auto_review_model};
use serde_json::{Value, json};

const MEMBER: &str = "ccs-member/reviewer";
const FIELD: &str = "auto_review_model_override";

#[test]
fn aggregation_t23_codex_review_preserves_contract() {
    let mut catalog = json!({
        "models": [
            {"slug": "main", "auto_review_model_override": "card-only", "priority": 1},
            {"slug": MEMBER, "auto_review_model_override": "other-card-only"},
            {"slug": "codex-auto-review", "visibility": "hide"}
        ]
    });
    assert_eq!(
        apply_codex_auto_review_model(&mut catalog, &[MEMBER], Some(MEMBER)),
        Ok(Some(MEMBER))
    );
    assert_eq!(
        catalog,
        json!({
            "models": [
                {"slug": "main", "auto_review_model_override": MEMBER, "priority": 1},
                {"slug": MEMBER, "auto_review_model_override": MEMBER},
                {"slug": "codex-auto-review", "visibility": "hide", "auto_review_model_override": MEMBER}
            ]
        })
    );

    let member = catalog["models"].as_array_mut().unwrap().remove(1);
    assert_eq!(apply_codex_auto_review_model(&mut catalog, &[], Some(MEMBER)), Ok(None));
    assert!(catalog["models"].as_array().unwrap().iter().all(|row| row.get(FIELD).is_none()));

    catalog["models"].as_array_mut().unwrap().push(member);
    assert_eq!(
        apply_codex_auto_review_model(&mut catalog, &[MEMBER], Some(MEMBER)),
        Ok(Some(MEMBER))
    );
    assert_eq!(catalog["models"][0][FIELD], MEMBER);
}

#[test]
fn aggregation_t23_codex_review_ignores_card_and_unpublished_models() {
    for requested in [None, Some("ccs-default/main"), Some(MEMBER)] {
        let mut catalog = json!({
            "models": [
                {"slug": "ccs-default/main", "auto_review_model_override": "card-only"}
            ]
        });
        assert_eq!(apply_codex_auto_review_model(&mut catalog, &[MEMBER], requested), Ok(None));
        assert_eq!(catalog, json!({"models": [{"slug": "ccs-default/main"}]}));
    }
}

#[test]
fn aggregation_t23_codex_review_rejects_invalid_catalog_without_mutating_rows() {
    for invalid in [Value::Null, json!({"slug": ""}), json!({"slug": MEMBER})] {
        let mut catalog = json!({"models": [
            {"slug": MEMBER, "auto_review_model_override": "original"},
            invalid
        ]});
        let original = catalog.clone();
        assert_eq!(
            apply_codex_auto_review_model(&mut catalog, &[MEMBER], Some(MEMBER)),
            Err(CodexReviewError::InvalidCatalog)
        );
        assert_eq!(catalog, original);
    }
}
